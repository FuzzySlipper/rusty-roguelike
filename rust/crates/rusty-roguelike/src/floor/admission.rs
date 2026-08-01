use std::collections::{BTreeMap, BTreeSet, VecDeque};

use rusty_procgen_preflight::core::ProcgenCore;
use rusty_procgen_preflight::{GridConnectivity, PieceInstance, PiecePlacement};

use crate::RUSTY_PROCGEN_REVISION;

use super::{
    authoring::AuthoredGenerationInputs, generation::ProcgenFloorOutput, FloorAdmissionError,
    FloorBounds, FloorCell, FloorFeature, FloorFeatureKind, FloorPortal, FloorRegion,
    FloorRegionKind, GeneratedFloor,
};

const MAX_FLOOR_WIDTH: u32 = 128;
const MAX_FLOOR_HEIGHT: u32 = 128;
const MAX_WALKABLE_CELLS: usize = 4_096;
const EXPECTED_NODES: [&str; 4] = ["gate.locked_1", "goal", "key.gate_1", "start"];

pub(crate) fn admit_procgen_floor(
    inputs: &AuthoredGenerationInputs,
    output: ProcgenFloorOutput,
) -> Result<GeneratedFloor, FloorAdmissionError> {
    require_provenance_integrity(inputs, &output)?;
    if output.result.candidate_id != output.candidate.candidate_id {
        return rejected(
            "procgen_candidate_mismatch",
            "the accepted result does not identify the generated candidate",
        );
    }
    if output.result.policy != inputs.catalog_policy {
        return rejected(
            "procgen_policy_mismatch",
            "the accepted result does not retain the authored catalog policy",
        );
    }
    require_result_validation(&output)?;
    let placement = output.result.placement.as_ref().ok_or_else(|| {
        FloorAdmissionError::new(
            "procgen_placement_missing",
            "the accepted result omitted its placement",
        )
    })?;
    if placement.catalog_id != inputs.catalog.catalog_id {
        return rejected(
            "procgen_catalog_mismatch",
            "the accepted placement does not identify the authored catalog",
        );
    }
    if placement.cell_size != 1 || placement.grid_connectivity != GridConnectivity::FourWay {
        return rejected(
            "floor_grid_incompatible",
            "the game admits only unit-sized four-way placements",
        );
    }

    let walkable_cells = admit_walkable_cells(placement)?;
    let bounds = floor_bounds(&walkable_cells)?;
    require_bounds(&bounds, walkable_cells.len())?;
    require_connected(&walkable_cells)?;
    let (regions, features) = admit_regions(placement, &walkable_cells)?;
    let portals = admit_portals(placement, &walkable_cells)?;
    let result_hash_suffix = output
        .provenance
        .procgen_result_hash
        .strip_prefix("fnv1a64:")
        .unwrap_or(output.provenance.procgen_result_hash.as_str());

    Ok(GeneratedFloor {
        schema_version: super::types::FLOOR_SCHEMA_VERSION,
        floor_id: format!("floor.{}.{}", inputs.seed, result_hash_suffix),
        bounds,
        walkable_cells,
        regions,
        features,
        portals,
        provenance: output.provenance,
    })
}

