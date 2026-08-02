use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{FloorCell, RoguelikeId};

pub const WORLD_VIEW_SCHEMA_VERSION: u32 = 3;
pub const MAX_VIEW_DEPTH: i32 = 6;
pub const MAX_DISCOVERED_CELLS: usize = 4_096;
pub const MAX_PROJECTED_WORLD_FACTS: usize = 256;
pub const MAX_MINIMAP_FACTS: usize = MAX_DISCOVERED_CELLS * 2;
pub const MAX_VISIBLE_ACTORS: usize = 64;
pub const MAX_VISIBLE_SCENE_PLACEMENTS: usize = 64;
pub const MAX_WORLD_FLOOR_ID_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WorldCell {
    pub x: i32,
    pub y: i32,
}

impl From<&FloorCell> for WorldCell {
    fn from(value: &FloorCell) -> Self {
        Self {
            x: value.x,
            y: value.y,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum Facing {
    North,
    East,
    South,
    West,
}

impl Facing {
    pub const fn left(self) -> Self {
        match self {
            Self::North => Self::West,
            Self::West => Self::South,
            Self::South => Self::East,
            Self::East => Self::North,
        }
    }

    pub const fn right(self) -> Self {
        match self {
            Self::North => Self::East,
            Self::East => Self::South,
            Self::South => Self::West,
            Self::West => Self::North,
        }
    }

    pub const fn forward(self) -> (i32, i32) {
        match self {
            Self::North => (0, -1),
            Self::East => (1, 0),
            Self::South => (0, 1),
            Self::West => (-1, 0),
        }
    }

    pub const fn right_axis(self) -> (i32, i32) {
        let (x, y) = self.forward();
        (-y, x)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum RelativeStep {
    Forward,
    Backward,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum EnemyParticipation {
    Dormant,
    Participating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum WorldViewCellKind {
    Floor,
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum RelativeSceneFacing {
    Forward,
    Right,
    Backward,
    Left,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
#[ts(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum VisibleSceneContent {
    Prop {
        content_id: String,
    },
    PointLight {
        color_rgb: String,
        intensity_milli: u32,
        range_cells: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct VisibleScenePlacementView {
    pub id: String,
    pub lateral: i16,
    pub depth: u8,
    pub facing: RelativeSceneFacing,
    pub content: VisibleSceneContent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum MinimapTerrainKind {
    Floor,
    Wall,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(rename_all = "kebab-case")]
pub enum MinimapFeatureKind {
    Entry,
    Goal,
    Key,
    OpenDoor,
    LockedDoor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MinimapCellView {
    pub x: i32,
    pub y: i32,
    pub terrain: MinimapTerrainKind,
    pub feature: Option<MinimapFeatureKind>,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MinimapActorView {
    pub actor_id: RoguelikeId,
    #[ts(type = "number")]
    pub entity_id: u64,
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub participating: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct MinimapView {
    pub party: WorldCell,
    pub facing: Facing,
    pub cells: Vec<MinimapCellView>,
    pub visible_actors: Vec<MinimapActorView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WorldViewCell {
    pub lateral: i16,
    pub depth: u8,
    pub kind: WorldViewCellKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct VisibleActorView {
    pub actor_id: RoguelikeId,
    #[ts(type = "number")]
    pub entity_id: u64,
    pub name: String,
    pub lateral: i16,
    pub depth: u8,
    pub participating: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
#[ts(rename_all = "camelCase")]
pub struct WorldView {
    pub schema_version: u32,
    #[ts(type = "number")]
    pub revision: u64,
    pub floor_id: String,
    pub facing: Facing,
    pub discovered_cell_count: u16,
    pub cells: Vec<WorldViewCell>,
    pub scene_placements: Vec<VisibleScenePlacementView>,
    pub visible_actors: Vec<VisibleActorView>,
    pub minimap: MinimapView,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldStateError {
    code: &'static str,
    detail: String,
}

impl WorldStateError {
    pub fn new(code: &'static str, detail: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
        }
    }

    pub const fn code(&self) -> &'static str {
        self.code
    }

    pub fn detail(&self) -> &str {
        &self.detail
    }
}

impl std::fmt::Display for WorldStateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}: {}", self.code, self.detail)
    }
}

impl std::error::Error for WorldStateError {}
