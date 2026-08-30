using System.Security.Cryptography;
using System.Text;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace RustyRoguelike.Product.Floors;

/// <summary>Strict boundary for offline Procgen output. It never invokes a generator.</summary>
public static partial class FloorArtifactAdmission
{
    public static FloorAdmissionResult Admit(ReadOnlySpan<byte> artifactBytes, FloorAdmissionProfile profile)
    {
        ArgumentNullException.ThrowIfNull(profile);
        if (artifactBytes.IsEmpty)
        {
            return Reject("floor_artifact_empty", "The artifact contains no bytes.");
        }

        try
        {
            byte[] artifactPayload = artifactBytes.ToArray();
            using JsonDocument document = JsonDocument.Parse(artifactPayload);
            RejectDuplicateProperties(document.RootElement);
            if (document.RootElement.ValueKind != JsonValueKind.Object || !document.RootElement.TryGetProperty("floor", out JsonElement floorElement))
            {
                return Reject("floor_artifact_shape_invalid", "The artifact must contain a floor object.");
            }

            RawArtifact artifact = JsonSerializer.Deserialize(artifactPayload, FloorArtifactJsonContext.Default.RawArtifact)
                ?? throw new FloorArtifactException("floor_artifact_decode_failed", "The artifact decoded to null.");
            if (artifact.SchemaVersion != 1 || artifact.ArtifactId != profile.ArtifactId)
            {
                return Reject("floor_artifact_identity_mismatch", "The artifact schema or identity is not admitted by this profile.");
            }

            string expectedHash = "sha256:" + Convert.ToHexString(SHA256.HashData(Encoding.UTF8.GetBytes(floorElement.GetRawText()))).ToLowerInvariant();
            if (!CryptographicOperations.FixedTimeEquals(Encoding.ASCII.GetBytes(artifact.ArtifactHash), Encoding.ASCII.GetBytes(expectedHash)))
            {
                return Reject("floor_artifact_hash_mismatch", "The floor payload does not match its declared artifact hash.");
            }

            FloorState floor = Validate(artifact.Floor, profile);
            return new FloorAdmissionResult(floor, null, null);
        }
        catch (FloorArtifactException exception)
        {
            return Reject(exception.Code, exception.Message);
        }
        catch (JsonException exception)
        {
            return Reject("floor_artifact_decode_failed", exception.Message);
        }
    }

    private static FloorState Validate(RawFloor raw, FloorAdmissionProfile profile)
    {
        Require(raw.SchemaVersion == 2, "floor_schema_unsupported", "Only floor schema 2 is admitted.");
        RequireId(raw.FloorId, "floor_id_invalid");
        FloorProvenance provenance = ToProvenance(raw.Provenance);
        Require(provenance == profile.ExpectedProvenance, "floor_provenance_mismatch", "The complete Procgen provenance does not match this profile.");
        Require(raw.Bounds.Width > 0 && raw.Bounds.Height > 0 && raw.Bounds.Width <= profile.MaxWidth && raw.Bounds.Height <= profile.MaxHeight,
            "floor_bounds_invalid", "Floor bounds exceed the profile.");
        Require(raw.WalkableCells.Length > 0 && raw.WalkableCells.Length <= profile.MaxWalkableCells,
            "floor_cells_invalid", "Walkable cell count exceeds the profile.");
        Require(raw.Regions.Length > 0 && raw.Regions.Length <= profile.MaxRegions,
            "floor_regions_invalid", "Region count exceeds the profile.");
        Require(raw.ScenePlacements.Length <= profile.MaxScenePlacements, "floor_scene_count_invalid", "Scene placement count exceeds the profile.");

        FloorBounds bounds = new(raw.Bounds.MinX, raw.Bounds.MinY, raw.Bounds.Width, raw.Bounds.Height);
        FloorCell[] cells = raw.WalkableCells.Select(ToCell).ToArray();
        HashSet<FloorCell> walkable = new();
        foreach (FloorCell cell in cells)
        {
            Require(IsInside(bounds, cell), "floor_cell_out_of_bounds", "A walkable cell lies outside bounds.");
            Require(walkable.Add(cell), "floor_cell_duplicate", "Walkable cells must be unique.");
        }
        RequireConnected(walkable);

        FloorRegion[] regions = raw.Regions.Select(region => ValidateRegion(region, walkable)).ToArray();
        RequireUnique(regions.Select(region => region.Id), "floor_region_duplicate");
        FloorFeature[] features = raw.Features.Select(feature => ValidateFeature(feature, walkable)).ToArray();
        ValidateFeatures(features, profile);
        FloorPortal[] portals = raw.Portals.Select(portal => ValidatePortal(portal, walkable)).ToArray();
        ValidatePortals(portals, profile);
        FloorScenePlacement[] placements = raw.ScenePlacements.Select(placement => ValidateScenePlacement(placement, walkable, profile)).ToArray();
        ValidateScenePairs(placements, profile);

        return new FloorState(raw.FloorId, bounds, cells, regions, features, portals, placements, provenance);
    }

