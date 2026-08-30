namespace RustyRoguelike.Product.Rules;

internal enum Ability { Might, Finesse, Intellect, Spirit }
internal enum Defense { Armor, Grit, Wits, Nerve }
internal enum TargetMode { HostileCell, HostilePartySquare }

internal readonly record struct GridCell(int X, int Y)
{
    internal int ManhattanDistance(GridCell other) => Math.Abs(X - other.X) + Math.Abs(Y - other.Y);
    internal GridCell Step(int deltaX, int deltaY) => new(checked(X + deltaX), checked(Y + deltaY));
}

internal sealed record ActionDefinition(
    string Id,
    string Name,
    Ability Ability,
    Defense Defense,
    TargetMode Target,
    int Range,
    int DiceCount,
    int DiceSides,
    int DamageBonus);

internal sealed record ItemDefinition(string Id, string Name, string Slot, string? GrantsAction, Defense? Defense, int DefenseBonus);

internal sealed record ActorDefinition(
    string Id,
    ulong EntityId,
    string Name,
    string Title,
    bool IsParty,
    int Vitality,
    int Might,
    int Finesse,
    int Intellect,
    int Spirit,
    IReadOnlyList<string> Actions,
    IReadOnlyList<string> Items);

internal sealed record GameplayTuning(
    ulong CampaignSeed,
    int PartySize,
    int ActivationCost,
    int BaseDefense,
    int AbilityDefenseDivisor,
    int AttackRollMinimum,
    int AttackRollMaximum,
    int AutomaticSettlementLimit,
    string InitiativePolicy,
    GridCell EntryCell,
    string RngScope)
{
    internal static GameplayTuning Starter { get; } = new(
        CampaignSeed: 7_554,
        PartySize: 3,
        ActivationCost: 1,
        BaseDefense: 8,
        AbilityDefenseDivisor: 2,
        AttackRollMinimum: 1,
        AttackRollMaximum: 20,
        AutomaticSettlementLimit: 32,
        InitiativePolicy: "living-party-plus-admitted-opposition:finesse-descending,entity-id-ascending; automatic-opposition-to-next-party-decision",
        EntryCell: new GridCell(0, 0),
        RngScope: "rusty-roguelike.combat.v1");
}

/// <summary>Small, inspectable starter corpus. It deliberately retains domain-shaped definitions rather than a generic evaluator.</summary>
internal sealed class RoguelikeRules
{
    private RoguelikeRules(
        GameplayTuning tuning,
        IReadOnlyList<ActorDefinition> party,
        IReadOnlyList<ActorDefinition> opposition,
        IReadOnlyDictionary<string, ActionDefinition> actions,
        IReadOnlyDictionary<string, ItemDefinition> items)
    {
        Tuning = tuning;
        Party = party;
        Opposition = opposition;
        Actions = actions;
        Items = items;
    }

    internal GameplayTuning Tuning { get; }
    internal IReadOnlyList<ActorDefinition> Party { get; }
    internal IReadOnlyList<ActorDefinition> Opposition { get; }
    internal IReadOnlyDictionary<string, ActionDefinition> Actions { get; }
    internal IReadOnlyDictionary<string, ItemDefinition> Items { get; }

    internal static RoguelikeRules Starter { get; } = CreateStarter();

