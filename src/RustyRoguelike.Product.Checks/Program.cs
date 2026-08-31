using Rusty.Engine;
using RustyRoguelike.Product.Combat;
using RustyRoguelike.Product.Exploration;
using RustyRoguelike.Product.Floors;
using RustyRoguelike.Product.Rules;
using RustyRoguelike.Product.Session;

CheckFloorAdmission();
CheckInitiative();
CheckDeadActorPruning();
CheckAtomicRngFailure();
CheckAtomicMoveFailure();
CheckOppositionTrackBounds();
CheckRestoreValidation();
Console.WriteLine("Roguelike focused product checks passed");

static void CheckFloorAdmission()
{
    string artifactPath = Path.Combine(AppContext.BaseDirectory, "content", "starter-floor.5201.procgen.json");
    FloorArtifactAdmissionProbe.Verify(File.ReadAllBytes(artifactPath));
}

static void CheckInitiative()
{
    var random = new ProbeRandom();
    GameSession session = NewSession(random);
    Require(session.Submit(new BeginExpeditionCommand(0)).Accepted, "begin failed");
    Require(session.CurrentActor?.Id == "kestrel", "finesse order did not select kestrel");
    string before = Fingerprint(session.Capture());
    SessionCommandReceipt outOfTurn = session.Submit(new UseActionCommand(session.Revision, "brann", "guardian-strike", "goblin-scrapper"));
    Require(!outOfTurn.Accepted && outOfTurn.Code == "out-of-turn-actor" && Fingerprint(session.Capture()) == before, "out-of-turn command changed state");
    Require(session.Submit(new WaitCommand(session.Revision)).Accepted, "kestrel wait failed");
    Require(session.Receipts.Count(receipt => receipt.Attacker == "goblin-scrapper") == 1, "admitted enemy did not act exactly once before next party decision");
    Require(session.Submit(new WaitCommand(session.Revision)).Accepted, "mira wait failed");
    Require(session.Submit(new WaitCommand(session.Revision)).Accepted, "brann wait failed");
    Require(session.Round == 2 && session.CurrentActor?.Id == "kestrel", "round cursor did not rebuild deterministically");
}

static void CheckAtomicRngFailure()
{
    var random = new ProbeRandom { FailOnDraw = 3 };
    GameSession session = NewSession(random);
    Require(session.Submit(new BeginExpeditionCommand(0)).Accepted, "begin failed");
    string before = Fingerprint(session.Capture());
    SessionCommandReceipt failed = session.Submit(new UseActionCommand(session.Revision, "kestrel", "aimed-shot", "goblin-scrapper"));
    Require(!failed.Accepted && failed.Code == "command-settlement-failed", "injected RNG failure was accepted");
    Require(Fingerprint(session.Capture()) == before, "injected RNG failure changed the complete live checkpoint");
}

static void CheckAtomicMoveFailure()
{
    var random = new ProbeRandom { FailOnDraw = 1 };
    var navigation = new NavigationAdmissionProbe();
    GameSession session = NewSession(random, navigation.Evaluate);
    Require(session.Submit(new BeginExpeditionCommand(0)).Accepted, "begin failed");
    string productBefore = Fingerprint(session.Capture());
    string navigationBefore = navigation.Fingerprint();

    SessionCommandReceipt failed = session.Submit(new MovePartyCommand(session.Revision, 1, 0));

    Require(!failed.Accepted && failed.Code == "command-settlement-failed", "late move settlement failure was accepted");
    Require(navigation.EvaluationCount == 1, "move did not pass through navigation admission before settlement failed");
    Require(Fingerprint(session.Capture()) == productBefore, "late move settlement failure changed the complete live checkpoint");
    Require(navigation.Fingerprint() == navigationBefore, "late move settlement failure changed retained navigation readout state");
}

static void CheckDeadActorPruning()
{
    var random = new ProbeRandom();
    GameSession session = NewSession(random);
    Require(session.Submit(new BeginExpeditionCommand(0)).Accepted, "begin failed");
    Require(session.Submit(new UseActionCommand(session.Revision, "kestrel", "aimed-shot", "goblin-scrapper")).Accepted, "first aimed shot failed");
    Require(session.Submit(new WaitCommand(session.Revision)).Accepted, "mira wait failed");
    Require(session.Submit(new WaitCommand(session.Revision)).Accepted, "brann wait failed");
    ulong before = session.ActivationIndex;
    Require(session.Submit(new UseActionCommand(session.Revision, "kestrel", "aimed-shot", "goblin-scrapper")).Accepted, "second aimed shot failed");
    Require(session.World.Find("goblin-scrapper")?.IsLiving == false, "probe did not defeat goblin before its turn");
    Require(session.ActivationIndex == before + 1 && session.CurrentActor?.Id == "mira", "defeated actor consumed an activation or became a decision");
}

static GameSession NewSession(IRandomService random, Func<GridCell, GridCell, bool>? movementAdmission = null)
{
    return new GameSession(random, NewProbeFloor(), VisibleProbe, movementAdmission);
}

