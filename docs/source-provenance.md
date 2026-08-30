# Source provenance

## Runtime provider

- **Rusty Engine:** the product consumes the adjacent checkout at
  `../rusty-engine` exactly as it stands. Both C# projects reference only its
  public `csharp/Rusty.Engine/Rusty.Engine.csproj`; the NativeAOT project also
  references the public `Rusty.Engine.ProductGenerator` analyzer. The checkout
  is not pulled, synchronized, or modified by this repository. The current
  migration consultation used Engine revision
  `94ba020e37ee1409c34f4683f96c09a144042e14`.

## Starter-floor artifact

- **Rusty Procgen:** `https://github.com/FuzzySlipper/rusty-procgen` at
  reviewed revision `722e2c479bdf88ab39b66d2d33ab466b698ec7df`.
- `dependency-sources.json` carries that exact source selection.
- `src/RustyRoguelike.NativeProduct/DevelopmentHost/content/floors/starter-floor.5201.procgen.json`
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
