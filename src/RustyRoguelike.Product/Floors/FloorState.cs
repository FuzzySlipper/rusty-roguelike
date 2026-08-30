using System.Collections.ObjectModel;

namespace RustyRoguelike.Product.Floors;

/// <summary>
/// Immutable, product-owned meaning admitted from a Procgen-produced artifact.
/// This is deliberately a floor fact, not a generator or an Engine spatial mirror.
/// </summary>
public sealed class FloorState
{
    internal FloorState(
        string floorId,
        FloorBounds bounds,
        IReadOnlyList<FloorCell> walkableCells,
        IReadOnlyList<FloorRegion> regions,
        IReadOnlyList<FloorFeature> features,
        IReadOnlyList<FloorPortal> portals,
        IReadOnlyList<FloorScenePlacement> scenePlacements,
        FloorProvenance provenance)
    {
        FloorId = floorId;
        Bounds = bounds;
        WalkableCells = Copy(walkableCells);
        Regions = Copy(regions);
        Features = Copy(features);
        Portals = Copy(portals);
        ScenePlacements = Copy(scenePlacements);
        Provenance = provenance;
    }

    public string FloorId { get; }
    public FloorBounds Bounds { get; }
    public IReadOnlyList<FloorCell> WalkableCells { get; }
    public IReadOnlyList<FloorRegion> Regions { get; }
    public IReadOnlyList<FloorFeature> Features { get; }
    public IReadOnlyList<FloorPortal> Portals { get; }
    public IReadOnlyList<FloorScenePlacement> ScenePlacements { get; }
    public FloorProvenance Provenance { get; }

    private static IReadOnlyList<T> Copy<T>(IReadOnlyList<T> source) =>
        new ReadOnlyCollection<T>(source.ToArray());
}

public sealed class AdmittedFloorStore
{
    private FloorState? _current;

    public FloorState? Current => _current;

    /// <summary>Publishes only a fully admitted candidate; failed artifacts leave the current floor intact.</summary>
    public FloorAdmissionResult TryReplace(ReadOnlySpan<byte> artifactBytes, FloorAdmissionProfile profile)
    {
        FloorAdmissionResult result = FloorArtifactAdmission.Admit(artifactBytes, profile);
        if (result.Floor is not null)
        {
            _current = result.Floor;
        }

        return result;
    }
}

public sealed record FloorBounds(int MinX, int MinY, int Width, int Height);
public sealed record FloorCell(int X, int Y);
public sealed record FloorRegion(string Id, string SourcePieceId, string Kind, IReadOnlyList<FloorCell> Cells, IReadOnlyList<string> Tags);
public sealed record FloorFeature(string Id, string SourceNodeId, string Kind, FloorCell Cell);
public sealed record FloorPortal(string Id, string SourceEdgeId, IReadOnlyList<FloorCell> Cells, string Orientation, string Traversal, string? RequiredItem);
public sealed record FloorScenePlacement(string Id, string SourceInstanceId, string SourceSocketId, FloorCell Cell, string Facing, IReadOnlyList<string> Tags, FloorSceneContent Content);
public sealed record FloorSceneContent(string Kind, string? ContentId, string? ColorRgb, int? IntensityMilli, int? RangeCells);

public sealed record FloorProvenance(
    int SchemaVersion,
    string RustyProcgenRevision,
    ulong Seed,
    ulong RuleSeed,
    ulong GeometrySeed,
    ulong RealizationSeed,
    string IntentHash,
    string GeometryPolicyHash,
    string CatalogHash,
    string CatalogPolicyHash,
    string CandidateHash,
    string SourceGeometryHash,
    string SourcePiecePlanHash,
    string ProcgenResultHash,
    string AcceptedGeometryHash,
    string AcceptedPlacementHash,
    int SelectedAttempt);
