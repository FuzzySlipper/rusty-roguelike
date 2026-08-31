using Rusty.Engine;
using RustyRoguelike.Product.Combat;
using RustyRoguelike.Product.Exploration;
using RustyRoguelike.Product.Floors;
using RustyRoguelike.Product.Party;
using RustyRoguelike.Product.Rules;

namespace RustyRoguelike.Product.Session;

internal enum SessionPhase { Preparation, PartyDecision, Terminal }
internal enum SessionOutcome { Ongoing, Victory, Defeat }
internal enum InitiativeSide { Party, Opposition }
internal enum SessionDecisionClass { Preparation, PartyActivation, AutomaticOpposition, Terminal }
internal abstract record SessionCommand(ulong ExpectedRevision);
internal sealed record BeginExpeditionCommand(ulong ExpectedRevision) : SessionCommand(ExpectedRevision);
internal sealed record MovePartyCommand(ulong ExpectedRevision, int DeltaX, int DeltaY) : SessionCommand(ExpectedRevision);
internal sealed record UseActionCommand(ulong ExpectedRevision, string ActorId, string ActionId, string TargetId) : SessionCommand(ExpectedRevision);
internal sealed record WaitCommand(ulong ExpectedRevision) : SessionCommand(ExpectedRevision);
internal sealed record SessionCommandReceipt(bool Accepted, string Code, ulong Revision, IReadOnlyList<CombatReceipt> CombatReceipts);
internal sealed record InitiativeActorSnapshot(string Id, InitiativeSide Side, int Finesse, ulong EntityId);

/// <summary>Continuous product initiative. Every command settles in a detached candidate before live state is replaced.</summary>
internal sealed class GameSession
{
    private readonly RoguelikeRules _rules;
    private readonly CombatResolver _combat;
    private readonly Func<GridCell, GridCell, bool> _movementAdmission;
    private readonly OppositionVisibilityQuery _visibilityQuery;
    private Dictionary<string, int> _targetCursors = new(StringComparer.Ordinal);
    private List<CombatReceipt> _receipts = [];
    private List<InitiativeActorSnapshot> _initiative = [];

    internal GameSession(
        IRandomService random,
        FloorState floor,
        OppositionVisibilityQuery visibilityQuery,
        Func<GridCell, GridCell, bool>? movementAdmission = null)
    {
        _rules = RoguelikeRules.Starter;
        Party = new PartyState(_rules);
        _visibilityQuery = visibilityQuery ?? throw new ArgumentNullException(nameof(visibilityQuery));
        World = new ExplorationState(_rules, floor, _visibilityQuery);
        _combat = new CombatResolver(random);
        _movementAdmission = movementAdmission ?? ((_, _) => false);
        Phase = SessionPhase.Preparation;
        DecisionClass = SessionDecisionClass.Preparation;
    }

    internal PartyState Party { get; private set; }
    internal ExplorationState World { get; private set; }
    internal SessionPhase Phase { get; private set; }
    internal SessionOutcome Outcome { get; private set; }
    internal SessionDecisionClass DecisionClass { get; private set; }
    internal ulong Revision { get; private set; }
    internal ulong ActivationIndex { get; private set; }
    internal ulong Round { get; private set; }
    internal int InitiativeCursor { get; private set; }
    internal IReadOnlyList<InitiativeActorSnapshot> Initiative => _initiative;
    internal InitiativeActorSnapshot? CurrentActor => InitiativeCursor >= 0 && InitiativeCursor < _initiative.Count ? _initiative[InitiativeCursor] : null;
    internal IReadOnlyList<CombatReceipt> Receipts => _receipts;
    internal GameplayTuning Tuning => _rules.Tuning;