static FloorState NewProbeFloor()
{
    FloorCell[] cells = Enumerable.Range(0, 20).Select(x => new FloorCell(x, 0)).ToArray();
    FloorState floor = new(
        "probe-floor", new FloorBounds(0, 0, 20, 1), cells,
        [new FloorRegion("main", "probe", "room", cells, [])],
        [new FloorFeature("entry", "entry", "entry", new FloorCell(0, 0))], [], [],
        new FloorProvenance(2, "probe", 1, 2, 3, 4, "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", 0));
    return floor;
}

static IReadOnlySet<ulong> VisibleProbe(GridCell party, IReadOnlyList<OppositionState> opposition, ExplorationTuning tuning) =>
    opposition.Where(enemy => enemy.Position.ManhattanDistance(party) <= tuning.AdmissionRadius)
        .Select(enemy => enemy.Definition.EntityId).ToHashSet();

static void CheckOppositionTrackBounds()
{
    ActorDefinition definition = new("probe", 9_001, "Probe", "Probe", false, 5, 1, 1, 1, 1, [], []);
    var opposition = new OppositionState(definition, new GridCell(1, 0));
    Require(opposition.ApplyDamage(99) == 5 && opposition.Vitality == 0, "opposition track did not clamp damage at zero");
    opposition.Restore(5, false, new GridCell(1, 0));
    Require(opposition.Vitality == 5, "opposition track did not restore its exact maximum");
}

static void CheckRestoreValidation()
{
    var random = new ProbeRandom();
    GameSession session = NewSession(random);
    Require(session.Submit(new BeginExpeditionCommand(0)).Accepted, "begin failed");
    SessionCheckpoint checkpoint = session.Capture();
    InitiativeActorSnapshot badSide = checkpoint.Initiative[0] with { Side = (InitiativeSide)99 };
    RequireRestoreReject(checkpoint with { Initiative = [badSide] }, "invalid initiative enum accepted");
    CombatReceipt malformedReceipt = new("not-an-actor", "goblin-scrapper", "hostile-cell", 0, 20, 0, 0, 1, 1, true);
    RequireRestoreReject(checkpoint with { Receipts = [malformedReceipt] }, "unknown receipt identity accepted");
    CombatReceipt outOfBoundsReceipt = new("kestrel", "goblin-scrapper", "hostile-cell", 0, 20, 0, 0, 1, 2, true);
    RequireRestoreReject(checkpoint with { Receipts = [outOfBoundsReceipt] }, "malformed receipt bounds accepted");
    RequireRestoreReject(checkpoint with { InitiativeCursor = checkpoint.Initiative.Count }, "incoherent decision cursor accepted");
}

static void RequireRestoreReject(SessionCheckpoint checkpoint, string message)
{
    try
    {
        _ = GameSession.Restore(new ProbeRandom(), NewProbeFloor(), checkpoint, VisibleProbe);
        throw new InvalidOperationException(message);
    }
    catch (InvalidOperationException exception) when (exception.Message != message)
    {
    }
}

static string Fingerprint(SessionCheckpoint state) => string.Join("|",
    state.Phase, state.Outcome, state.DecisionClass, state.Revision, state.ActivationIndex, state.Round, state.InitiativeCursor,
    string.Join(",", state.Initiative.Select(actor => $"{actor.Id}:{actor.Side}:{actor.Finesse}:{actor.EntityId}")),
    string.Join(",", state.Party.Select(actor => $"{actor.Id}:{actor.Vitality}")),
    state.PartyCellX, state.PartyCellY,
    string.Join(",", state.Opposition.Select(actor => $"{actor.Id}:{actor.Vitality}:{actor.Participating}:{actor.X}:{actor.Y}")),
    string.Join(",", state.TargetCursors.OrderBy(pair => pair.Key).Select(pair => $"{pair.Key}:{pair.Value}")),
    string.Join(",", state.Receipts.Select(receipt => $"{receipt.Attacker}:{receipt.Target}:{receipt.Roll}:{receipt.AppliedDamage}")));

static void Require(bool condition, string message)
{
    if (!condition) throw new InvalidOperationException(message);
}

sealed class ProbeRandom : IRandomService
{
    private int _draws;
    public int? FailOnDraw { get; init; }
    public KeyedRngReceipt DrawKeyed(KeyedRngRequest request)
    {
        if (++_draws == FailOnDraw) throw new InvalidOperationException("injected-rng-failure");
        return new KeyedRngReceipt(request.Maximum);
    }
    public Rng CreateScoped(ScopedRngCreateRequest request) => throw new NotSupportedException();
    public Rng ForkScoped(ScopedRngForkRequest request) => throw new NotSupportedException();
    public RngValue NextU64(Rng stream) => throw new NotSupportedException();
    public RngValue NextBoundedU32(ScopedRngBoundedRequest request) => throw new NotSupportedException();
    public RngValue NextBool(Rng stream) => throw new NotSupportedException();
}

sealed class NavigationAdmissionProbe
{
    private readonly GridCell[] _retainedPath = [new(0, 0), new(1, 0), new(2, 0)];

    public int EvaluationCount { get; private set; }

    public bool Evaluate(GridCell from, GridCell destination)
    {
        EvaluationCount++;
        return destination == from.Step(1, 0);
    }

    public string Fingerprint() => string.Join(";", _retainedPath.Select(cell => $"{cell.X},{cell.Y}"));
}
