using Rusty.Engine;
using RustyRoguelike.Product.Integration;

namespace RustyRoguelike.Product.Presentation;

/// <summary>Observational Engine integration facts kept separate from the game session view.</summary>
internal sealed class EngineIntegrationProjection : IDisposable
{
    private const string StreamName = "rusty-roguelike.engine";
    private const string ContractName = "rusty-roguelike.engine.v1";
    private readonly IUiService _ui;
    private readonly UiStream _stream;
    private ulong _sequence;

    internal EngineIntegrationProjection(IUiService ui)
    {
        _ui = ui;
        _stream = ui.OpenStream(new UiStreamRequest(StreamName, ContractName));
    }

    internal void Publish(FloorEngineReadout floor, SaveOperationReadout save)
    {
        LifecycleValueBuilder value = new();
        uint source = value.Object(
            ("contentPath", value.String(floor.ContentPath)),
            ("floorId", value.String(floor.FloorId)),
            ("sourceRevision", value.Number(floor.SourceRevision)),
            ("collisionRevision", value.Number(floor.CollisionRevision)),
            ("navigationRevision", value.Number(floor.NavigationRevision)),
            ("meshRevision", value.Number(floor.MeshRevision)),
            ("authorityHash", value.String($"0x{floor.AuthorityHash:x16}")),
            ("solidVoxelCount", value.Number(floor.SolidVoxelCount)),
            ("walkableCellCount", value.Number(floor.WalkableCellCount)),
            ("navigationProjectionHash", value.String($"0x{floor.NavigationProjectionHash:x16}")),
            ("sceneProjectionCount", value.Number(floor.SceneProjectionCount)),
            ("lightCount", value.Number(floor.LightCount)));
        uint persistence = value.Object(
            ("lastOperation", value.String(save.Operation)),
            ("status", value.String(save.Status)),
            ("storeRevision", value.Number(save.StoreRevision)),
            ("detail", value.String(save.Detail)));
        uint root = value.Object(
            ("floor", source),
            ("persistence", persistence),
            ("ownership", value.String("engine-content-spatial-voxel-ui-persistence; product-floor-rules-save")));
        _ui.PublishProjection(new UiProjection(_stream, checked(++_sequence), value.Build(root)));
    }

    public void Dispose() => _stream.Dispose();
}

internal sealed record SaveOperationReadout(string Operation, string Status, ulong StoreRevision, string Detail)
{
    internal static SaveOperationReadout None { get; } = new("none", "not-requested", 0, "no save command has been admitted");
}
