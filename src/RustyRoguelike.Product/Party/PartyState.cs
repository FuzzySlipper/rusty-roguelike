using Rusty.Engine.Mechanics;
using RustyRoguelike.Product.Rules;
using Rules = RustyRoguelike.Product.Rules;

namespace RustyRoguelike.Product.Party;

internal sealed class PartyMemberState
{
    private static readonly TrackId VitalityTrackId = TrackId.Parse("roguelike.vitality");
    private readonly ExactTrack _vitality;

    internal PartyMemberState(ActorDefinition definition, RoguelikeRules rules)
    {
        Definition = definition;
        IReadOnlyList<RustyRoguelike.Product.Rules.ItemDefinition> equipped = definition.Items.Select(id => rules.Items[id]).ToArray();
        EquippedItems = equipped;
        _vitality = new ExactTrack(
            new ExactTrackDefinition(VitalityTrackId, ExactValue.Zero, new ExactTrackMaximum.Fixed(new ExactValue(definition.Vitality))),
            new ExactValue(definition.Vitality));
    }

    internal ActorDefinition Definition { get; }
    internal IReadOnlyList<RustyRoguelike.Product.Rules.ItemDefinition> EquippedItems { get; }
    internal long Vitality => _vitality.Current.Raw;
    internal bool IsLiving => Vitality > 0;

    internal int Ability(Rules.Ability ability) => ability switch
    {
        Rules.Ability.Might => Definition.Might,
        Rules.Ability.Finesse => Definition.Finesse,
        Rules.Ability.Intellect => Definition.Intellect,
        Rules.Ability.Spirit => Definition.Spirit,
        _ => throw new ArgumentOutOfRangeException(nameof(ability)),
    };

    internal int Defense(Rules.Defense defense, GameplayTuning tuning) =>
        tuning.BaseDefense + (Ability(defense switch
        {
            Rules.Defense.Armor => Rules.Ability.Finesse,
            Rules.Defense.Grit => Rules.Ability.Might,
            Rules.Defense.Wits => Rules.Ability.Intellect,
            Rules.Defense.Nerve => Rules.Ability.Spirit,
            _ => throw new ArgumentOutOfRangeException(nameof(defense)),
        }) / tuning.AbilityDefenseDivisor) + EquippedItems.Where(item => item.Defense == defense).Sum(item => item.DefenseBonus);

    internal int ApplyDamage(int requested)
    {
        int applied = checked((int)Math.Min(Vitality, requested));
        _vitality.Spend(new ExactValue(applied));
        return applied;
    }

    internal void RestoreVitality(long value)
    {
        if (value < 0 || value > Definition.Vitality)
        {
            throw new InvalidOperationException("party-vitality-out-of-range");
        }

        ApplyDamage(checked((int)(Definition.Vitality - value)));
    }
}

internal sealed class PartyState
{
    private readonly List<PartyMemberState> _members;
    private readonly LoadoutMechanics _loadout;
    internal PartyState(RoguelikeRules rules)
    {
        _members = rules.Party.Select(definition => new PartyMemberState(definition, rules)).ToList();
        _loadout = new LoadoutMechanics(_members);
    }
    internal IReadOnlyList<PartyMemberState> Members => _members;
    internal IReadOnlyList<PartyMemberState> Living => _members.Where(member => member.IsLiving).ToArray();
    internal PartyMemberState? Find(string id) => _members.SingleOrDefault(member => member.Definition.Id == id);
    internal ulong LoadoutRevision => _loadout.Revision;

    internal IReadOnlyList<ActorVitalitySnapshot> CaptureVitality() => _members
        .Select(member => new ActorVitalitySnapshot(member.Definition.Id, member.Vitality))
        .ToArray();

    internal void RestoreVitality(IReadOnlyList<ActorVitalitySnapshot> saved)
    {
        if (saved.Count != _members.Count || saved.Select(member => member.Id).Distinct(StringComparer.Ordinal).Count() != saved.Count)
        {
            throw new InvalidOperationException("party-save-roster-mismatch");
        }

        foreach (ActorVitalitySnapshot snapshot in saved)
        {
            PartyMemberState member = Find(snapshot.Id) ?? throw new InvalidOperationException("party-save-member-unknown");
            member.RestoreVitality(snapshot.Vitality);
        }
    }
}

internal sealed record ActorVitalitySnapshot(string Id, long Vitality);