    private static RoguelikeRules CreateStarter()
    {
        GameplayTuning tuning = GameplayTuning.Starter;
        var actions = new[]
        {
            new ActionDefinition("guardian-strike", "Guardian Strike", Ability.Might, Defense.Armor, TargetMode.HostileCell, 1, 1, 8, 2),
            new ActionDefinition("sweeping-strike", "Sweeping Strike", Ability.Might, Defense.Grit, TargetMode.HostileCell, 1, 1, 10, 0),
            new ActionDefinition("aimed-shot", "Aimed Shot", Ability.Finesse, Defense.Armor, TargetMode.HostileCell, 6, 1, 8, 1),
            new ActionDefinition("quick-shot", "Quick Shot", Ability.Finesse, Defense.Wits, TargetMode.HostileCell, 4, 1, 6, 0),
            new ActionDefinition("arcane-bolt", "Arcane Bolt", Ability.Intellect, Defense.Wits, TargetMode.HostileCell, 5, 1, 8, 1),
            new ActionDefinition("flame-burst", "Flame Burst", Ability.Intellect, Defense.Grit, TargetMode.HostileCell, 4, 2, 6, 0),
            new ActionDefinition("mind-spike", "Mind Spike", Ability.Spirit, Defense.Nerve, TargetMode.HostileCell, 3, 1, 10, 0),
            new ActionDefinition("rusty-blade", "Rusty Blade", Ability.Finesse, Defense.Armor, TargetMode.HostilePartySquare, 1, 1, 6, 0),
            new ActionDefinition("ember-shot", "Ember Shot", Ability.Spirit, Defense.Nerve, TargetMode.HostilePartySquare, 4, 1, 6, 1),
        }.ToDictionary(action => action.Id, StringComparer.Ordinal);
        var items = new[]
        {
            new ItemDefinition("longsword", "Longsword", "weapon", "sweeping-strike", null, 0),
            new ItemDefinition("scale-mail", "Scale Mail", "body", null, Defense.Armor, 2),
            new ItemDefinition("shortbow", "Shortbow", "weapon", "quick-shot", null, 0),
            new ItemDefinition("leather-armor", "Leather Armor", "body", null, Defense.Armor, 1),
            new ItemDefinition("ash-staff", "Ash Staff", "weapon", null, null, 0),
            new ItemDefinition("focus-orb", "Focus Orb", "focus", "mind-spike", Defense.Wits, 1),
            new ItemDefinition("traveling-robes", "Traveling Robes", "body", null, null, 0),
            new ItemDefinition("rusty-knife", "Rusty Knife", "weapon", null, null, 0),
            new ItemDefinition("ember-focus", "Ember Focus", "focus", null, Defense.Nerve, 1),
        }.ToDictionary(item => item.Id, StringComparer.Ordinal);
        ActorDefinition[] party =
        [
            new("brann", 101, "Brann", "Shield of the Road", true, 28, 16, 11, 9, 12, ["guardian-strike", "sweeping-strike"], ["longsword", "scale-mail"]),
            new("kestrel", 102, "Kestrel", "Pathfinder", true, 21, 10, 17, 12, 11, ["aimed-shot", "quick-shot"], ["shortbow", "leather-armor"]),
            new("mira", 103, "Mira", "Lantern Adept", true, 18, 8, 12, 17, 15, ["arcane-bolt", "flame-burst", "mind-spike"], ["ash-staff", "focus-orb", "traveling-robes"]),
        ];
        ActorDefinition Goblin(string id, ulong entityId, string name, string title, int vitality, int might, int finesse, int intellect, int spirit) =>
            new(id, entityId, name, title, false, vitality, might, finesse, intellect, spirit, ["rusty-blade"], ["rusty-knife"]);
        ActorDefinition Ember(string id, ulong entityId, string name, string title, int vitality, int might, int finesse, int intellect, int spirit) =>
            new(id, entityId, name, title, false, vitality, might, finesse, intellect, spirit, ["ember-shot"], ["ember-focus"]);
        ActorDefinition[] opposition =
        [
            Goblin("goblin-scrapper", 201, "Goblin Scrapper", "Dormant Raider", 12, 11, 14, 8, 9),
            Ember("ember-watcher", 202, "Ember Watcher", "Dormant Sentinel", 14, 8, 12, 11, 15),
            Goblin("tunnel-runner", 104, "Tunnel Runner", "Knife in the Dark", 9, 9, 15, 8, 9),
            Ember("cinder-eye", 105, "Cinder Eye", "Ashen Lookout", 10, 7, 13, 10, 14),
            Goblin("slag-cutpurse", 106, "Slag Cutpurse", "Ruin Lurker", 9, 10, 14, 9, 8),
            Goblin("ash-skirmisher", 107, "Ash Skirmisher", "Cinder Vanguard", 11, 12, 13, 8, 9),
            Ember("furnace-lookout", 108, "Furnace Lookout", "Coal-Eyed Sentry", 12, 8, 12, 12, 14),
            Goblin("soot-stalker", 109, "Soot Stalker", "Smoke-Shrouded Knife", 9, 9, 15, 9, 8),
            Ember("coal-sentry", 110, "Coal Sentry", "Banked-Flame Guard", 13, 9, 11, 11, 15),
            Goblin("clinker-knife", 111, "Clinker Knife", "Rubble Ambusher", 10, 10, 14, 8, 10),
            Ember("flare-watcher", 112, "Flare Watcher", "Bright-Eyed Hunter", 11, 7, 13, 12, 14),
            Goblin("ruin-scuttler", 113, "Ruin Scuttler", "Broken-Stone Prowler", 9, 9, 15, 8, 9),
            Ember("ember-seer", 114, "Ember Seer", "Furnace Oracle", 14, 8, 12, 13, 15),
            Goblin("slag-runner", 115, "Slag Runner", "Molten-Track Raider", 10, 11, 14, 8, 9),
            Goblin("cinder-stalker", 116, "Cinder Stalker", "Last-Light Pursuer", 11, 10, 15, 9, 8),
        ];
        return new RoguelikeRules(tuning, party, opposition, actions, items);
    }
}
