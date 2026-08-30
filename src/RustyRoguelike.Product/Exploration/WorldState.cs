using RustyRoguelike.Product.Rules;
using RustyRoguelike.Product.Floors;

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

    internal void Restore(long vitality, bool participating, GridCell position)
    {
        if (vitality < 0 || vitality > Definition.Vitality)
        {
            throw new InvalidOperationException("opposition-vitality-out-of-range");
        }

        Vitality = checked((int)vitality);
        Participating = participating;
        Position = position;
    }
}

/// <summary>Product-owned floor participation policy; floor topology itself is admitted by the dedicated floor domain.</summary>
internal sealed class ExplorationState
{
    private readonly List<OppositionState> _opposition;
    internal ExplorationState(RoguelikeRules rules, FloorState floor)
    {
        ArgumentNullException.ThrowIfNull(floor);
        Floor = floor;
        FloorCell entry = floor.Features.Single(feature => feature.Kind == "entry").Cell;
        PartyCell = new GridCell(entry.X, entry.Y);
        FloorCell[] oppositionCells = floor.WalkableCells
            .Where(cell => cell != entry)
            .OrderBy(cell => cell.Y)
            .ThenBy(cell => cell.X)
            .ToArray();
        if (oppositionCells.Length < rules.Opposition.Count)
        {
            throw new InvalidOperationException("floor-has-insufficient-opposition-cells");
        }

        _opposition = rules.Opposition
            .Select((definition, index) => new OppositionState(definition, new GridCell(oppositionCells[index].X, oppositionCells[index].Y)))
            .ToList();
    }
    internal FloorState Floor { get; }
    internal GridCell PartyCell { get; private set; }
    internal IReadOnlyList<OppositionState> Opposition => _opposition;
    internal bool IsWalkable(GridCell cell) => Floor.WalkableCells.Contains(new FloorCell(cell.X, cell.Y));
    internal void MoveParty(GridCell destination)
    {
        if (!IsWalkable(destination))
        {
            throw new InvalidOperationException("destination-not-admitted-by-floor");
        }

        PartyCell = destination;
    }
    internal void AdmitVisibleOpposition() =>
        _opposition.Where(enemy => enemy.IsLiving && enemy.Position.ManhattanDistance(PartyCell) <= 1).ToList().ForEach(enemy => enemy.Participating = true);
    internal OppositionState? Find(string id) => _opposition.SingleOrDefault(enemy => enemy.Definition.Id == id);

    internal IReadOnlyList<OppositionSnapshot> CaptureOpposition() => _opposition
        .Select(enemy => new OppositionSnapshot(enemy.Definition.Id, enemy.Vitality, enemy.Participating, enemy.Position.X, enemy.Position.Y))
        .ToArray();

    internal void Restore(GridCell partyCell, IReadOnlyList<OppositionSnapshot> opposition)
    {
        if (!IsWalkable(partyCell) || opposition.Count != _opposition.Count
            || opposition.Select(enemy => enemy.Id).Distinct(StringComparer.Ordinal).Count() != opposition.Count)
        {
            throw new InvalidOperationException("world-save-shape-invalid");
        }

        HashSet<GridCell> occupied = [];
        foreach (OppositionSnapshot saved in opposition)
        {
            OppositionState state = Find(saved.Id) ?? throw new InvalidOperationException("world-save-opposition-unknown");
            GridCell position = new(saved.X, saved.Y);
            if (!IsWalkable(position) || position == partyCell || !occupied.Add(position))
            {
                throw new InvalidOperationException("world-save-position-invalid");
            }

            state.Restore(saved.Vitality, saved.Participating, position);
        }

        PartyCell = partyCell;
    }
}

internal sealed record OppositionSnapshot(string Id, long Vitality, bool Participating, int X, int Y);
