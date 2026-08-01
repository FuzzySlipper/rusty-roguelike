# Source provenance

## Runtime dependencies

- Rusty Engine: `https://github.com/FuzzySlipper/rusty-engine` at
  `fb608e323a8b44a55195f5720101224ff37fd5db`. The bootstrap consumes its public
  retained renderer packages directly.
- Rusty Procgen: `https://github.com/FuzzySlipper/rusty-procgen` at
  `1540ed9deb43cb259b94778cca2c2188ac635f03`. Rust links the public
  filesystem-free `rusty_procgen_preflight::core::ProcgenCore` facade.

`dependency-sources.json` and both lockfiles are the executable identity proof.
There are no sibling path fallbacks.

## Donor evidence

The initial Nx/Angular package seams, Rust static-host pattern, retained abstract
scene, and browser-proof approach were adapted from Rusty D20 through exact
revision `2ef818e180abf507b3af7fd9bc1029f1e0983237`. Names, contracts, code, and
ownership were reduced and rewritten for this product. Rusty D20 is not a
runtime, build, or test dependency.

The game premise and later Ruleweaver-inspired content are separately owned by
Rusty Roguelike. Donor evidence never overrides this repository's design.