    private static FloorRegion ValidateRegion(RawRegion raw, HashSet<FloorCell> walkable)
    {
        RequireId(raw.Id, "floor_region_invalid");
        RequireId(raw.SourcePieceId, "floor_region_invalid");
        Require(raw.Kind is "room" or "threshold" or "key", "floor_region_kind_invalid", "An unsupported region kind was supplied.");
        FloorCell[] cells = raw.Cells.Select(ToCell).ToArray();
        Require(cells.Length > 0 && cells.All(walkable.Contains), "floor_region_cells_invalid", "Region cells must be walkable.");
        RequireUnique(cells, "floor_region_cell_duplicate");
        RequireUnique(raw.Tags, "floor_region_tag_duplicate");
        return new FloorRegion(raw.Id, raw.SourcePieceId, raw.Kind, Array.AsReadOnly(cells), Array.AsReadOnly(raw.Tags));
    }

    private static FloorFeature ValidateFeature(RawFeature raw, HashSet<FloorCell> walkable)
    {
        RequireId(raw.Id, "floor_feature_invalid");
        RequireId(raw.SourceNodeId, "floor_feature_invalid");
        Require(raw.Kind is "entry" or "goal" or "key" or "gate", "floor_feature_kind_invalid", "An unsupported feature kind was supplied.");
        FloorCell cell = ToCell(raw.Cell);
        Require(walkable.Contains(cell), "floor_feature_cell_invalid", "Feature cells must be walkable.");
        return new FloorFeature(raw.Id, raw.SourceNodeId, raw.Kind, cell);
    }

    private static void ValidateFeatures(FloorFeature[] features, FloorAdmissionProfile profile)
    {
        Require(features.Length == profile.RequiredFeatures.Count, "floor_feature_set_invalid", "Feature count does not match the profile.");
        RequireUnique(features.Select(feature => feature.Id), "floor_feature_duplicate");
        RequireUnique(features.Select(feature => feature.SourceNodeId), "floor_feature_source_duplicate");
        foreach (FloorFeature feature in features)
        {
            Require(profile.RequiredFeatures.TryGetValue(feature.SourceNodeId, out FloorFeatureRequirement? expected)
                && feature.Kind == expected.Kind && feature.Id == expected.Id,
                "floor_feature_semantics_invalid", "Feature identity or semantic kind is not admitted.");
        }
    }

    private static FloorPortal ValidatePortal(RawPortal raw, HashSet<FloorCell> walkable)
    {
        RequireId(raw.Id, "floor_portal_invalid");
        RequireId(raw.SourceEdgeId, "floor_portal_invalid");
        Require(raw.Orientation is "north" or "east" or "south" or "west", "floor_portal_orientation_invalid", "Portal orientation is unsupported.");
        Require(raw.Traversal is "open" or "locked", "floor_portal_traversal_invalid", "Portal traversal is unsupported.");
        Require((raw.Traversal == "locked") == (raw.RequiredItem is not null), "floor_portal_key_invalid", "Locked portals require exactly one key item.");
        FloorCell[] cells = raw.Cells.Select(ToCell).ToArray();
        Require(cells.Length > 0 && cells.All(walkable.Contains), "floor_portal_cells_invalid", "Portal cells must be walkable.");
        RequireUnique(cells, "floor_portal_cell_duplicate");
        return new FloorPortal(raw.Id, raw.SourceEdgeId, Array.AsReadOnly(cells), raw.Orientation, raw.Traversal, raw.RequiredItem);
    }

    private static void ValidatePortals(FloorPortal[] portals, FloorAdmissionProfile profile)
    {
        Require(portals.Length == profile.RequiredPortals.Count, "floor_portal_set_invalid", "Portal count does not match the profile.");
        RequireUnique(portals.Select(portal => portal.Id), "floor_portal_duplicate");
        RequireUnique(portals.Select(portal => portal.SourceEdgeId), "floor_portal_source_duplicate");
        foreach (FloorPortal portal in portals)
        {
            Require(profile.RequiredPortals.TryGetValue(portal.SourceEdgeId, out FloorPortalRequirement? expected)
                && portal.Traversal == expected.Traversal && portal.RequiredItem == expected.RequiredItem,
                "floor_portal_semantics_invalid", "Portal semantics are not admitted by this profile.");
        }
    }

