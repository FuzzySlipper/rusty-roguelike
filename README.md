# Rusty Roguelike

Rusty Roguelike is a compact C# downstream product for Rusty Engine:
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
rusty install
rusty dev --project ./src/RustyRoguelike.Product/RustyRoguelike.Product.csproj
```

`Directory.Build.props` pins the immutable Engine SDK/runtime pair. The Engine's
`rusty` command (bootstrap:
`curl -fsSL https://raw.githubusercontent.com/FuzzySlipper/rusty-engine/main/scripts/install-rusty.sh | bash`)
installs it, supplies its SDK package source, and runs the product on its
runtime; `rusty update` moves the pin. CoreCLR is the edit/run loop; NativeAOT is
a separate fidelity/release operation:

```bash
rusty build --project ./src/RustyRoguelike.Product/RustyRoguelike.Product.csproj --aot
bash src/scripts/exercise-product.sh
```

An Engine contributor may explicitly use `rusty dev --engine-source
/absolute/rusty-engine`; the project must then set the SDK-required explicit
source properties. Normal downstream work never discovers or builds an Engine
source checkout.

See [the design](docs/design.md), [code atlas](docs/agent-code-atlas.md),
[verification](docs/verification.md), and [known limitations](docs/known-limitations.md).
