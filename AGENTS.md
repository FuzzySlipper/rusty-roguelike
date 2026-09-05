# Rusty Roguelike agent guidance

## Repository role

Rusty Roguelike is one concrete collapsed-party first-person roguelike and a
small C# downstream example for Rusty Engine. It owns its game rules, floor
admission policy, session orchestration, save meaning, controls, and
observational projections. It is not a reusable RPG framework and must never
depend on Rusty D20.

Rusty Engine owns reusable host-neutral mechanisms. Consume its immutable
`Rusty.Engine` SDK package and the matching installed runtime pack; the SDK
generates composition below `obj`. An Engine checkout is only an explicit
contributor override (`rusty dev --engine-source` with the matching MSBuild
properties), never the normal downstream setup. Rusty Procgen remains exact
public source provenance for the committed starter-floor artifact. Do not copy
either provider's implementation into this repository.

## Architecture

Read [docs/design.md](docs/design.md) before changing authority, dependency
direction, persistence, floor admission, or the turn model. Use
[docs/agent-code-atlas.md](docs/agent-code-atlas.md) for path-level ownership
and [docs/csharp-migration-map.md](docs/csharp-migration-map.md) for the donor
dispositions that made this cutover possible.

- C# is the sole authoritative gameplay runtime.
- `RoguelikeProduct` is an Engine-owned `IEngineProduct`: it reacts to admitted
  updates and never creates a second loop or handwritten interop boundary.
- The collapsed party occupies one C#-owned grid square. One accepted party
  command consumes one activation, then admitted opposition settles in a
  deterministic visible order.
- Dormant opponents remain inactive until C# admits them. Enemy attacks target
  the party square before the product chooses an affected living member.
- Rusty Engine owns content bytes, spatial/collision/navigation, voxel-scene
  presentation, lights, input delivery, UI streams, deterministic random, and
  durable blob storage. The product owns policy and the meanings it stores.
- The development host's minimal HTML page and UI streams are observational;
  neither is a renderer or gameplay authority.

## Work and verification

Treat a dirty worktree as shared state. Preserve unrelated changes, especially
`.agent-teams/`. Commit and push each reviewable milestone directly to the
current branch and record its exact SHA in Den.

Run only the focused maintained checks:

```bash
dotnet run --project src/RustyRoguelike.Product.Checks/RustyRoguelike.Product.Checks.csproj
dotnet msbuild src/RustyRoguelike.Product/RustyRoguelike.Product.csproj -t:StageRustyEngineCoreClrProduct
./.runtime/runtime-pack-cbf35130d06c/bin/rusty dev --project ./src/RustyRoguelike.Product/RustyRoguelike.Product.csproj --runtime ./.runtime/runtime-pack-cbf35130d06c
dotnet msbuild src/RustyRoguelike.Product/RustyRoguelike.Product.csproj -t:VerifyRustyEngineAot
bash src/scripts/exercise-product.sh
```

`rusty dev` is the ordinary CoreCLR edit/run path. The explicit AOT target and
the isolated exercise are fidelity/release evidence, not a second development
host. Do not restore the removed Rust, Node, Angular, Nx, generated-protocol,
or broad browser-test workflows for compatibility. Add a focused proof only
when a new C# product behavior needs one.

Update [docs/source-provenance.md](docs/source-provenance.md) when Engine or
Procgen source selection changes, and [docs/known-limitations.md](docs/known-limitations.md)
when an intentional C# phase boundary remains.

## Den guidance bootstrap

- Project ID: `rusty-roguelike`
- Resolve live guidance with Den's `get_agent_guidance` before substantial work.
- Treat the resolved Den packet and its referenced Den documents as the source
  of truth.
- If Den is unreachable, stop and report the failed Den operation rather than
  reconstructing Den state from local files.
