using System.Numerics;
using Rusty.Engine;
using RustyRoguelike.Product.Floors;
using RustyRoguelike.Product.Rules;

namespace RustyRoguelike.Product.Integration;

/// <summary>
/// Product composition for one admitted floor. Geometry meaning remains in the floor
/// artifact; Engine owns the resident voxels, collision, navigation and renderer scene.
/// </summary>
internal sealed class FloorEngineProjection : IDisposable
{
    internal const string StarterFloorContentPath = "floors/starter-floor.5201.procgen.json";

    private static readonly FloorProjectionTuning Tuning = FloorProjectionTuning.Starter;
    private readonly IEngineContext _engine;
    private readonly SpatialSession _spatial;
    private readonly Material _floorMaterial;
    private readonly VoxelScenePresentation _scene;
    private readonly List<Light> _lights;
    private bool _disposed;

    private FloorEngineProjection(
        IEngineContext engine,
        FloorState floor,
        ContentReferenceInfo content,
        SpatialSession spatial,
        Material floorMaterial,
        VoxelScenePresentation scene,
        IReadOnlyList<Light> lights,
        FloorEngineReadout readout)
    {
        _engine = engine;
        Floor = floor;
        Content = content;
        _spatial = spatial;
        _floorMaterial = floorMaterial;
        _scene = scene;
        _lights = lights.ToList();
        Readout = readout;
    }

    internal FloorState Floor { get; }
    internal ContentReferenceInfo Content { get; }
    internal FloorEngineReadout Readout { get; private set; }

    internal static FloorEngineProjection Create(IEngineContext engine)
    {
        ArgumentNullException.ThrowIfNull(engine);
        (FloorState floor, ContentReferenceInfo content) = LoadAndAdmitFloor(engine.Content);
        SpatialSession spatial = engine.Spatial.CreateSession(new SpatialSessionConfig(
            Tuning.CollisionVoxelSize,
            Tuning.CollisionChunkSize,
            Tuning.VoxelSurfaceMode));
        Material? material = null;
        VoxelScenePresentation? scene = null;
        List<Light>? lights = null;
        try
        {
            material = engine.Appearance.CreateMaterial(Tuning.FloorMaterial);
            VoxelSceneReadout before = engine.Voxel.ReadScene(new VoxelSceneReadRequest(spatial));
            VoxelEdit[] edits = floor.WalkableCells
                .Select(cell => new VoxelEdit(VoxelEditKind.Set, new VoxelAddress(cell.X, Tuning.FloorVoxelY, cell.Y), Tuning.FloorMaterialSlot))
                .ToArray();
            VoxelEditReceipt voxel = engine.Voxel.ApplyEdits(new VoxelEditTransaction(spatial, before.SourceRevision, edits));
            NavigationReplaceReceipt navigation = engine.Spatial.ReplaceNavigation(new NavigationReplaceRequest(
                spatial,
                new PlanarNavConfig(Tuning.NavigationGridId, Tuning.CollisionVoxelSize, Tuning.CollisionChunkSize, Tuning.MaximumNavigationStepCells),
                floor.WalkableCells.Select(cell => new PlanarNavCell(cell.X, Tuning.NavigationPlaneY, cell.Y)).ToArray()));
            scene = engine.VoxelScenePresentation.ProjectScene(new ProjectVoxelSceneRequest(
                spatial,
                new VoxelSceneMaterialBinding[] { new(Tuning.FloorMaterialSlot, material) }));
            lights = ProjectLights(engine.Appearance, floor);
            SpatialProjectionReadout projection = engine.Spatial.ReadProjection(new SpatialProjectionReadRequest(spatial));
            VoxelSceneReadout after = engine.Voxel.ReadScene(new VoxelSceneReadRequest(spatial));
            NavigationProjectionReadout navigationReadout = engine.Spatial.ReadNavigationProjection(new NavigationProjectionReadRequest(spatial));
            return new FloorEngineProjection(engine, floor, content, spatial, material, scene, lights,
                new FloorEngineReadout(content.Path, floor.FloorId, voxel.AcceptedRevision, voxel.CollisionRevision, voxel.NavigationRevision,
                    voxel.MeshRevision, projection.AuthorityHash, after.SolidVoxelCount, navigation.WalkableCellCount,
                    navigation.NavigationRevision, navigationReadout.ProjectionHash, 1, lights.Count));
        }
        catch
        {
            foreach (Light light in lights ?? []) light.Dispose();
            scene?.Dispose();
            material?.Dispose();
            spatial.Dispose();
            throw;
        }
    }