    internal SessionCommandReceipt Submit(SessionCommand command)
    {
        ArgumentNullException.ThrowIfNull(command);
        if (command.ExpectedRevision != Revision) return Reject("stale-revision");
        if (Phase == SessionPhase.Terminal) return Reject("terminal");
        try
        {
            GameSession candidate = Restore(_combat.Random, World.Floor, Capture(), _visibilityQuery, _movementAdmission);
            SessionCommandReceipt receipt = candidate.Execute(command);
            if (!receipt.Accepted) return Reject(receipt.Code);
            Adopt(candidate);
            return receipt;
        }
        catch (EngineCallException)
        {
            return Reject("command-settlement-failed");
        }
        catch (InvalidOperationException)
        {
            return Reject("command-settlement-failed");
        }
        catch (OverflowException)
        {
            return Reject("command-settlement-failed");
        }
    }

    private SessionCommandReceipt Execute(SessionCommand command) => command switch
    {
        BeginExpeditionCommand => Begin(),
        MovePartyCommand move => Move(move),
        UseActionCommand action => UseAction(action),
        WaitCommand => Wait(),
        _ => Reject("unsupported-command"),
    };

    private SessionCommandReceipt Begin()
    {
        if (Phase != SessionPhase.Preparation) return Reject("already-begun");
        if (Party.Members.Count != _rules.Tuning.PartySize || Party.Living.Count != _rules.Tuning.PartySize) return Reject("party-not-ready");
        World.AdmitVisibleOpposition();
        BuildRound();
        return Accept(SettleAutomaticActors());
    }

    private SessionCommandReceipt Move(MovePartyCommand command)
    {
        if (!HasPartyDecision()) return Reject("not-party-decision");
        int distance = Math.Abs(command.DeltaX) + Math.Abs(command.DeltaY);
        if (distance != _rules.Tuning.ActivationCost) return Reject("movement-must-be-one-step");
        GridCell destination = World.PartyCell.Step(command.DeltaX, command.DeltaY);
        if (!_movementAdmission(World.PartyCell, destination)) return Reject("engine-navigation-rejected-step");
        World.MoveParty(destination);
        World.AdmitVisibleOpposition(); // newly admitted opposition joins only on the next round rebuild.
        return CompletePartyActivation([]);
    }

    private SessionCommandReceipt UseAction(UseActionCommand command)
    {
        if (!HasPartyDecision()) return Reject("not-party-decision");
        InitiativeActorSnapshot current = CurrentActor!;
        if (!StringComparer.Ordinal.Equals(current.Id, command.ActorId)) return Reject("out-of-turn-actor");
        PartyMemberState? actor = Party.Find(command.ActorId);
        OppositionState? target = World.Find(command.TargetId);
        if (actor is null || target is null || !actor.IsLiving || !target.IsLiving || !target.Participating) return Reject("invalid-actor-or-target");
        if (!actor.Definition.Actions.Contains(command.ActionId) || !_rules.Actions.TryGetValue(command.ActionId, out ActionDefinition? action)) return Reject("action-unavailable");
        if (World.PartyCell.ManhattanDistance(target.Position) > action.Range) return Reject("target-out-of-range");
        return CompletePartyActivation([_combat.ResolvePartyAttack(actor, target, action, _rules.Tuning, Revision)]);
    }

    private SessionCommandReceipt Wait() => !HasPartyDecision() ? Reject("not-party-decision") : CompletePartyActivation([]);

    private SessionCommandReceipt CompletePartyActivation(IReadOnlyList<CombatReceipt> initial)
    {
        var settled = new List<CombatReceipt>(initial);
        ActivationIndex = checked(ActivationIndex + (ulong)_rules.Tuning.ActivationCost);
        AdvanceCursor();
        settled.AddRange(SettleAutomaticActors());
        return Accept(settled);
    }

