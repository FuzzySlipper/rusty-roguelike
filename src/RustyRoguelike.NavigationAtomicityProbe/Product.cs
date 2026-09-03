using Rusty.Engine;
using RustyRoguelike.Product.Exploration;
using RustyRoguelike.Product.Integration;
using RustyRoguelike.Product.Presentation;
using RustyRoguelike.Product.Rules;
using RustyRoguelike.Product.Session;

namespace RustyRoguelike.NavigationAtomicityProbe;

public sealed class NavigationAtomicityProbeProduct : IEngineProduct
{
    private readonly IUiService _ui;
    private readonly UiStream _stream;
    private readonly FloorEngineProjection _floor;
    private bool _disposed;

    public NavigationAtomicityProbeProduct(ProductCreateContext context)
    {
        ArgumentNullException.ThrowIfNull(context);
        FloorEngineProjection? floor = null;
        UiStream? stream = null;
        try
        {
            floor = FloorEngineProjection.Create(context.Engine);
            stream = context.Engine.Ui.OpenStream(new UiStreamRequest(
                "rusty-roguelike.navigation-atomicity",
                "rusty-roguelike.navigation-atomicity.v1"));
            _ui = context.Engine.Ui;
            _stream = stream;
            _floor = floor;
            RunProof(context.Engine.Random);
        }
        catch
        {
            stream?.Dispose();
            floor?.Dispose();
            throw;
        }
    }

    public void Start() { }
    public void Attach() { }
    public ProductUpdateResult Update(ProductUpdate update) => ProductUpdateResult.None;
    public void Pause() { }
    public void Resume() { }
    public void Restart() { }
    public void Shutdown() => Dispose();

    public void Dispose()
    {
        if (_disposed) return;
        _stream.Dispose();
        _floor.Dispose();
        _disposed = true;
    }

    private void RunProof(IRandomService random)
    {
        int visibilityQueries = 0;
        IReadOnlySet<ulong> FailAfterMovement(
            GridCell partyCell,
            IReadOnlyList<OppositionState> opposition,
            ExplorationTuning tuning)
        {
            visibilityQueries++;
            if (visibilityQueries == 2)
            {
                throw new InvalidOperationException("injected-post-admission-settlement-failure");
            }

            return _floor.QueryVisibleOpposition(partyCell, opposition, tuning);
        }

        var session = new GameSession(
            random,
            _floor.Floor,
            FailAfterMovement,
            _floor.ProposePartyStep);
        Require(session.Submit(new BeginExpeditionCommand(0)).Accepted, "probe-begin-rejected");

        GridCell from = session.World.PartyCell;
        GridCell destination = from.Step(1, 0);
        // Equal length keeps both paths fully sampled; reversed cells expose a mutating proposal.
        NavigationPathReadout seeded = _floor.SeedNavigationPath(destination, from);
        Require(
            seeded.Outcome == NavigationPathOutcome.Reached && seeded.PathLen == 2,
            "probe-navigation-sentinel-rejected");
        FloorNavigationState navigationBefore = _floor.ReadNavigationState(seeded.PathLen);
        string productBefore = Fingerprint(session.Capture());

        SessionCommandReceipt failed = session.Submit(
            new MovePartyCommand(session.Revision, 1, 0));

        FloorNavigationState navigationAfter = _floor.ReadNavigationState(seeded.PathLen);
        string productAfter = Fingerprint(session.Capture());
        Require(
            !failed.Accepted && failed.Code == "command-settlement-failed",
            "probe-late-settlement-did-not-fail");
        Require(productAfter == productBefore, "probe-product-state-mutated");
        Require(navigationAfter.SameAs(navigationBefore), "probe-engine-navigation-mutated");

        LifecycleValueBuilder value = new();
        uint root = value.Object(
            ("accepted", value.String("true")),
            ("settlementCode", value.String(failed.Code)),
            ("productStateUnchanged", value.String("true")),
            ("engineNavigationUnchanged", value.String("true")),
            ("retainedPathLength", value.Number(seeded.PathLen)),
            ("navigationRevision", value.Number(navigationBefore.Navigation.NavigationRevision)),
            ("pathHash", value.String($"0x{seeded.PathHash:x16}")));
        _ui.PublishProjection(new UiProjection(_stream, 1, value.Build(root)));
    }

    private static string Fingerprint(SessionCheckpoint state) => string.Join("|",
        state.Phase,
        state.Outcome,
        state.DecisionClass,
        state.Revision,
        state.ActivationIndex,
        state.Round,
        state.InitiativeCursor,
        string.Join(",", state.Initiative.Select(actor => $"{actor.Id}:{actor.Side}:{actor.Finesse}:{actor.EntityId}")),
        string.Join(",", state.Party.Select(actor => $"{actor.Id}:{actor.Vitality}")),
        state.PartyCellX,
        state.PartyCellY,
        string.Join(",", state.Opposition.Select(actor => $"{actor.Id}:{actor.Vitality}:{actor.Participating}:{actor.X}:{actor.Y}")),
        string.Join(",", state.TargetCursors.OrderBy(pair => pair.Key).Select(pair => $"{pair.Key}:{pair.Value}")),
        string.Join(",", state.Receipts.Select(receipt => string.Join(":",
            receipt.Attacker,
            receipt.Target,
            receipt.TargetPolicy,
            receipt.EligibleMembers,
            receipt.Roll,
            receipt.AttackModifier,
            receipt.Defense,
            receipt.RequestedDamage,
            receipt.AppliedDamage,
            receipt.Hit))));

    private static void Require(bool condition, string message)
    {
        if (!condition) throw new InvalidOperationException(message);
    }
}