    internal bool ProposePartyStep(GridCell from, GridCell destination)
    {
        NavigationStepReceipt step = _engine.Spatial.ProposeNavigationStep(new NavigationStepRequest(
            _spatial,
            new Vector3(from.X, Tuning.PartyHeight, from.Y),
            new Vector3(destination.X, Tuning.PartyHeight, destination.Y),
            Tuning.MaximumPartyStepUnits,
            Tuning.MaximumNavigationVisited));
        return step.Outcome == NavigationPathOutcome.Reached && step.NextPathCell == new PlanarNavCell(destination.X, Tuning.NavigationPlaneY, destination.Y);
    }

    internal void RefreshReadout()
    {
        VoxelScenePresentationReadout scene = _engine.VoxelScenePresentation.RefreshScene(_scene);
        SpatialProjectionReadout spatial = _engine.Spatial.ReadProjection(new SpatialProjectionReadRequest(_spatial));
        VoxelSceneReadout voxels = _engine.Voxel.ReadScene(new VoxelSceneReadRequest(_spatial));
        NavigationProjectionReadout navigation = _engine.Spatial.ReadNavigationProjection(new NavigationProjectionReadRequest(_spatial));
        Readout = Readout with
        {
            SourceRevision = voxels.SourceRevision,
            CollisionRevision = voxels.CollisionRevision,
            NavigationRevision = navigation.NavigationRevision,
            MeshRevision = scene.MeshRevision,
            AuthorityHash = spatial.AuthorityHash,
            SolidVoxelCount = voxels.SolidVoxelCount,
            WalkableCellCount = navigation.WalkableCellCount,
            NavigationProjectionHash = navigation.ProjectionHash,
        };
    }

    public void Dispose()
    {
        if (_disposed) return;
        foreach (Light light in _lights)
        {
            light.Dispose();
        }
        _scene.Dispose();
        _floorMaterial.Dispose();
        _spatial.Dispose();
        _disposed = true;
    }

    private static (FloorState Floor, ContentReferenceInfo Content) LoadAndAdmitFloor(IContentService content)
    {
        using ContentReference reference = content.OpenReference(new ContentOpenRequest(StarterFloorContentPath));
        ReadOnlySpan<ContentReferenceInfo> references = content.ReadReferenceInfo(reference).Span;
        if (references.Length != 1)
        {
            throw new InvalidOperationException("starter-floor-content-reference-invalid");
        }
        ContentReferenceInfo info = references[0];
        if (info.ByteLength == 0 || info.ByteLength > FloorProjectionTuning.Starter.MaximumArtifactBytes || info.ByteLength > int.MaxValue)
        {
            throw new InvalidOperationException("starter-floor-content-size-invalid");
        }

        byte[] bytes = new byte[(int)info.ByteLength];
        uint readSize = Math.Min((uint)bytes.Length, FloorProjectionTuning.Starter.MaximumContentReadBytes);
        ReadOnlyMemory<byte> read = content.ReadBytes(new ContentReadBytesRequest(reference, 0, readSize));
        if (read.Length != bytes.Length)
        {
            throw new InvalidOperationException("starter-floor-content-read-incomplete");
        }
        read.CopyTo(bytes);

        FloorAdmissionResult admitted = FloorArtifactAdmission.Admit(bytes, FloorAdmissionProfile.Starter);
        return admitted.Floor is not null
            ? (admitted.Floor, info)
            : throw new InvalidOperationException($"starter-floor-rejected:{admitted.RejectionCode}");
    }

