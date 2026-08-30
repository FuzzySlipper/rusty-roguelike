using System.Buffers;
using System.Text.Json;
using System.Text.Json.Serialization;
using Rusty.Engine;
using Rusty.Engine.Persistence;
using RustyRoguelike.Product.Floors;
using RustyRoguelike.Product.Integration;
using RustyRoguelike.Product.Session;

namespace RustyRoguelike.Product.Saves;

/// <summary>Closed save contract for this product. Engine owns durable blob storage, not save meaning.</summary>
internal sealed record RoguelikeSave(
    uint SchemaVersion,
    string RulesFingerprint,
    FloorProvenance FloorProvenance,
    ContentSha256 FloorContentHash,
    SessionCheckpoint Session);

internal sealed class RoguelikeSaveStore : IDisposable
{
    internal const string SaveSlot = "starter-session";
    private readonly ProductStateStore<RoguelikeSave> _store;

    internal RoguelikeSaveStore(IEngineContext engine) =>
        _store = new ProductStateStore<RoguelikeSave>(engine, "rusty-roguelike", new RoguelikeSaveCodec());

    internal PersistenceSaveReceipt Save(GameSession session, FloorEngineProjection floor) =>
        _store.Save(SaveSlot, new RoguelikeSave(
            RoguelikeSaveCodec.CurrentSchema,
            RoguelikeSaveCodec.StarterRulesFingerprint,
            floor.Floor.Provenance,
            floor.Content.Sha256,
            session.Capture()));

    internal ProductStateLoad<RoguelikeSave> Load() => _store.Load(SaveSlot);

    public void Dispose() => _store.Dispose();
}

internal sealed class RoguelikeSaveCodec : IProductStateCodec<RoguelikeSave>
{
    internal const uint CurrentSchema = 2;
    internal const string StarterRulesFingerprint = "rusty-roguelike.starter-rules.csharp-v2-initiative";
    public uint SchemaVersion => CurrentSchema;

    public void Encode(in RoguelikeSave state, IBufferWriter<byte> destination)
    {
        ArgumentNullException.ThrowIfNull(destination);
        if (state.SchemaVersion != CurrentSchema || state.RulesFingerprint != StarterRulesFingerprint)
        {
            throw new InvalidOperationException("save-schema-or-rules-fingerprint-invalid");
        }
        destination.Write(JsonSerializer.SerializeToUtf8Bytes(state, RoguelikeSaveJsonContext.Default.RoguelikeSave));
    }

    public RoguelikeSave Decode(ReadOnlySpan<byte> payload)
    {
        RoguelikeSave save = JsonSerializer.Deserialize(payload, RoguelikeSaveJsonContext.Default.RoguelikeSave)
            ?? throw new InvalidOperationException("save-decode-null");
        if (save.SchemaVersion != CurrentSchema || save.RulesFingerprint != StarterRulesFingerprint)
        {
            throw new InvalidOperationException("save-schema-or-rules-fingerprint-invalid");
        }
        return save;
    }
}

[JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase, UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow)]
[JsonSerializable(typeof(RoguelikeSave))]
internal sealed partial class RoguelikeSaveJsonContext : JsonSerializerContext;