fn require_provenance_integrity(
    inputs: &AuthoredGenerationInputs,
    output: &ProcgenFloorOutput,
) -> Result<(), FloorAdmissionError> {
    let provenance = &output.provenance;
    if provenance.schema_version != 1 {
        return rejected(
            "procgen_provenance_schema_unsupported",
            format!(
                "expected provenance schema 1, observed {}",
                provenance.schema_version
            ),
        );
    }
    if provenance.rusty_procgen_revision != RUSTY_PROCGEN_REVISION {
        return rejected(
            "procgen_revision_mismatch",
            "the generated floor does not identify the pinned Procgen revision",
        );
    }
    let expected_rule_seed = inputs.seed.checked_add(1);
    let expected_geometry_seed = inputs.seed.checked_add(2);
    let expected_realization_seed = inputs.seed.checked_add(3);
    if provenance.seed != inputs.seed
        || Some(provenance.rule_seed) != expected_rule_seed
        || Some(provenance.geometry_seed) != expected_geometry_seed
        || Some(provenance.realization_seed) != expected_realization_seed
    {
        return rejected(
            "procgen_seed_provenance_mismatch",
            "the generated floor does not retain its authored stage seeds",
        );
    }

    require_hash(
        "intent_hash_mismatch",
        &provenance.intent_hash,
        &inputs.intent,
    )?;
    require_hash(
        "geometry_policy_hash_mismatch",
        &provenance.geometry_policy_hash,
        &inputs.geometry_policy,
    )?;
    require_hash(
        "catalog_hash_mismatch",
        &provenance.catalog_hash,
        &inputs.catalog,
    )?;
    require_hash(
        "catalog_policy_hash_mismatch",
        &provenance.catalog_policy_hash,
        &inputs.catalog_policy,
    )?;
    require_hash(
        "candidate_hash_mismatch",
        &provenance.candidate_hash,
        &output.candidate,
    )?;
    require_hash(
        "source_geometry_hash_mismatch",
        &provenance.source_geometry_hash,
        &output.source_geometry,
    )?;
    require_hash(
        "source_piece_plan_hash_mismatch",
        &provenance.source_piece_plan_hash,
        &output.source_plan,
    )?;
    require_hash(
        "procgen_result_hash_mismatch",
        &provenance.procgen_result_hash,
        &output.result,
    )?;
    require_hash(
        "accepted_geometry_hash_mismatch",
        &provenance.accepted_geometry_hash,
        output.result.geometry.as_ref().ok_or_else(|| {
            FloorAdmissionError::new(
                "procgen_geometry_missing",
                "the accepted result omitted geometry",
            )
        })?,
    )?;
    require_hash(
        "accepted_placement_hash_mismatch",
        &provenance.accepted_placement_hash,
        output.result.placement.as_ref().ok_or_else(|| {
            FloorAdmissionError::new(
                "procgen_placement_missing",
                "the accepted result omitted its placement",
            )
        })?,
    )?;
    if output.result.selected_attempt != Some(provenance.selected_attempt) {
        return rejected(
            "procgen_attempt_provenance_mismatch",
            "the generated floor does not retain the selected Procgen attempt",
        );
    }
    Ok(())
}

fn require_hash<T: serde::Serialize>(
    code: &str,
    observed: &str,
    value: &T,
) -> Result<(), FloorAdmissionError> {
    let expected = ProcgenCore::canonical_hash(value)
        .map_err(|detail| FloorAdmissionError::new("procgen_provenance_hash_failed", detail))?;
    if observed == expected {
        Ok(())
    } else {
        rejected(code, format!("expected {expected}, observed {observed}"))
    }
}

fn require_result_validation(output: &ProcgenFloorOutput) -> Result<(), FloorAdmissionError> {
    for (code, report) in [
        (
            "procgen_accepted_geometry_invalid",
            output.result.geometry_validation.as_ref(),
        ),
        (
            "procgen_accepted_placement_invalid",
            output.result.placement_validation.as_ref(),
        ),
    ] {
        let report = report.ok_or_else(|| {
            FloorAdmissionError::new(code, "the accepted result omitted validation evidence")
        })?;
        if !report.ok {
            return rejected(
                code,
                "the accepted result carries failed validation evidence",
            );
        }
    }
    let built_flow = output
        .result
        .built_flow_validation
        .as_ref()
        .ok_or_else(|| {
            FloorAdmissionError::new(
                "procgen_built_flow_missing",
                "the accepted result omitted built-flow validation",
            )
        })?;
    if !built_flow.ok {
        return rejected(
            "procgen_built_flow_invalid",
            "the accepted result carries failed built-flow validation",
        );
    }
    let geometry = output.result.geometry.as_ref().ok_or_else(|| {
        FloorAdmissionError::new(
            "procgen_geometry_missing",
            "the accepted result omitted geometry",
        )
    })?;
    let plan = output.result.piece_plan.as_ref().ok_or_else(|| {
        FloorAdmissionError::new(
            "procgen_piece_plan_missing",
            "the accepted result omitted its piece plan",
        )
    })?;
    let placement = output.result.placement.as_ref().ok_or_else(|| {
        FloorAdmissionError::new(
            "procgen_placement_missing",
            "the accepted result omitted its placement",
        )
    })?;
    if geometry.candidate_id != output.candidate.candidate_id
        || plan.candidate_id != output.candidate.candidate_id
        || output.source_geometry.candidate_id != output.candidate.candidate_id
        || output.source_plan.candidate_id != output.candidate.candidate_id
    {
        return rejected(
            "procgen_artifact_chain_mismatch",
            "the accepted artifact chain does not retain one candidate identity",
        );
    }

    let fresh_geometry = ProcgenCore::validate_geometry(geometry);
    if !fresh_geometry.ok {
        return rejected(
            "procgen_fresh_geometry_invalid",
            diagnostic_summary(&fresh_geometry.diagnostics),
        );
    }
    let fresh_placement = ProcgenCore::validate_placement(placement);
    if !fresh_placement.ok {
        return rejected(
            "procgen_fresh_placement_invalid",
            diagnostic_summary(&fresh_placement.diagnostics),
        );
    }
    let fresh_built_flow =
        ProcgenCore::validate_built_flow(&output.candidate, geometry, plan, placement);
    if !fresh_built_flow.ok {
        return rejected(
            "procgen_fresh_built_flow_invalid",
            diagnostic_summary(&fresh_built_flow.diagnostics),
        );
    }
    Ok(())
}

