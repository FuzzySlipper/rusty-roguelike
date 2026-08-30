using System.Security.Cryptography;
using System.Text;
using System.Text.Json.Nodes;

namespace RustyRoguelike.Product.Floors;

/// <summary>Small executable proof shape for the artifact boundary; it is not a test framework.</summary>
internal static class FloorArtifactAdmissionProbe
{
    internal static void Verify(ReadOnlySpan<byte> committedArtifact)
    {
        FloorAdmissionProfile trustedProfile = FloorAdmissionProfile.Starter;
        AdmittedFloorStore store = new();
        FloorAdmissionResult accepted = store.TryReplace(committedArtifact, trustedProfile);
        Require(accepted.Accepted && store.Current is not null, "The committed artifact did not admit.");
        string admittedFloorId = store.Current!.FloorId;

        JsonObject forgedEnvelope = JsonNode.Parse(committedArtifact)!.AsObject();
        forgedEnvelope["floor"]!["floorId"] = "floor.5201.forged";
        RewritePayloadDigest(forgedEnvelope);
        FloorAdmissionResult forged = store.TryReplace(Encode(forgedEnvelope), trustedProfile);
        Require(!forged.Accepted && forged.RejectionCode == "floor_artifact_trusted_hash_mismatch" && store.Current!.FloorId == admittedFloorId,
            "A recomputed envelope forgery replaced the admitted floor.");

        JsonObject malformedEnvelope = JsonNode.Parse(committedArtifact)!.AsObject();
        malformedEnvelope["floor"]!["bounds"] = null;
        RewritePayloadDigest(malformedEnvelope);
        byte[] malformedBytes = Encode(malformedEnvelope);
        FloorAdmissionProfile malformedProfile = trustedProfile with { ExpectedArtifactSha256 = Sha256Digest(malformedBytes) };
        FloorAdmissionResult malformed = store.TryReplace(malformedBytes, malformedProfile);
        Require(!malformed.Accepted && malformed.RejectionCode == "floor_artifact_null_required" && store.Current!.FloorId == admittedFloorId,
            "A required null was not rejected atomically.");
    }

    private static void RewritePayloadDigest(JsonObject envelope) =>
        envelope["artifactHash"] = Sha256Digest(Encoding.UTF8.GetBytes(envelope["floor"]!.ToJsonString()));

    private static byte[] Encode(JsonObject value) => Encoding.UTF8.GetBytes(value.ToJsonString());
    private static string Sha256Digest(ReadOnlySpan<byte> bytes) => "sha256:" + Convert.ToHexString(SHA256.HashData(bytes)).ToLowerInvariant();
    private static void Require(bool condition, string message) { if (!condition) throw new InvalidOperationException(message); }
}
