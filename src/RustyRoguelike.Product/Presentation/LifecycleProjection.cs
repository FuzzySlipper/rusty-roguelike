using System.Text;
using Rusty.Engine;

namespace RustyRoguelike.Product.Presentation;

/// <summary>Publishes Engine lifecycle facts as an inspectable product-owned readout.</summary>
internal sealed class LifecycleProjection : IDisposable
{
    private const string StreamName = "rusty-roguelike.lifecycle";
    private const string ContractName = "rusty-roguelike.lifecycle.v1";

    private readonly IUiService _ui;
    private readonly UiStream _stream;
    private ulong _sequence;

    internal LifecycleProjection(IUiService ui)
    {
        _ui = ui ?? throw new ArgumentNullException(nameof(ui));
        _stream = _ui.OpenStream(new UiStreamRequest(StreamName, ContractName));
    }

    internal void Publish(LifecycleSnapshot snapshot)
    {
        LifecycleValueBuilder value = new();
        uint root = value.Object(
            ("state", value.String(snapshot.State)),
            ("mode", value.String(snapshot.Mode)),
            ("generation", value.Number(snapshot.Generation)),
            ("controlRevision", value.Number(snapshot.ControlRevision)),
            ("simulationStep", value.Number(snapshot.SimulationStep)),
            ("admittedSteps", value.Number(snapshot.AdmittedStepCount)),
            ("inputEvents", value.Number(snapshot.InputEventCount)));
        _ui.PublishProjection(new UiProjection(_stream, checked(++_sequence), value.Build(root)));
    }

    public void Dispose() => _stream.Dispose();
}

internal readonly record struct LifecycleSnapshot(
    string State,
    string Mode,
    ulong Generation,
    ulong ControlRevision,
    ulong SimulationStep,
    uint AdmittedStepCount,
    int InputEventCount)
{
    internal static LifecycleSnapshot Created { get; } = new("created", "not-admitted", 0, 0, 0, 0, 0);
    internal static LifecycleSnapshot Started { get; } = new("started", "not-admitted", 0, 0, 0, 0, 0);
    internal static LifecycleSnapshot Paused { get; } = new("paused", "not-admitted", 0, 0, 0, 0, 0);
    internal static LifecycleSnapshot Resumed { get; } = new("resumed", "not-admitted", 0, 0, 0, 0, 0);
    internal static LifecycleSnapshot Restarted { get; } = new("restarted", "not-admitted", 0, 0, 0, 0, 0);

    internal static LifecycleSnapshot From(ProductUpdateFacts facts, int inputEventCount) => new(
        facts.LifecycleState.ToString().ToLowerInvariant(),
        facts.Mode.ToString().ToLowerInvariant(),
        facts.Generation,
        facts.ControlRevision,
        facts.SimulationStep,
        facts.AdmittedStepCount,
        inputEventCount);
}

/// <summary>Constructs copied Engine UI values; no callback or input storage is retained.</summary>
internal sealed class LifecycleValueBuilder
{
    private readonly List<StructuredValueNode> _nodes = [];
    private readonly List<uint> _edges = [];
    private readonly List<byte> _utf8 = [];

    internal uint Number(double value) => Add(StructuredValueKind.Number, numberValue: value);

    internal uint String(string value)
    {
        ArgumentNullException.ThrowIfNull(value);
        (uint offset, uint length) = Bytes(value);
        return Add(StructuredValueKind.String, textOffset: offset, textLength: length);
    }

    internal uint Object(params (string Key, uint Value)[] fields)
    {
        ArgumentNullException.ThrowIfNull(fields);
        uint firstEdge = checked((uint)_edges.Count);
        foreach ((string key, uint value) in fields)
        {
            if (value >= (uint)_nodes.Count)
            {
                throw new ArgumentOutOfRangeException(nameof(fields));
            }

            (uint offset, uint length) = Bytes(key);
            uint keyedValue = checked((uint)_nodes.Count);
            _nodes.Add(_nodes[checked((int)value)] with { KeyOffset = offset, KeyLen = length });
            _edges.Add(keyedValue);
        }

        return Add(StructuredValueKind.Object, firstEdge: firstEdge, childCount: checked((uint)fields.Length));
    }

    internal UiValue Build(uint root)
    {
        if (root >= (uint)_nodes.Count)
        {
            throw new ArgumentOutOfRangeException(nameof(root));
        }

        return new UiValue(_nodes.ToArray(), _edges.ToArray(), root, _utf8.ToArray());
    }

    private uint Add(
        StructuredValueKind kind,
        double numberValue = 0,
        uint textOffset = 0,
        uint textLength = 0,
        uint firstEdge = 0,
        uint childCount = 0)
    {
        uint index = checked((uint)_nodes.Count);
        _nodes.Add(new StructuredValueNode(kind, 0, numberValue, 0, 0, textOffset, textLength, firstEdge, childCount));
        return index;
    }

    private (uint Offset, uint Length) Bytes(string value)
    {
        byte[] bytes = Encoding.UTF8.GetBytes(value);
        uint offset = checked((uint)_utf8.Count);
        _utf8.AddRange(bytes);
        return (offset, checked((uint)bytes.Length));
    }
}
