# Source provenance

## Runtime provider

- **Rusty Engine:** the product references immutable package
  `Rusty.Engine` `0.1.0-dev.cbf35130d06c` from `.runtime/sdk-feed`. The matching
  installed runtime pack is `.runtime/runtime-pack-cbf35130d06c`, built from Engine
  revision `cbf35130d06ceec72b1d80ac8e28451453cccde2`. Both carry ABI protocol
  `1`, fingerprint
  `9b0093d77fc58cbcb18197f743e58e3243abdf3115ab97a62a5a120833d10fbd`, and
  build identity `rusty-engine-sdk/v1`. The SDK generates product composition
  below ignored `obj`; no Engine checkout, generated binding, native bootstrap,
  host, or browser asset is tracked here. Source use is an explicit contributor
  override only.

## Starter-floor artifact

- **Rusty Procgen:** `https://github.com/FuzzySlipper/rusty-procgen` at
  reviewed revision `722e2c479bdf88ab39b66d2d33ab466b698ec7df`.
- `dependency-sources.json` carries that exact source selection.
- `content/floors/starter-floor.5201.procgen.json`
  is a committed artifact derived offline from the reviewed public Procgen
  surface. Its full seed chain, selected attempt, source hashes, required
  features, portals, and scene sockets are pinned by
  `FloorAdmissionProfile.Starter` before it is accepted. The profile also pins
  the exact full-envelope SHA-256
  `58b0e5ab3971c10a17f30c44fb6086f3b90d0f2c2f54b44ef06e0091d1d109e2`;
  the artifact's internal floor-payload hash is only an additional integrity
  check and is never its trust anchor.

The C# product does not link Procgen, shell out to it, copy its algorithms, or
claim live generation. Its admission and Engine projection code are original
product code. A safe runtime C# generation capability is upstream work.

## Donor consultation

The retired Rust/TypeScript implementation at
`909dd4521e57ddebd277522b7e887534a34fa23d` was consulted as semantic evidence
for collapsed-party policy, named actor vocabulary, receipts, and provenance
facts. It is not a runtime/build/test dependency and none of its implementation
lanes remain active. The resulting C# layout is documented in
`docs/csharp-migration-map.md`.
