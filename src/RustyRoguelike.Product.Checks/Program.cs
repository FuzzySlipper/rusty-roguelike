using Rusty.Engine;
using RustyRoguelike.Product.Floors;
using RustyRoguelike.Product.Session;

CheckInitiative();
CheckDeadActorPruning();
CheckAtomicRngFailure();
Console.WriteLine("Roguelike focused session checks passed");

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

static GameSession NewSession(IRandomService random)
{
    FloorCell[] cells = Enumerable.Range(0, 20).Select(x => new FloorCell(x, 0)).ToArray();
    FloorState floor = new(
        "probe-floor", new FloorBounds(0, 0, 20, 1), cells,
        [new FloorRegion("main", "probe", "room", cells, [])],
        [new FloorFeature("entry", "entry", "entry", new FloorCell(0, 0))], [], [],
        new FloorProvenance(2, "probe", 1, 2, 3, 4, "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", 0));
    return new GameSession(random, floor);
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