    private List<CombatReceipt> SettleAutomaticActors()
    {
        var settled = new List<CombatReceipt>();
        int automaticSettlements = 0;
        while (Outcome == SessionOutcome.Ongoing)
        {
            SkipUnavailableActors();
            if (Outcome != SessionOutcome.Ongoing) return settled;
            InitiativeActorSnapshot? current = CurrentActor;
            if (current is null) { BuildRound(); continue; }
            if (current.Side == InitiativeSide.Party)
            {
                Phase = SessionPhase.PartyDecision;
                DecisionClass = SessionDecisionClass.PartyActivation;
                return settled;
            }
            DecisionClass = SessionDecisionClass.AutomaticOpposition;
            if (checked(++automaticSettlements) > _rules.Tuning.AutomaticSettlementLimit)
                throw new InvalidOperationException("automatic-settlement-limit-exceeded");
            OppositionState enemy = World.Find(current.Id) ?? throw new InvalidOperationException("initiative-opposition-missing");
            PartyMemberState[] living = Party.Living.ToArray();
            if (living.Length == 0) { SetOutcome(SessionOutcome.Defeat); return settled; }
            ActionDefinition action = _rules.Actions[enemy.Definition.Actions[0]];
            if (enemy.Position.ManhattanDistance(World.PartyCell) <= action.Range)
            {
                int cursor = _targetCursors.GetValueOrDefault(enemy.Definition.Id);
                PartyMemberState target = living[cursor % living.Length];
                settled.Add(_combat.ResolveOppositionAttack(enemy, target, living.Length, action, _rules.Tuning, Revision));
                _targetCursors[enemy.Definition.Id] = checked(cursor + 1);
            }
            ActivationIndex = checked(ActivationIndex + (ulong)_rules.Tuning.ActivationCost);
            if (Party.Living.Count == 0) { SetOutcome(SessionOutcome.Defeat); return settled; }
            AdvanceCursor();
        }
        return settled;
    }

    private bool HasPartyDecision() => Phase == SessionPhase.PartyDecision && DecisionClass == SessionDecisionClass.PartyActivation && CurrentActor is { Side: InitiativeSide.Party };

    private void SkipUnavailableActors()
    {
        while (Outcome == SessionOutcome.Ongoing && CurrentActor is InitiativeActorSnapshot current && !IsAvailable(current))
        {
            InitiativeCursor = checked(InitiativeCursor + 1);
            if (InitiativeCursor >= _initiative.Count) BuildRound();
        }
    }

    private bool IsAvailable(InitiativeActorSnapshot actor) => actor.Side switch
    {
        InitiativeSide.Party => Party.Find(actor.Id)?.IsLiving == true,
        InitiativeSide.Opposition => World.Find(actor.Id) is { IsLiving: true, Participating: true },
        _ => false,
    };

    private void AdvanceCursor()
    {
        InitiativeCursor = checked(InitiativeCursor + 1);
        if (InitiativeCursor >= _initiative.Count) BuildRound();
    }

    private void BuildRound()
    {
        if (Party.Living.Count == 0) { SetOutcome(SessionOutcome.Defeat); return; }
        if (!World.Opposition.Any(enemy => enemy.IsLiving)) { SetOutcome(SessionOutcome.Victory); return; }
        _initiative = Party.Living.Select(member => new InitiativeActorSnapshot(member.Definition.Id, InitiativeSide.Party, member.Definition.Finesse, member.Definition.EntityId))
            .Concat(World.Opposition.Where(enemy => enemy.IsLiving && enemy.Participating).Select(enemy => new InitiativeActorSnapshot(enemy.Definition.Id, InitiativeSide.Opposition, enemy.Definition.Finesse, enemy.Definition.EntityId)))
            .OrderByDescending(entry => entry.Finesse).ThenBy(entry => entry.EntityId).ToList();
        if (_initiative.Count == 0) throw new InvalidOperationException("initiative-has-no-admitted-actors");
        InitiativeCursor = 0;
        Round = checked(Round + 1);
    }

    private void SetOutcome(SessionOutcome outcome)
    {
        Outcome = outcome;
        Phase = SessionPhase.Terminal;
        DecisionClass = SessionDecisionClass.Terminal;
        InitiativeCursor = _initiative.Count;
    }

    private SessionCommandReceipt Accept(IReadOnlyList<CombatReceipt> receipts)
    {
        _receipts.AddRange(receipts);
        Revision = checked(Revision + 1);
        return new SessionCommandReceipt(true, "accepted", Revision, receipts);
    }

