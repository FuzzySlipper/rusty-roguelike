use rusty_procgen_preflight::core::ProcgenCore;

use super::{
    admission::admit_procgen_floor,
    authoring::authored_inputs,
    generation::{run_procgen, ProcgenFloorOutput},
    FloorFeatureKind, FloorGenerationRequest, FloorState,
};

const SEED: u64 = 5_201;

#[test]
fn exact_public_procgen_pipeline_is_deterministic_and_admitted() {
    let first = super::generate_authored_floor(SEED).expect("first admitted floor");
    let repeated = super::generate_authored_floor(SEED).expect("repeated admitted floor");
    assert_eq!(first, repeated);
    assert_eq!(first.schema_version, 1);
    assert!(first.bounds.width <= 128);
    assert!(first.bounds.height <= 128);
    assert!(first.walkable_cells.len() <= 4_096);
    assert_eq!(first.regions.len(), 4);
    assert_eq!(first.portals.len(), 4);
    assert_eq!(
        first
            .portals
            .iter()
            .find(|portal| portal.source_edge_id == "edge.gate_1.goal")
            .expect("goal gate portal")
            .required_item
            .as_deref(),
        Some("item.gate_key_1")
    );
    assert_eq!(
        first
            .features
            .iter()
            .map(|feature| feature.kind)
            .collect::<Vec<_>>(),
        vec![
            FloorFeatureKind::Gate,
            FloorFeatureKind::Goal,
            FloorFeatureKind::Key,
            FloorFeatureKind::Entry,
        ]
    );
    assert_eq!(first.provenance.seed, SEED);
    assert_eq!(first.provenance.rule_seed, SEED + 1);
    assert_eq!(first.provenance.geometry_seed, SEED + 2);
    assert_eq!(first.provenance.realization_seed, SEED + 3);
    assert_eq!(
        ProcgenCore::canonical_hash(&first).expect("floor hash"),
        ProcgenCore::canonical_hash(&repeated).expect("repeated floor hash")
    );
}

#[test]
fn malformed_procgen_output_and_rejected_generation_publish_nothing() {
    let inputs = authored_inputs(SEED).expect("authored inputs");
    let output = run_procgen(&inputs).expect("accepted Procgen output");
    let encoded = serde_json::to_vec(&output.result).expect("encode accepted result");
    let mut forged_result: rusty_procgen_preflight::core::CatalogAwareGenerationResult =
        serde_json::from_slice(&encoded).expect("clone accepted result");
    let duplicate = forged_result
        .placement
        .as_ref()
        .expect("accepted placement")
        .occupied_cells[0]
        .clone();
    forged_result
        .placement
        .as_mut()
        .expect("accepted placement")
        .occupied_cells
        .push(duplicate);
    let mut forged = ProcgenFloorOutput {
        candidate: output.candidate,
        source_geometry: output.source_geometry,
        source_plan: output.source_plan,
        result: forged_result,
        provenance: output.provenance,
    };
    forged.provenance.procgen_result_hash =
        ProcgenCore::canonical_hash(&forged.result).expect("forged result hash");
    forged.provenance.accepted_placement_hash = ProcgenCore::canonical_hash(
        forged
            .result
            .placement
            .as_ref()
            .expect("forged accepted placement"),
    )
    .expect("forged placement hash");
    assert_eq!(
        admit_procgen_floor(&inputs, forged)
            .expect_err("duplicate output must be rejected")
            .code(),
        "duplicate_walkable_cell"
    );

    let mut rejected_inputs = authored_inputs(SEED).expect("authored inputs");
    rejected_inputs.catalog.shapes.clear();
    assert_eq!(
        run_procgen(&rejected_inputs)
            .expect_err("empty catalog must be rejected")
            .code(),
        "procgen_catalog_coverage_gap"
    );
}

#[test]
fn failed_replacement_is_atomic() {
    let mut state = FloorState::default();
    let admitted = state
        .replace_generated(FloorGenerationRequest { seed: SEED })
        .expect("initial floor")
        .clone();
    let error = state
        .replace_generated(FloorGenerationRequest { seed: u64::MAX })
        .expect_err("derived seed overflow must reject");
    assert_eq!(error.code(), "generation_seed_range");
    assert_eq!(state.current(), Some(&admitted));
}
