use serde::{Deserialize, Serialize};

use super::{generation::generate_floor, FloorAdmissionError};

pub const FLOOR_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FloorGenerationRequest {
    pub seed: u64,
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorCell {
    pub x: i32,
    pub y: i32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorBounds {
    pub min_x: i32,
    pub min_y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FloorRegionKind {
    Room,
    Threshold,
    Key,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorRegion {
    pub id: String,
    pub source_piece_id: String,
    pub kind: FloorRegionKind,
    pub cells: Vec<FloorCell>,
    pub tags: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FloorFeatureKind {
    Entry,
    Goal,
    Key,
    Gate,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorFeature {
    pub id: String,
    pub source_node_id: String,
    pub kind: FloorFeatureKind,
    pub cell: FloorCell,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorPortal {
    pub id: String,
    pub source_edge_id: String,
    pub cells: Vec<FloorCell>,
    pub orientation: String,
    pub traversal: String,
    pub required_item: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FloorGenerationProvenance {
    pub schema_version: u32,
    pub rusty_procgen_revision: String,
    pub seed: u64,
    pub rule_seed: u64,
    pub geometry_seed: u64,
    pub realization_seed: u64,
    pub intent_hash: String,
    pub geometry_policy_hash: String,
    pub catalog_hash: String,
    pub catalog_policy_hash: String,
    pub candidate_hash: String,
    pub source_geometry_hash: String,
    pub source_piece_plan_hash: String,
    pub procgen_result_hash: String,
    pub accepted_geometry_hash: String,
    pub accepted_placement_hash: String,
    pub selected_attempt: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedFloor {
    pub schema_version: u32,
    pub floor_id: String,
    pub bounds: FloorBounds,
    pub walkable_cells: Vec<FloorCell>,
    pub regions: Vec<FloorRegion>,
    pub features: Vec<FloorFeature>,
    pub portals: Vec<FloorPortal>,
    pub provenance: FloorGenerationProvenance,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FloorState {
    current: Option<GeneratedFloor>,
}

impl FloorState {
    pub fn current(&self) -> Option<&GeneratedFloor> {
        self.current.as_ref()
    }

    pub fn replace_generated(
        &mut self,
        request: FloorGenerationRequest,
    ) -> Result<&GeneratedFloor, FloorAdmissionError> {
        let proposed = generate_floor(request)?;
        self.current = Some(proposed);
        Ok(self
            .current
            .as_ref()
            .expect("the proposed floor was published"))
    }
}