fn diagnostic_summary(diagnostics: &[rusty_procgen_preflight::Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.detail))
        .collect::<Vec<_>>()
        .join("; ")
}

fn admit_walkable_cells(placement: &PiecePlacement) -> Result<Vec<FloorCell>, FloorAdmissionError> {
    let mut cells = BTreeSet::new();
    for cell in &placement.occupied_cells {
        if !cells.insert(FloorCell {
            x: cell.x,
            y: cell.y,
        }) {
            return rejected(
                "duplicate_walkable_cell",
                format!("placement repeats walkable cell {},{}", cell.x, cell.y),
            );
        }
    }
    if cells.is_empty() {
        return rejected("floor_empty", "placement contains no walkable cells");
    }
    Ok(cells.into_iter().collect())
}

fn floor_bounds(cells: &[FloorCell]) -> Result<FloorBounds, FloorAdmissionError> {
    let min_x = cells
        .iter()
        .map(|cell| cell.x)
        .min()
        .expect("nonempty cells");
    let max_x = cells
        .iter()
        .map(|cell| cell.x)
        .max()
        .expect("nonempty cells");
    let min_y = cells
        .iter()
        .map(|cell| cell.y)
        .min()
        .expect("nonempty cells");
    let max_y = cells
        .iter()
        .map(|cell| cell.y)
        .max()
        .expect("nonempty cells");
    let width = i64::from(max_x) - i64::from(min_x) + 1;
    let height = i64::from(max_y) - i64::from(min_y) + 1;
    Ok(FloorBounds {
        min_x,
        min_y,
        width: u32::try_from(width).map_err(|_| {
            FloorAdmissionError::new("floor_bounds_overflow", "floor width does not fit u32")
        })?,
        height: u32::try_from(height).map_err(|_| {
            FloorAdmissionError::new("floor_bounds_overflow", "floor height does not fit u32")
        })?,
    })
}

fn require_bounds(bounds: &FloorBounds, cells: usize) -> Result<(), FloorAdmissionError> {
    if bounds.width > MAX_FLOOR_WIDTH
        || bounds.height > MAX_FLOOR_HEIGHT
        || cells > MAX_WALKABLE_CELLS
    {
        return rejected(
            "floor_bounds_exceeded",
            format!(
                "floor is {}x{} with {} cells; maxima are {}x{} and {} cells",
                bounds.width,
                bounds.height,
                cells,
                MAX_FLOOR_WIDTH,
                MAX_FLOOR_HEIGHT,
                MAX_WALKABLE_CELLS
            ),
        );
    }
    Ok(())
}

