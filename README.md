# Rusty Roguelike

Rusty Roguelike is a compact C# NativeAOT downstream example for Rusty Engine:
a collapsed party shares one grid cell, accepts one revision-bound activation,
and settles admitted opposition with visible receipts. It is intentionally a
product, not an RPG framework.

The C# product owns rules, party/session policy, admission of the committed
Procgen starter-floor artifact, and save meaning. Rusty Engine supplies the
product lifecycle, input, content, spatial/navigation, voxel-scene
presentation, UI stream, deterministic random, and durable state mechanisms.
The host page is an observational readout, not a second renderer or gameplay
runtime.

```bash
dotnet build RustyRoguelike.sln -c Release
dotnet publish src/RustyRoguelike.NativeProduct/RustyRoguelike.NativeProduct.csproj -c Release -r linux-x64
bash src/scripts/exercise-native-product.sh
```

The final command starts the adjacent Engine C# product runtime, admits a
begin/save/load sequence, exercises lifecycle transitions, and releases the
product. It is deliberately a narrow continuation point rather than a full
interactive game certification.

See [the design](docs/design.md), [code atlas](docs/agent-code-atlas.md),
[verification](docs/verification.md), and [known limitations](docs/known-limitations.md).
