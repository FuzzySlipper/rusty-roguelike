using RustyRoguelike.Product.Rules;

namespace RustyRoguelike.Product.Exploration;

internal sealed class OppositionState
{
    internal OppositionState(ActorDefinition definition, GridCell position)
    {
        Definition = definition;
        Position = position;
        Vitality = definition.Vitality;
    }
    internal ActorDefinition Definition { get; }
    internal GridCell Position { get; set; }
    internal int Vitality { get; private set; }
    internal bool Participating { get; set; }
    internal bool IsLiving => Vitality > 0;
    internal int ApplyDamage(int requested)
    {
        int applied = Math.Min(Vitality, requested);
        Vitality = checked(Vitality - applied);
        return applied;
    }
}

/// <summary>Product-owned floor participation policy; floor topology itself is admitted by the dedicated floor domain.</summary>
internal sealed class ExplorationState
{
    private readonly List<OppositionState> _opposition;
    internal ExplorationState(RoguelikeRules rules)
    {
        PartyCell = rules.Tuning.EntryCell;
        _opposition = rules.Opposition.Select((definition, index) => new OppositionState(definition, new GridCell(index + 1, 0))).ToList();
    }
    internal GridCell PartyCell { get; private set; }
    internal IReadOnlyList<OppositionState> Opposition => _opposition;
    internal void MoveParty(GridCell destination) => PartyCell = destination;
    internal void AdmitVisibleOpposition() =>
        _opposition.Where(enemy => enemy.IsLiving && enemy.Position.ManhattanDistance(PartyCell) <= 1).ToList().ForEach(enemy => enemy.Participating = true);
    internal OppositionState? Find(string id) => _opposition.SingleOrDefault(enemy => enemy.Definition.Id == id);
}