fn require_connected(cells: &[FloorCell]) -> Result<(), FloorAdmissionError> {
    let all = cells
        .iter()
        .map(|cell| (cell.x, cell.y))
        .collect::<BTreeSet<_>>();
    let first = *all.iter().next().expect("nonempty cells");
    let mut reached = BTreeSet::from([first]);
    let mut pending = VecDeque::from([first]);
    while let Some((x, y)) = pending.pop_front() {
        let neighbors = [
            x.checked_sub(1).map(|next_x| (next_x, y)),
            x.checked_add(1).map(|next_x| (next_x, y)),
            y.checked_sub(1).map(|next_y| (x, next_y)),
            y.checked_add(1).map(|next_y| (x, next_y)),
        ];
        for next in neighbors.into_iter().flatten() {
            if all.contains(&next) && reached.insert(next) {
                pending.push_back(next);
            }
        }
    }
    if reached.len() != all.len() {
        return rejected(
            "floor_disconnected",
            format!(
                "only {} of {} walkable cells share the entry component",
                reached.len(),
                all.len()
            ),
        );
    }
    Ok(())
}

fn admit_regions(
    placement: &PiecePlacement,
    walkable: &[FloorCell],
) -> Result<(Vec<FloorRegion>, Vec<FloorFeature>), FloorAdmissionError> {
    let global = walkable
        .iter()
        .map(|cell| (cell.x, cell.y))
        .collect::<BTreeSet<_>>();
    let mut by_node = BTreeMap::<String, (&PieceInstance, Vec<FloorCell>)>::new();
    for instance in &placement.instances {
        let node_refs = instance
            .source_refs
            .iter()
            .filter_map(|source| source.strip_prefix("node:"))
            .collect::<Vec<_>>();
        if node_refs.is_empty() {
            continue;
        }
        if node_refs.len() != 1 {
            return rejected(
                "floor_region_source_ambiguous",
                format!(
                    "{} has {} node references",
                    instance.piece_id,
                    node_refs.len()
                ),
            );
        }
        let mut cells = instance
            .occupied_cells
            .iter()
            .map(|cell| FloorCell {
                x: cell.x,
                y: cell.y,
            })
            .collect::<Vec<_>>();
        cells.sort();
        cells.dedup();
        if cells.is_empty() || cells.iter().any(|cell| !global.contains(&(cell.x, cell.y))) {
            return rejected(
                "floor_region_cells_invalid",
                format!(
                    "{} does not map to admitted walkable cells",
                    instance.piece_id
                ),
            );
        }
        if by_node
            .insert(node_refs[0].to_owned(), (instance, cells))
            .is_some()
        {
            return rejected(
                "floor_region_source_duplicate",
                format!("node {} maps to more than one room", node_refs[0]),
            );
        }
    }
    let observed = by_node.keys().map(String::as_str).collect::<Vec<_>>();
    if observed != EXPECTED_NODES {
        return rejected(
            "floor_region_set_incompatible",
            format!(
                "expected nodes {:?}, observed {:?}",
                EXPECTED_NODES, observed
            ),
        );
    }

    let mut regions = Vec::new();
    let mut features = Vec::new();
    for (node, (instance, cells)) in by_node {
        let (region_kind, feature_kind) = node_semantics(node.as_str(), instance)?;
        let cell = central_cell(&cells);
        regions.push(FloorRegion {
            id: format!("region.{node}"),
            source_piece_id: instance.piece_id.clone(),
            kind: region_kind,
            cells,
            tags: sorted_unique(instance.tags.clone()),
        });
        features.push(FloorFeature {
            id: format!("feature.{node}"),
            source_node_id: node,
            kind: feature_kind,
            cell,
        });
    }
    regions.sort_by(|left, right| left.id.cmp(&right.id));
    features.sort_by(|left, right| left.id.cmp(&right.id));
    Ok((regions, features))
}

fn node_semantics(
    node: &str,
    instance: &PieceInstance,
) -> Result<(FloorRegionKind, FloorFeatureKind), FloorAdmissionError> {
    let expected = match node {
        "start" => ("room", FloorRegionKind::Room, FloorFeatureKind::Entry),
        "goal" => ("room", FloorRegionKind::Room, FloorFeatureKind::Goal),
        "key.gate_1" => ("key", FloorRegionKind::Key, FloorFeatureKind::Key),
        "gate.locked_1" => (
            "threshold",
            FloorRegionKind::Threshold,
            FloorFeatureKind::Gate,
        ),
        _ => {
            return rejected(
                "floor_feature_unknown",
                format!("node {node} has no Roguelike floor semantics"),
            )
        }
    };
    if instance.requirement_kind != expected.0 {
        return rejected(
            "floor_region_kind_mismatch",
            format!(
                "node {node} requires {} but placement used {}",
                expected.0, instance.requirement_kind
            ),
        );
    }
    Ok((expected.1, expected.2))
}

