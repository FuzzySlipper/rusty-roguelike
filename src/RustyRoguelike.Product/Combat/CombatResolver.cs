using Rusty.Engine;
using RustyRoguelike.Product.Exploration;
using RustyRoguelike.Product.Party;
using RustyRoguelike.Product.Rules;

namespace RustyRoguelike.Product.Combat;

internal sealed record CombatReceipt(
    string Attacker,
    string Target,
    string TargetPolicy,
    int EligibleMembers,
    long Roll,
    int AttackModifier,
    int Defense,
    int RequestedDamage,
    int AppliedDamage,
    bool Hit);

/// <summary>Builds one attack receipt from immutable facts before mutating one product-owned vitality track.</summary>
internal sealed class CombatResolver
{
    private readonly IRandomService _random;
    internal CombatResolver(IRandomService random) => _random = random ?? throw new ArgumentNullException(nameof(random));
    internal IRandomService Random => _random;

    internal CombatReceipt ResolvePartyAttack(
        PartyMemberState attacker,
        OppositionState target,
        ActionDefinition action,
        GameplayTuning tuning,
        ulong revision)
    {
        if (!attacker.IsLiving || !target.IsLiving || action.Target != TargetMode.HostileCell)
        {
            throw new InvalidOperationException("The requested party attack is not currently legal.");
        }
        int defense = target.Definition.Finesse / tuning.AbilityDefenseDivisor + tuning.BaseDefense;
        long roll = Draw(tuning, revision, attacker.Definition.Id, action.Id, tuning.AttackRollMinimum, tuning.AttackRollMaximum);
        int modifier = attacker.Ability(action.Ability) / tuning.AbilityDefenseDivisor;
        bool hit = checked(roll + modifier) >= defense;
        int requested = hit ? RollDamage(tuning, revision, attacker.Definition.Id, action) : 0;
        int applied = target.ApplyDamage(requested);
        return new CombatReceipt(attacker.Definition.Id, target.Definition.Id, "hostile-cell", 0, roll, modifier, defense, requested, applied, hit);
    }

    internal CombatReceipt ResolveOppositionAttack(
        OppositionState attacker,
        PartyMemberState target,
        int eligibleMembers,
        ActionDefinition action,
        GameplayTuning tuning,
        ulong revision)
    {
        if (!attacker.IsLiving || !target.IsLiving || action.Target != TargetMode.HostilePartySquare)
        {
            throw new InvalidOperationException("The requested opposition attack is not currently legal.");
        }
        int defense = target.Defense(action.Defense, tuning);
        long roll = Draw(tuning, revision, attacker.Definition.Id, action.Id, tuning.AttackRollMinimum, tuning.AttackRollMaximum);
        int modifier = (action.Ability switch
        {
            Ability.Might => attacker.Definition.Might,
            Ability.Finesse => attacker.Definition.Finesse,
            Ability.Intellect => attacker.Definition.Intellect,
            Ability.Spirit => attacker.Definition.Spirit,
            _ => throw new ArgumentOutOfRangeException(nameof(action)),
        }) / tuning.AbilityDefenseDivisor;
        bool hit = checked(roll + modifier) >= defense;
        int requested = hit ? RollDamage(tuning, revision, attacker.Definition.Id, action) : 0;
        int applied = target.ApplyDamage(requested);
        return new CombatReceipt(attacker.Definition.Id, target.Definition.Id, "party-square-round-robin", eligibleMembers, roll, modifier, defense, requested, applied, hit);
    }

    private long Draw(GameplayTuning tuning, ulong revision, string actor, string action, int minimum, int maximum) =>
        _random.DrawKeyed(new KeyedRngRequest(tuning.CampaignSeed, tuning.RngScope, $"{revision}:{actor}:{action}:attack", minimum, maximum)).Value;

    private int RollDamage(GameplayTuning tuning, ulong revision, string actor, ActionDefinition action)
    {
        int total = action.DamageBonus;
        for (int die = 0; die < action.DiceCount; die++)
        {
            total = checked(total + (int)Draw(tuning, revision, actor, action.Id + ":damage:" + die, 1, action.DiceSides));
        }
        return total;
    }
}
