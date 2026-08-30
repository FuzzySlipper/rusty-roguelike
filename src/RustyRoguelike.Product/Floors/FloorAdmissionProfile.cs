namespace RustyRoguelike.Product.Floors;

/// <summary>
/// Product policy for one catalog/artifact family. New floor catalogs add a profile,
/// rather than changing session code or reimplementing Procgen.
/// </summary>
public sealed record FloorAdmissionProfile(
    string ArtifactId,
    string ExpectedArtifactSha256,
    FloorProvenance ExpectedProvenance,
    int MaxWidth,
    int MaxHeight,
    int MaxWalkableCells,
    int MaxRegions,
    int MaxScenePlacements,
    IReadOnlyDictionary<string, FloorFeatureRequirement> RequiredFeatures,
    IReadOnlyDictionary<string, FloorPortalRequirement> RequiredPortals,
    IReadOnlyDictionary<string, SceneSocketBinding> SceneSocketBindings)
{
    public static FloorAdmissionProfile Starter { get; } = new(
        ArtifactId: "rusty-roguelike.starter-floor.5201",
        // Hash the exact committed envelope bytes, including its final newline. This is the
        // trust anchor; the envelope's payload hash remains a separate corruption check.
        ExpectedArtifactSha256: "sha256:58b0e5ab3971c10a17f30c44fb6086f3b90d0f2c2f54b44ef06e0091d1d109e2",
        ExpectedProvenance: new FloorProvenance(
            SchemaVersion: 1,
            RustyProcgenRevision: "722e2c479bdf88ab39b66d2d33ab466b698ec7df",
            Seed: 5201,
            RuleSeed: 5202,
            GeometrySeed: 5203,
            RealizationSeed: 5204,
            IntentHash: "fnv1a64:0f208a8e4b38637f",
            GeometryPolicyHash: "fnv1a64:31ab386b10e4d95d",
            CatalogHash: "fnv1a64:b7a79772466cc01b",
            CatalogPolicyHash: "fnv1a64:09dbc4e8dfb15339",
            CandidateHash: "fnv1a64:4c745813a166fdcc",
            SourceGeometryHash: "fnv1a64:0128bfcbebafbf6a",
            SourcePiecePlanHash: "fnv1a64:65407acd347e47a5",
            ProcgenResultHash: "fnv1a64:5aa8d1cbd83cb5a2",
            AcceptedGeometryHash: "fnv1a64:2edb50e8713efa02",
            AcceptedPlacementHash: "fnv1a64:62efed7299c1637d",
            SelectedAttempt: 1),
        MaxWidth: 128,
        MaxHeight: 128,
        MaxWalkableCells: 4096,
        MaxRegions: 32,
        MaxScenePlacements: 64,
        RequiredFeatures: new Dictionary<string, FloorFeatureRequirement>(StringComparer.Ordinal)
        {
            ["start"] = new("entry", "feature.start"),
            ["goal"] = new("goal", "feature.goal"),
            ["key.gate_1"] = new("key", "feature.key.gate_1"),
            ["gate.locked_1"] = new("gate", "feature.gate.locked_1"),
        },
        RequiredPortals: new Dictionary<string, FloorPortalRequirement>(StringComparer.Ordinal)
        {
            ["edge.start.gate_1"] = new("open", null),
            ["edge.start.key_1"] = new("open", null),
            ["edge.key_1.gate_1"] = new("open", null),
            ["edge.gate_1.goal"] = new("locked", "item.gate_key_1"),
        },
        SceneSocketBindings: StarterSockets());

    private static IReadOnlyDictionary<string, SceneSocketBinding> StarterSockets() =>
        new Dictionary<string, SceneSocketBinding>(StringComparer.Ordinal)
        {
            ["torch.wall.prop"] = SceneSocketBinding.Prop("torch.wall.light", "prop.torch.medieval"),
            ["torch.wall.light"] = SceneSocketBinding.WarmTorchLight("torch.wall.prop"),
            ["torch.west.prop"] = SceneSocketBinding.Prop("torch.west.light", "prop.torch.medieval"),
            ["torch.west.light"] = SceneSocketBinding.WarmTorchLight("torch.west.prop"),
            ["torch.east.prop"] = SceneSocketBinding.Prop("torch.east.light", "prop.torch.medieval"),
            ["torch.east.light"] = SceneSocketBinding.WarmTorchLight("torch.east.prop"),
        };
}

public sealed record FloorFeatureRequirement(string Kind, string Id);
public sealed record FloorPortalRequirement(string Traversal, string? RequiredItem);
public sealed record SceneSocketBinding(
    string PairedSocketId,
    string ContentKind,
    string? ContentId,
    string? ColorRgb,
    int? IntensityMilli,
    int? RangeCells)
{
    public static SceneSocketBinding Prop(string pairedSocketId, string contentId) =>
        new(pairedSocketId, "prop", contentId, null, null, null);

    public static SceneSocketBinding WarmTorchLight(string pairedSocketId) =>
        new(pairedSocketId, "point_light", null, "#ffb45f", 2500, 6);
}
