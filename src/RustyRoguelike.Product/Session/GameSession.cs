using Rusty.Engine;
using RustyRoguelike.Product.Combat;
using RustyRoguelike.Product.Exploration;
using RustyRoguelike.Product.Floors;
using RustyRoguelike.Product.Party;
using RustyRoguelike.Product.Rules;

namespace RustyRoguelike.Product.Session;

internal enum SessionPhase { Preparation, PartyDecision, Terminal }
internal enum SessionOutcome { Ongoing, Victory, Defeat }
internal abstract record SessionCommand(ulong ExpectedRevision);
internal sealed record BeginExpeditionCommand(ulong ExpectedRevision) : SessionCommand(ExpectedRevision);
internal sealed record MovePartyCommand(ulong ExpectedRevision, int DeltaX, int DeltaY) : SessionCommand(ExpectedRevision);
internal sealed record UseActionCommand(ulong ExpectedRevision, string ActorId, string ActionId, string TargetId) : SessionCommand(ExpectedRevision);
internal sealed record WaitCommand(ulong ExpectedRevision) : SessionCommand(ExpectedRevision);
internal sealed record SessionCommandReceipt(bool Accepted, string Code, ulong Revision, IReadOnlyList<CombatReceipt> CombatReceipts);

/// <summary>One product-owned continuous initiative session. Commands stage all legality before they advance revision or mutate state.</summary>
internal sealed class GameSession
{
    private readonly RoguelikeRules _rules;
    private readonly CombatResolver _combat;
    private readonly Dictionary<string, int> _targetCursors = new(StringComparer.Ordinal);
    private readonly List<CombatReceipt> _receipts = [];
    private readonly Func<GridCell, GridCell, bool> _movementAdmission;
    private IReadOnlyList<string> _lastAdmittedInitiative = [];
    internal GameSession(IRandomService random, FloorState floor, Func<GridCell, GridCell, bool>? movementAdmission = null)
    {
        _rules = RoguelikeRules.Starter;
        Party = new PartyState(_rules);
        World = new ExplorationState(_rules, floor);
        _combat = new CombatResolver(random);
        _movementAdmission = movementAdmission ?? ((_, destination) => World.IsWalkable(destination));
        Phase = SessionPhase.Preparation;
        Outcome = SessionOutcome.Ongoing;
    }

    internal PartyState Party { get; }
    internal ExplorationState World { get; }
    internal SessionPhase Phase { get; private set; }
    internal SessionOutcome Outcome { get; private set; }
    internal ulong Revision { get; private set; }
    internal ulong ActivationIndex { get; private set; }
    internal int LastAdmittedOppositionActivations { get; private set; }
    internal IReadOnlyList<string> LastAdmittedInitiative => _lastAdmittedInitiative;
    internal IReadOnlyList<CombatReceipt> Receipts => _receipts;
    internal GameplayTuning Tuning => _rules.Tuning;

    internal SessionCommandReceipt Submit(SessionCommand command)
    {
        ArgumentNullException.ThrowIfNull(command);
        if (command.ExpectedRevision != Revision) return Reject("stale-revision");
        if (Phase == SessionPhase.Terminal) return Reject("terminal");
        try
        {
            return command switch
            {
                BeginExpeditionCommand => Begin(),
                MovePartyCommand move => Move(move),
                UseActionCommand action => UseAction(action),
                WaitCommand => Wait(),
                _ => Reject("unsupported-command"),
            };
        }
        catch (InvalidOperationException exception)
        {
            return Reject(exception.Message);
        }
    }

    private SessionCommandReceipt Begin()
    {
        if (Phase != SessionPhase.Preparation) return Reject("already-begun");
        if (Party.Members.Count != _rules.Tuning.PartySize || Party.Living.Count != _rules.Tuning.PartySize) return Reject("party-not-ready");
        if (World.Opposition.Count > _rules.Tuning.AutomaticSettlementLimit) return Reject("opposition-roster-exceeds-settlement-limit");
        Phase = SessionPhase.PartyDecision;
        World.AdmitVisibleOpposition();
        return Accept([]);
    }

    private SessionCommandReceipt Move(MovePartyCommand command)
    {
        if (Phase != SessionPhase.PartyDecision) return Reject("not-party-decision");
        int distance = Math.Abs(command.DeltaX) + Math.Abs(command.DeltaY);
        if (distance != _rules.Tuning.ActivationCost) return Reject("movement-must-be-one-step");
        GridCell destination = World.PartyCell.Step(command.DeltaX, command.DeltaY);
        if (!_movementAdmission(World.PartyCell, destination)) return Reject("engine-navigation-rejected-step");
        World.MoveParty(destination);
        World.AdmitVisibleOpposition();
        return SettleAndAccept([]);
    }

    private SessionCommandReceipt UseAction(UseActionCommand command)
    {
        if (Phase != SessionPhase.PartyDecision) return Reject("not-party-decision");
        PartyMemberState? actor = Party.Find(command.ActorId);
        OppositionState? target = World.Find(command.TargetId);
        if (actor is null || target is null || !actor.IsLiving || !target.IsLiving || !target.Participating) return Reject("invalid-actor-or-target");
        if (!actor.Definition.Actions.Contains(command.ActionId) || !_rules.Actions.TryGetValue(command.ActionId, out ActionDefinition? action)) return Reject("action-unavailable");
        if (World.PartyCell.ManhattanDistance(target.Position) > action.Range) return Reject("target-out-of-range");
        CombatReceipt receipt = _combat.ResolvePartyAttack(actor, target, action, _rules.Tuning, Revision);
        return SettleAndAccept([receipt]);
    }

    private SessionCommandReceipt Wait()
    {
        if (Phase != SessionPhase.PartyDecision) return Reject("not-party-decision");
        return SettleAndAccept([]);
    }