    private static List<Light> ProjectLights(IAppearanceService appearance, FloorState floor)
    {
        var lights = new List<Light>();
        try
        {
            int index = 0;
            foreach (FloorScenePlacement placement in floor.ScenePlacements.Where(placement => placement.Content.Kind == "point_light"))
            {
                FloorSceneContent content = placement.Content;
                Vector3 color = ParseColor(content.ColorRgb);
                lights.Add(appearance.CreateLight(new LightRequest(
                    checked(Tuning.FirstLightLogicalId + (ulong)index++),
                    false,
                    0,
                    new LightDescriptor(LightKind.Point, color, content.IntensityMilli!.Value / Tuning.LightIntensityScale,
                        true, new Vector3(placement.Cell.X + Tuning.LightOffsetX, Tuning.LightHeight, placement.Cell.Y + Tuning.LightOffsetZ),
                        Vector3.UnitY, true, content.RangeCells!.Value, Tuning.LightDecay, 0, 0, LightShadowIntent.Disabled))));
            }
            return lights;
        }
        catch
        {
            foreach (Light light in lights) light.Dispose();
            throw;
        }
    }

    private static Vector3 ParseColor(string? value) => value switch
    {
        "#ffb45f" => new Vector3(1.0f, 0.7058824f, 0.37254903f),
        _ => throw new InvalidOperationException("floor-light-color-not-admitted"),
    };
}

internal sealed record FloorEngineReadout(
    string ContentPath,
    string FloorId,
    ulong SourceRevision,
    ulong CollisionRevision,
    ulong NavigationRevision,
    ulong MeshRevision,
    ulong AuthorityHash,
    ulong SolidVoxelCount,
    ulong WalkableCellCount,
    ulong NavigationProjectionRevision,
    ulong NavigationProjectionHash,
    int SceneProjectionCount,
    int LightCount);

internal sealed record FloorProjectionTuning(
    double CollisionVoxelSize,
    uint CollisionChunkSize,
    VoxelSurfaceMode VoxelSurfaceMode,
    uint FloorMaterialSlot,
    long FloorVoxelY,
    long NavigationPlaneY,
    ulong NavigationGridId,
    uint MaximumNavigationStepCells,
    uint MaximumNavigationVisited,
    float MaximumPartyStepUnits,
    float PartyHeight,
    ulong FirstLightLogicalId,
    float LightIntensityScale,
    float LightHeight,
    float LightOffsetX,
    float LightOffsetZ,
    float LightDecay,
    uint MaximumContentReadBytes,
    ulong MaximumArtifactBytes,
    MaterialRequest FloorMaterial)
{
    internal static FloorProjectionTuning Starter { get; } = new(
        CollisionVoxelSize: 1.0,
        CollisionChunkSize: 16,
        VoxelSurfaceMode: VoxelSurfaceMode.GreedyCubes,
        FloorMaterialSlot: 1,
        FloorVoxelY: 0,
        NavigationPlaneY: 0,
        NavigationGridId: 7_554,
        MaximumNavigationStepCells: 1,
        MaximumNavigationVisited: 128,
        MaximumPartyStepUnits: 1.0f,
        PartyHeight: 1.0f,
        FirstLightLogicalId: 7_554_000,
        LightIntensityScale: 1_000.0f,
        LightHeight: 1.7f,
        LightOffsetX: 0.5f,
        LightOffsetZ: 0.5f,
        LightDecay: 2.0f,
        MaximumContentReadBytes: 256 * 1024,
        MaximumArtifactBytes: 256 * 1024,
        FloorMaterial: new MaterialRequest(new Color(0.22f, 0.27f, 0.31f, 1.0f), new RenderResourceHandle(0), 0.92f,
            new Color(1, 1, 1, 1), Vector3.Zero, 0, false));
}