    private SessionCommandReceipt Reject(string code) => new(false, code, Revision, []);

    private void Adopt(GameSession candidate)
    {
        Party = candidate.Party; World = candidate.World; Phase = candidate.Phase; Outcome = candidate.Outcome; DecisionClass = candidate.DecisionClass;
        Revision = candidate.Revision; ActivationIndex = candidate.ActivationIndex; Round = candidate.Round; InitiativeCursor = candidate.InitiativeCursor;
        _initiative = candidate._initiative; _targetCursors = candidate._targetCursors; _receipts = candidate._receipts;
    }

    internal SessionCheckpoint Capture() => new(Phase, Outcome, DecisionClass, Revision, ActivationIndex, Round, InitiativeCursor, Initiative.ToArray(), Party.CaptureVitality(), World.PartyCell.X, World.PartyCell.Y, World.CaptureOpposition(), new Dictionary<string, int>(_targetCursors, StringComparer.Ordinal), Receipts.ToArray());

    internal static GameSession Restore(
        IRandomService random,
        FloorState floor,
        SessionCheckpoint checkpoint,
        OppositionVisibilityQuery visibilityQuery,
        Func<GridCell, GridCell, bool>? movementAdmission = null)
    {
        ArgumentNullException.ThrowIfNull(checkpoint);
        if (!Enum.IsDefined(checkpoint.Phase) || !Enum.IsDefined(checkpoint.Outcome) || !Enum.IsDefined(checkpoint.DecisionClass)
            || checkpoint.InitiativeCursor < 0 || checkpoint.InitiativeCursor > checkpoint.Initiative.Count
            || checkpoint.Initiative.Select(entry => entry.Id).Distinct(StringComparer.Ordinal).Count() != checkpoint.Initiative.Count
            || checkpoint.Initiative.Any(entry => !Enum.IsDefined(entry.Side) || String.IsNullOrWhiteSpace(entry.Id) || entry.Finesse < 0 || entry.EntityId == 0))
            throw new InvalidOperationException("session-save-shape-invalid");
        if (checkpoint.DecisionClass == SessionDecisionClass.AutomaticOpposition)
            throw new InvalidOperationException("session-save-transient-automatic-settlement");
        GameSession restored = new(random, floor, visibilityQuery, movementAdmission);
        restored.Party.RestoreVitality(checkpoint.Party);
        restored.World.Restore(new GridCell(checkpoint.PartyCellX, checkpoint.PartyCellY), checkpoint.Opposition);
        if (checkpoint.TargetCursors.Any(cursor => restored.World.Find(cursor.Key) is null || cursor.Value < 0 || (ulong)cursor.Value > checkpoint.ActivationIndex)) throw new InvalidOperationException("session-save-target-cursor-invalid");
        foreach (InitiativeActorSnapshot entry in checkpoint.Initiative)
        {
            if (entry.Side == InitiativeSide.Party)
            {
                PartyMemberState? member = restored.Party.Find(entry.Id);
                if (member is null || member.Definition.Finesse != entry.Finesse || member.Definition.EntityId != entry.EntityId)
                    throw new InvalidOperationException("session-save-initiative-invalid");
            }
            else
            {
                OppositionState? enemy = restored.World.Find(entry.Id);
                if (enemy is null || enemy.Definition.Finesse != entry.Finesse || enemy.Definition.EntityId != entry.EntityId)
                    throw new InvalidOperationException("session-save-initiative-invalid");
            }
        }
        if (checkpoint.Phase != SessionPhase.Terminal && checkpoint.Initiative.Count > 0
            && !checkpoint.Initiative.SequenceEqual(checkpoint.Initiative.OrderByDescending(entry => entry.Finesse).ThenBy(entry => entry.EntityId)))
            throw new InvalidOperationException("session-save-initiative-order-invalid");
        ValidateReceipts(restored, checkpoint.Receipts);
        InitiativeActorSnapshot? savedCurrent = checkpoint.InitiativeCursor < checkpoint.Initiative.Count
            ? checkpoint.Initiative[checkpoint.InitiativeCursor]
            : null;
        bool coherent = (checkpoint.Phase, checkpoint.Outcome, checkpoint.DecisionClass) switch
        {
            (SessionPhase.Preparation, SessionOutcome.Ongoing, SessionDecisionClass.Preparation) => checkpoint.Round == 0 && checkpoint.Initiative.Count == 0 && savedCurrent is null,
            (SessionPhase.PartyDecision, SessionOutcome.Ongoing, SessionDecisionClass.PartyActivation) => savedCurrent is { Side: InitiativeSide.Party }
                && restored.Party.Find(savedCurrent.Id)?.IsLiving == true,
            (SessionPhase.Terminal, SessionOutcome.Victory or SessionOutcome.Defeat, SessionDecisionClass.Terminal) => savedCurrent is null && checkpoint.InitiativeCursor == checkpoint.Initiative.Count,
            _ => false,
        };
        if (!coherent) throw new InvalidOperationException("session-save-decision-state-invalid");
        restored.Phase = checkpoint.Phase; restored.Outcome = checkpoint.Outcome; restored.DecisionClass = checkpoint.DecisionClass;
        restored.Revision = checkpoint.Revision; restored.ActivationIndex = checkpoint.ActivationIndex; restored.Round = checkpoint.Round; restored.InitiativeCursor = checkpoint.InitiativeCursor;
        restored._initiative = checkpoint.Initiative.ToList(); restored._targetCursors = new Dictionary<string, int>(checkpoint.TargetCursors, StringComparer.Ordinal); restored._receipts = checkpoint.Receipts.ToList();
        return restored;
    }

