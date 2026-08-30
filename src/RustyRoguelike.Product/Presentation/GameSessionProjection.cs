using Rusty.Engine;
using RustyRoguelike.Product.Combat;
using RustyRoguelike.Product.Exploration;
using RustyRoguelike.Product.Party;
using RustyRoguelike.Product.Session;

namespace RustyRoguelike.Product.Presentation;

/// <summary>Observer-only session readout. Values preserve the product's decision and tuning evidence without becoming a gameplay input lane.</summary>
internal sealed class GameSessionProjection : IDisposable
{
    private const string StreamName = "rusty-roguelike.session";
    private const string ContractName = "rusty-roguelike.session.v1";
    private readonly IUiService _ui;
    private readonly UiStream _stream;
    private ulong _sequence;

    internal GameSessionProjection(IUiService ui)
    {
        _ui = ui ?? throw new ArgumentNullException(nameof(ui));
        _stream = _ui.OpenStream(new UiStreamRequest(StreamName, ContractName));
    }

    internal void Publish(GameSession session)
    {
        LifecycleValueBuilder value = new();
        uint party = Party(value, session.Party.Members);
        uint opposition = Opposition(value, session.World.Opposition);
        uint latestReceipt = LatestReceipt(value, session.Receipts.LastOrDefault());
        uint initiative = Initiative(value, session.Initiative);
        uint tuning = value.Object(
            ("campaignSeed", value.Number(session.Tuning.CampaignSeed)),
            ("activationCost", value.Number(session.Tuning.ActivationCost)),
            ("automaticSettlementLimit", value.Number(session.Tuning.AutomaticSettlementLimit)),
            ("initiativePolicy", value.String(session.Tuning.InitiativePolicy)),
            ("rngScope", value.String(session.Tuning.RngScope)));
        uint root = value.Object(
            ("phase", value.String(session.Phase.ToString().ToLowerInvariant())),
            ("outcome", value.String(session.Outcome.ToString().ToLowerInvariant())),
            ("revision", value.Number(session.Revision)),
            ("activationIndex", value.Number(session.ActivationIndex)),
            ("round", value.Number(session.Round)),
            ("initiativeCursor", value.Number(session.InitiativeCursor)),
            ("currentActor", value.String(session.CurrentActor?.Id ?? "none")),
            ("decisionClass", value.String(session.DecisionClass.ToString().ToLowerInvariant())),
            ("initiative", initiative),
            ("partyCellX", value.Number(session.World.PartyCell.X)),
            ("partyCellY", value.Number(session.World.PartyCell.Y)),
            ("party", party),
            ("loadoutRevision", value.Number(session.Party.LoadoutRevision)),
            ("opposition", opposition),
            ("latestReceipt", latestReceipt),
            ("tuning", tuning),
            ("sourceFacts", value.String("csharp-starter-complete-opposition-roster; explicit typed definitions")));
        _ui.PublishProjection(new UiProjection(_stream, checked(++_sequence), value.Build(root)));
    }

    private static uint Party(LifecycleValueBuilder value, IReadOnlyList<PartyMemberState> members) => value.Object(
        members.Select(member => (member.Definition.Id, value.Object(
            ("name", value.String(member.Definition.Name)),
            ("vitality", value.Number(member.Vitality)),
            ("living", value.String(member.IsLiving ? "true" : "false"))))).ToArray());

    private static uint Opposition(LifecycleValueBuilder value, IReadOnlyList<OppositionState> enemies) => value.Object(
        enemies.Select(enemy => (enemy.Definition.Id, value.Object(
            ("vitality", value.Number(enemy.Vitality)),
            ("participating", value.String(enemy.Participating ? "true" : "false")),
            ("cellX", value.Number(enemy.Position.X)),
            ("cellY", value.Number(enemy.Position.Y))))).ToArray());

    private static uint Initiative(LifecycleValueBuilder value, IReadOnlyList<InitiativeActorSnapshot> actors) => value.Object(
        actors.Select((actor, index) => (actor.Id, value.Object(
            ("index", value.Number(index)),
            ("side", value.String(actor.Side.ToString().ToLowerInvariant())),
            ("finesse", value.Number(actor.Finesse)),
            ("entityId", value.Number(actor.EntityId))))).ToArray());

    private static uint LatestReceipt(LifecycleValueBuilder value, CombatReceipt? receipt) => receipt is null
        ? value.String("none")
        : value.Object(
            ("attacker", value.String(receipt.Attacker)),
            ("target", value.String(receipt.Target)),
            ("targetPolicy", value.String(receipt.TargetPolicy)),
            ("eligibleMembers", value.Number(receipt.EligibleMembers)),
            ("roll", value.Number(receipt.Roll)),
            ("defense", value.Number(receipt.Defense)),
            ("requestedDamage", value.Number(receipt.RequestedDamage)),
            ("appliedDamage", value.Number(receipt.AppliedDamage)));

    public void Dispose() => _stream.Dispose();
}