    private SessionCommandReceipt SettleAndAccept(IReadOnlyList<CombatReceipt> initial)
    {
        var settled = new List<CombatReceipt>(initial);
        ActivationIndex = checked(ActivationIndex + (ulong)_rules.Tuning.ActivationCost);
        World.AdmitVisibleOpposition();
        OppositionState[] admitted = World.Opposition
            .Where(enemy => enemy.Participating && enemy.IsLiving)
            .OrderByDescending(enemy => enemy.Definition.Finesse)
            .ThenBy(enemy => enemy.Definition.EntityId)
            .ToArray();
        LastAdmittedOppositionActivations = admitted.Length;
        _lastAdmittedInitiative = admitted.Select(enemy => enemy.Definition.Id).ToArray();
        foreach (OppositionState enemy in admitted)
        {
            PartyMemberState[] living = Party.Living.ToArray();
            if (living.Length == 0) break;
            ActionDefinition action = _rules.Actions[enemy.Definition.Actions[0]];
            if (enemy.Position.ManhattanDistance(World.PartyCell) > action.Range) continue;
            int cursor = _targetCursors.GetValueOrDefault(enemy.Definition.Id);
            PartyMemberState target = living[cursor % living.Length];
            CombatReceipt receipt = _combat.ResolveOppositionAttack(enemy, target, living.Length, action, _rules.Tuning, Revision);
            _targetCursors[enemy.Definition.Id] = checked(cursor + 1);
            settled.Add(receipt);
        }
        _receipts.AddRange(settled);
        if (!World.Opposition.Any(enemy => enemy.IsLiving)) { Outcome = SessionOutcome.Victory; Phase = SessionPhase.Terminal; }
        else if (Party.Living.Count == 0) { Outcome = SessionOutcome.Defeat; Phase = SessionPhase.Terminal; }
        return Accept(settled);
    }

    private SessionCommandReceipt Accept(IReadOnlyList<CombatReceipt> receipts)
    {
        Revision = checked(Revision + 1);
        return new SessionCommandReceipt(true, "accepted", Revision, receipts);
    }

    private SessionCommandReceipt Reject(string code) => new(false, code, Revision, []);

    internal SessionCheckpoint Capture()
    {
        return new SessionCheckpoint(
            Phase,
            Outcome,
            Revision,
            ActivationIndex,
            LastAdmittedOppositionActivations,
            LastAdmittedInitiative.ToArray(),
            Party.CaptureVitality(),
            World.PartyCell.X,
            World.PartyCell.Y,
            World.CaptureOpposition(),
            new Dictionary<string, int>(_targetCursors, StringComparer.Ordinal),
            Receipts.ToArray());
    }

    internal static GameSession Restore(
        IRandomService random,
        FloorState floor,
        SessionCheckpoint checkpoint,
        Func<GridCell, GridCell, bool>? movementAdmission = null)
    {
        ArgumentNullException.ThrowIfNull(checkpoint);
        if (!Enum.IsDefined(checkpoint.Phase) || !Enum.IsDefined(checkpoint.Outcome)
            || checkpoint.LastAdmittedOppositionActivations < 0
            || checkpoint.LastAdmittedOppositionInitiative.Distinct(StringComparer.Ordinal).Count() != checkpoint.LastAdmittedOppositionInitiative.Count)
        {
            throw new InvalidOperationException("session-save-shape-invalid");
        }

        GameSession restored = new(random, floor, movementAdmission);
        restored.Party.RestoreVitality(checkpoint.Party);
        restored.World.Restore(new GridCell(checkpoint.PartyCellX, checkpoint.PartyCellY), checkpoint.Opposition);
        if (checkpoint.TargetCursors.Any(cursor => restored.World.Find(cursor.Key) is null || cursor.Value < 0))
        {
            throw new InvalidOperationException("session-save-target-cursor-invalid");
        }
        if (checkpoint.Receipts.Any(receipt => restored.Party.Find(receipt.Attacker) is null && restored.World.Find(receipt.Attacker) is null
            || restored.Party.Find(receipt.Target) is null && restored.World.Find(receipt.Target) is null
            || receipt.EligibleMembers < 0 || receipt.RequestedDamage < 0 || receipt.AppliedDamage < 0 || receipt.AppliedDamage > receipt.RequestedDamage))
        {
            throw new InvalidOperationException("session-save-receipt-invalid");
        }

        restored.Phase = checkpoint.Phase;
        restored.Outcome = checkpoint.Outcome;
        restored.Revision = checkpoint.Revision;
        restored.ActivationIndex = checkpoint.ActivationIndex;
        restored.LastAdmittedOppositionActivations = checkpoint.LastAdmittedOppositionActivations;
        restored._lastAdmittedInitiative = checkpoint.LastAdmittedOppositionInitiative.ToArray();
        foreach ((string key, int value) in checkpoint.TargetCursors)
        {
            restored._targetCursors.Add(key, value);
        }
        restored._receipts.AddRange(checkpoint.Receipts);
        return restored;
    }
}

/// <summary>Closed product state used by the persistence codec; it never becomes an Engine gameplay schema.</summary>
internal sealed record SessionCheckpoint(
    SessionPhase Phase,
    SessionOutcome Outcome,
    ulong Revision,
    ulong ActivationIndex,
    int LastAdmittedOppositionActivations,
    IReadOnlyList<string> LastAdmittedOppositionInitiative,
    IReadOnlyList<ActorVitalitySnapshot> Party,
    int PartyCellX,
    int PartyCellY,
    IReadOnlyList<OppositionSnapshot> Opposition,
    IReadOnlyDictionary<string, int> TargetCursors,
    IReadOnlyList<CombatReceipt> Receipts);