    private static FloorScenePlacement ValidateScenePlacement(RawScenePlacement raw, HashSet<FloorCell> walkable, FloorAdmissionProfile profile)
    {
        RequireId(raw.Id, "floor_scene_invalid");
        RequireId(raw.SourceInstanceId, "floor_scene_invalid");
        if (!profile.SceneSocketBindings.TryGetValue(raw.SourceSocketId, out SceneSocketBinding? binding))
        {
            throw new FloorArtifactException("floor_scene_socket_unknown", "The scene socket is not in this profile.");
        }
        FloorCell cell = ToCell(raw.Cell);
        Require(walkable.Contains(cell), "floor_scene_cell_invalid", "Scene placement cells must be walkable.");
        Require(raw.Facing is "north" or "east" or "south" or "west", "floor_scene_facing_invalid", "Scene facing is unsupported.");
        RequireUnique(raw.Tags, "floor_scene_tag_duplicate");
        RawSceneContent content = raw.Content;
        Require(content.Kind == binding.ContentKind
            && content.ContentId == binding.ContentId
            && content.ColorRgb == binding.ColorRgb
            && content.IntensityMilli == binding.IntensityMilli
            && content.RangeCells == binding.RangeCells,
            "floor_scene_content_invalid", "Scene content does not match the admitted socket mapping.");
        return new FloorScenePlacement(raw.Id, raw.SourceInstanceId, raw.SourceSocketId, cell, raw.Facing, Array.AsReadOnly(raw.Tags),
            new FloorSceneContent(content.Kind, content.ContentId, content.ColorRgb, content.IntensityMilli, content.RangeCells));
    }

    private static void ValidateScenePairs(FloorScenePlacement[] placements, FloorAdmissionProfile profile)
    {
        RequireUnique(placements.Select(placement => placement.Id), "floor_scene_duplicate");
        HashSet<(string Instance, string Socket, FloorCell Cell)> observed = placements
            .Select(placement => (placement.SourceInstanceId, placement.SourceSocketId, placement.Cell)).ToHashSet();
        foreach (FloorScenePlacement placement in placements)
        {
            SceneSocketBinding binding = profile.SceneSocketBindings[placement.SourceSocketId];
            Require(observed.Contains((placement.SourceInstanceId, binding.PairedSocketId, placement.Cell)),
                "floor_scene_pairing_invalid", "Every admitted scene socket needs its paired prop/light at the same cell.");
        }
    }

    private static FloorProvenance ToProvenance(RawProvenance raw) => new(raw.SchemaVersion, raw.RustyProcgenRevision, raw.Seed, raw.RuleSeed, raw.GeometrySeed,
        raw.RealizationSeed, raw.IntentHash, raw.GeometryPolicyHash, raw.CatalogHash, raw.CatalogPolicyHash, raw.CandidateHash, raw.SourceGeometryHash,
        raw.SourcePiecePlanHash, raw.ProcgenResultHash, raw.AcceptedGeometryHash, raw.AcceptedPlacementHash, raw.SelectedAttempt);
    private static FloorCell ToCell(RawCell raw) => new(raw.X, raw.Y);
    private static bool IsInside(FloorBounds bounds, FloorCell cell) => cell.X >= bounds.MinX && cell.Y >= bounds.MinY
        && cell.X < (long)bounds.MinX + bounds.Width && cell.Y < (long)bounds.MinY + bounds.Height;

    private static void RequireConnected(HashSet<FloorCell> cells)
    {
        Queue<FloorCell> pending = new();
        HashSet<FloorCell> visited = new();
        pending.Enqueue(cells.First());
        while (pending.TryDequeue(out FloorCell? cell))
        {
            if (cell is null) continue;
            if (!visited.Add(cell)) continue;
            foreach (FloorCell neighbor in new[] { new FloorCell(cell.X + 1, cell.Y), new FloorCell(cell.X - 1, cell.Y), new FloorCell(cell.X, cell.Y + 1), new FloorCell(cell.X, cell.Y - 1) })
                if (cells.Contains(neighbor) && !visited.Contains(neighbor)) pending.Enqueue(neighbor);
        }
        Require(visited.Count == cells.Count, "floor_topology_disconnected", "Walkable cells must be four-way connected.");
    }