    private static void ValidateReceipts(GameSession restored, IReadOnlyList<CombatReceipt> receipts)
    {
        foreach (CombatReceipt receipt in receipts)
        {
            if (receipt is null || receipt.RequestedDamage < 0 || receipt.AppliedDamage < 0 || receipt.AppliedDamage > receipt.RequestedDamage
                || (!receipt.Hit && (receipt.RequestedDamage != 0 || receipt.AppliedDamage != 0))
                || receipt.Roll < restored.Tuning.AttackRollMinimum || receipt.Roll > restored.Tuning.AttackRollMaximum)
                throw new InvalidOperationException("session-save-receipt-bounds-invalid");

            PartyMemberState? partyAttacker = restored.Party.Find(receipt.Attacker);
            OppositionState? oppositionAttacker = restored.World.Find(receipt.Attacker);
            PartyMemberState? partyTarget = restored.Party.Find(receipt.Target);
            OppositionState? oppositionTarget = restored.World.Find(receipt.Target);
            bool partyAttack = partyAttacker is not null && oppositionTarget is not null
                && receipt.TargetPolicy == "hostile-cell" && receipt.EligibleMembers == 0;
            bool oppositionAttack = oppositionAttacker is not null && partyTarget is not null
                && receipt.TargetPolicy == "party-square-round-robin"
                && receipt.EligibleMembers > 0 && receipt.EligibleMembers <= restored.Party.Members.Count;
            long targetMaximum = partyAttack ? oppositionTarget!.Definition.Vitality : oppositionAttack ? partyTarget!.Definition.Vitality : -1;
            if ((!partyAttack && !oppositionAttack) || receipt.AppliedDamage > targetMaximum)
                throw new InvalidOperationException("session-save-receipt-identity-invalid");
        }
    }
}

/// <summary>Closed persistence state records current initiative rather than rederiving it during restore.</summary>
internal sealed record SessionCheckpoint(SessionPhase Phase, SessionOutcome Outcome, SessionDecisionClass DecisionClass, ulong Revision, ulong ActivationIndex, ulong Round, int InitiativeCursor, IReadOnlyList<InitiativeActorSnapshot> Initiative, IReadOnlyList<ActorVitalitySnapshot> Party, int PartyCellX, int PartyCellY, IReadOnlyList<OppositionSnapshot> Opposition, IReadOnlyDictionary<string, int> TargetCursors, IReadOnlyList<CombatReceipt> Receipts);
