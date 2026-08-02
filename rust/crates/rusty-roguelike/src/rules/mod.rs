mod authored;
mod candidate;
mod compiler;
mod component;
mod definitions;
mod identity;
mod mechanics;

pub const MAX_ROGUELIKE_DEFINITIONS_PER_KIND: usize = 64;
pub const MAX_ROGUELIKE_AUTHORED_TEXT_BYTES: usize = 512;
pub const MAX_ROGUELIKE_ACTION_TAGS: usize = 16;
pub const MAX_ROGUELIKE_DAMAGE_DICE: u8 = 16;
pub const MAX_ROGUELIKE_DAMAGE_DIE_SIDES: u16 = 100;
pub const MAX_ROGUELIKE_STATIC_ROLLS: usize = 4_096;
pub const MAX_ROGUELIKE_RANGE: u8 = 16;
pub const MAX_ROGUELIKE_EXPERIENCE: u32 = 1_000_000_000;

#[cfg(test)]
pub(crate) use authored::starter_ruleset_with_opposition;
pub use authored::{starter_candidate, starter_rule_package, starter_ruleset};
pub use candidate::*;
pub use compiler::RoguelikeCompileError;
pub use component::*;
pub use definitions::*;
pub use identity::{
    RoguelikeId, RoguelikeIdentityError, MAX_ROGUELIKE_ID_BYTES, ROGUELIKE_ID_PATTERN,
};
pub use mechanics::{
    defense_stat_id, equipment_slot_id, feat_source_id, inventory_capacity_id, item_definition_id,
    item_source_id, vitality_maximum_stat_id, vitality_track_id,
};

#[cfg(test)]
mod tests;