    private static void RejectDuplicateProperties(JsonElement element)
    {
        if (element.ValueKind == JsonValueKind.Object)
        {
            HashSet<string> names = new(StringComparer.Ordinal);
            foreach (JsonProperty property in element.EnumerateObject())
            {
                Require(names.Add(property.Name), "floor_artifact_duplicate_property", $"JSON property {property.Name} is repeated.");
                RejectDuplicateProperties(property.Value);
            }
        }
        else if (element.ValueKind == JsonValueKind.Array)
        {
            foreach (JsonElement item in element.EnumerateArray()) RejectDuplicateProperties(item);
        }
    }

    private static void RequireUnique<T>(IEnumerable<T> values, string code) where T : notnull =>
        Require(values.Distinct().Count() == values.Count(), code, "Artifact values that define identity must be unique.");
    private static void RequireId(string value, string code) => Require(!string.IsNullOrWhiteSpace(value) && value.Length <= 192, code, "An identifier is empty or too long.");
    private static void Require(bool condition, string code, string message) { if (!condition) throw new FloorArtifactException(code, message); }
    private static FloorAdmissionResult Reject(string code, string detail) => new(null, code, detail);

    private sealed class FloorArtifactException(string code, string message) : Exception(message) { public string Code { get; } = code; }

    private sealed class RawArtifact { public required int SchemaVersion { get; init; } public required string ArtifactId { get; init; } public required string ArtifactHash { get; init; } public required RawFloor Floor { get; init; } }
    private sealed class RawFloor { public required int SchemaVersion { get; init; } public required string FloorId { get; init; } public required RawBounds Bounds { get; init; } public required RawCell[] WalkableCells { get; init; } public required RawRegion[] Regions { get; init; } public required RawFeature[] Features { get; init; } public required RawPortal[] Portals { get; init; } public required RawScenePlacement[] ScenePlacements { get; init; } public required RawProvenance Provenance { get; init; } }
    private sealed class RawBounds { public required int MinX { get; init; } public required int MinY { get; init; } public required int Width { get; init; } public required int Height { get; init; } }
    private sealed class RawCell { public required int X { get; init; } public required int Y { get; init; } }
    private sealed class RawRegion { public required string Id { get; init; } public required string SourcePieceId { get; init; } public required string Kind { get; init; } public required RawCell[] Cells { get; init; } public required string[] Tags { get; init; } }
    private sealed class RawFeature { public required string Id { get; init; } public required string SourceNodeId { get; init; } public required string Kind { get; init; } public required RawCell Cell { get; init; } }
    private sealed class RawPortal { public required string Id { get; init; } public required string SourceEdgeId { get; init; } public required RawCell[] Cells { get; init; } public required string Orientation { get; init; } public required string Traversal { get; init; } public required string? RequiredItem { get; init; } }
    private sealed class RawScenePlacement { public required string Id { get; init; } public required string SourceInstanceId { get; init; } public required string SourceSocketId { get; init; } public required RawCell Cell { get; init; } public required string Facing { get; init; } public required string[] Tags { get; init; } public required RawSceneContent Content { get; init; } }
    private sealed class RawSceneContent { public required string Kind { get; init; } public string? ContentId { get; init; } public string? ColorRgb { get; init; } public int? IntensityMilli { get; init; } public int? RangeCells { get; init; } }
    private sealed class RawProvenance { public required int SchemaVersion { get; init; } public required string RustyProcgenRevision { get; init; } public required ulong Seed { get; init; } public required ulong RuleSeed { get; init; } public required ulong GeometrySeed { get; init; } public required ulong RealizationSeed { get; init; } public required string IntentHash { get; init; } public required string GeometryPolicyHash { get; init; } public required string CatalogHash { get; init; } public required string CatalogPolicyHash { get; init; } public required string CandidateHash { get; init; } public required string SourceGeometryHash { get; init; } public required string SourcePiecePlanHash { get; init; } public required string ProcgenResultHash { get; init; } public required string AcceptedGeometryHash { get; init; } public required string AcceptedPlacementHash { get; init; } public required int SelectedAttempt { get; init; } }

    [JsonSourceGenerationOptions(PropertyNamingPolicy = JsonKnownNamingPolicy.CamelCase, UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow)]
    [JsonSerializable(typeof(RawArtifact))]
    private sealed partial class FloorArtifactJsonContext : JsonSerializerContext;
}

public sealed record FloorAdmissionResult(FloorState? Floor, string? RejectionCode, string? RejectionDetail)
{
    public bool Accepted => Floor is not null;
}
