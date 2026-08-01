mod component;
mod navigation;
mod projection;
mod state;
mod types;

pub use component::*;
pub use state::*;
pub use types::*;

pub(crate) fn generated_world_typescript() -> String {
    use ts_rs::TS;

    let declarations = [
        WorldCell::decl(),
        Facing::decl(),
        EnemyParticipation::decl(),
        WorldViewCellKind::decl(),
        WorldViewCell::decl(),
        VisibleActorView::decl(),
        WorldView::decl(),
    ]
    .into_iter()
    .map(|declaration| format!("export {declaration}"))
    .collect::<Vec<_>>()
    .join("\n\n");
    format!(
        "export const WORLD_VIEW_SCHEMA_VERSION = {WORLD_VIEW_SCHEMA_VERSION} as const;\n\
export const WORLD_VIEW_LIMITS = Object.freeze({{\n\
  maxDepth: {MAX_VIEW_DEPTH},\n\
  maxDiscoveredCells: {MAX_DISCOVERED_CELLS},\n\
  maxProjectedFacts: {MAX_PROJECTED_WORLD_FACTS},\n\
  maxVisibleActors: {MAX_VISIBLE_ACTORS},\n\
  maxFloorIdBytes: {MAX_WORLD_FLOOR_ID_BYTES},\n\
}} as const);\n\n{declarations}"
    )
}

#[cfg(test)]
mod tests;
