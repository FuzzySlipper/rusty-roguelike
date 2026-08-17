/**
 * The single import surface for gameplay catalogs. Catalog files must import
 * from this module only — extending the grammar means editing `authoring/`
 * and the Rust compiler in `rust/crates/rusty-roguelike/src/rules/` in the
 * same change.
 */

export * from './definitions.js';
export * from './envelope.js';
