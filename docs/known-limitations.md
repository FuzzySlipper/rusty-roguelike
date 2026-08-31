# Known limitations

## Artifact-backed floor only

- **Status:** active, intentional first slice.
- **Affected surface:** `Floors/`, development-host content.
- **Limitation:** the product admits one reviewed committed Procgen artifact;
  it cannot generate a floor during a C# session.
- **Impact:** alternate seeds/catalogs require a separately produced,
  provenance-pinned artifact and matching admission profile.
- **Reason:** no safe public C# Procgen generation service was identified.
- **Follow-up:** request an upstream C# capability when runtime generation is a
  real product need. Do not add a Procgen port, Rust sidecar, CLI shell-out, or
  browser simulation.

## Narrow NativeAOT product readout

- **Status:** active, intentional continuation point.
- **Affected surface:** NativeAOT development host and UI projections.
- **Limitation:** the bundled page proves lifecycle/readout availability only;
  it is not a finished first-person UI, renderer composition, accessibility
  shell, or full gameplay-control surface.
- **Impact:** the maintained proof demonstrates begin, an accepted Wait
  activation, save/perturb/load restoration, and lifecycle—not broad
  interactive play.
- **Follow-up:** choose and build a concrete product UX through Engine's public
  presentation/input contracts. Keep all game decisions in C#.

## Side-effect-free movement admission

- **Status:** active upstream phase boundary.
- **Affected surface:** cardinal party movement.
- **Limitation:** the current public `Spatial.ProposeNavigationStep` mutates a
  retained Engine navigation-path readout. A command can still fail after a
  movement proposal, so the product refuses movement before that Engine call.
- **Impact:** `roguelike.move.*` is accepted as input syntax but its product
  command is rejected with `engine-navigation-rejected-step`; Wait is the
  demonstrated accepted activation.
- **Follow-up:** consume Rusty Engine task 7614's side-effect-free C# step
  admission when it is available. Do not query admitted floor cells, restore a
  path readout, or otherwise substitute for Engine navigation.

## Starter perception geometry

- **Status:** active, intentional small-floor posture.
- **Affected surface:** dormant-opposition admission.
- **Limitation:** admission uses copied Engine `Perception.QueryVisibility`
  facts plus the product's radius policy, but the starter floor contributes no
  dynamic occluder colliders to that query.
- **Impact:** Engine distance/facing results gate admission; no product-local
  visibility proxy or invented occlusion is present.
- **Follow-up:** when authored occluding geometry is admitted through an
  Engine-owned spatial representation, provide those colliders to the same
  query rather than adding downstream visibility code.

## Small starter rules and save schema

- **Status:** active, intentional first slice.
- **Affected surface:** C# rules, session, and persistence.
- **Limitation:** the catalog, one-floor roster, and schema-1 closed save are
  representative and intentionally modest. There is no claim of the retired
  Rust campaign's exhaustive rule, protocol, or browser coverage.
- **Follow-up:** expand C# definitions and save migrations together when a new
  product behavior is admitted; preserve named tuning/readout facts.