fn central_cell(cells: &[FloorCell]) -> FloorCell {
    let min_x = cells.iter().map(|cell| cell.x).min().expect("region cells");
    let max_x = cells.iter().map(|cell| cell.x).max().expect("region cells");
    let min_y = cells.iter().map(|cell| cell.y).min().expect("region cells");
    let max_y = cells.iter().map(|cell| cell.y).max().expect("region cells");
    let target_x = min_x + (max_x - min_x) / 2;
    let target_y = min_y + (max_y - min_y) / 2;
    cells
        .iter()
        .min_by_key(|cell| {
            (
                cell.x.abs_diff(target_x) + cell.y.abs_diff(target_y),
                cell.x,
                cell.y,
            )
        })
        .expect("region cells")
        .clone()
}

fn admit_portals(
    placement: &PiecePlacement,
    walkable: &[FloorCell],
) -> Result<Vec<FloorPortal>, FloorAdmissionError> {
    let global = walkable
        .iter()
        .map(|cell| (cell.x, cell.y))
        .collect::<BTreeSet<_>>();
    let mut ids = BTreeSet::new();
    let mut semantics = BTreeMap::new();
    let mut portals = Vec::new();
    for portal in &placement.gate_portals {
        if !ids.insert(portal.id.as_str()) {
            return rejected(
                "floor_portal_duplicate",
                format!("portal {} is repeated", portal.id),
            );
        }
        if portal.width != 1
            || !matches!(
                portal.orientation.as_str(),
                "north" | "east" | "south" | "west"
            )
            || !matches!(portal.traversal.as_str(), "open" | "locked")
            || (portal.traversal == "locked") != portal.required_item.is_some()
        {
            return rejected(
                "floor_portal_incompatible",
                format!("portal {} has unsupported traversal geometry", portal.id),
            );
        }
        if semantics
            .insert(
                portal.source_edge.as_str(),
                (portal.traversal.as_str(), portal.required_item.as_deref()),
            )
            .is_some()
        {
            return rejected(
                "floor_portal_source_duplicate",
                format!("edge {} maps to more than one portal", portal.source_edge),
            );
        }
        let mut cells = portal
            .cells
            .iter()
            .map(|cell| FloorCell {
                x: cell.x,
                y: cell.y,
            })
            .collect::<Vec<_>>();
        cells.sort();
        cells.dedup();
        if cells.len() != portal.cells.len()
            || cells.is_empty()
            || cells.iter().any(|cell| !global.contains(&(cell.x, cell.y)))
        {
            return rejected(
                "floor_portal_cells_invalid",
                format!("portal {} does not bind unique walkable cells", portal.id),
            );
        }
        portals.push(FloorPortal {
            id: portal.id.clone(),
            source_edge_id: portal.source_edge.clone(),
            cells,
            orientation: portal.orientation.clone(),
            traversal: portal.traversal.clone(),
            required_item: portal.required_item.clone(),
        });
    }
    portals.sort_by(|left, right| left.id.cmp(&right.id));
    let expected_semantics = BTreeMap::from([
        ("edge.gate_1.goal", ("locked", Some("item.gate_key_1"))),
        ("edge.key_1.gate_1", ("open", None)),
        ("edge.start.gate_1", ("open", None)),
        ("edge.start.key_1", ("open", None)),
    ]);
    if semantics != expected_semantics {
        return rejected(
            "floor_portal_set_incompatible",
            format!(
                "the first floor requires the authored lock/key portal semantics; observed {:?}",
                semantics
            ),
        );
    }
    Ok(portals)
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn rejected<T>(code: &str, detail: impl Into<String>) -> Result<T, FloorAdmissionError> {
    Err(FloorAdmissionError::new(code, detail))
}
