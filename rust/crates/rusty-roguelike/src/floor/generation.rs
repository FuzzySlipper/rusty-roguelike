use rusty_procgen_preflight::core::{
    CatalogAwareGenerationProvenance, CorridorRealization, GraphRule, ProcgenCore, RuleDisposition,
};
use rusty_procgen_preflight::{Candidate, Geometry2dArtifact, PieceBuildPlan};

use crate::RUSTY_PROCGEN_REVISION;

use super::{
    admission::admit_procgen_floor,
    authoring::{authored_inputs, AuthoredGenerationInputs},
    FloorAdmissionError, FloorGenerationProvenance, FloorGenerationRequest, GeneratedFloor,
};

pub fn generate_authored_floor(seed: u64) -> Result<GeneratedFloor, FloorAdmissionError> {
    generate_floor(FloorGenerationRequest { seed })
}

pub(crate) fn generate_floor(
    request: FloorGenerationRequest,
) -> Result<GeneratedFloor, FloorAdmissionError> {
    let inputs = authored_inputs(request.seed)?;
    let output = run_procgen(&inputs)?;
    admit_procgen_floor(&inputs, output)
}

#[derive(Debug)]
pub(crate) struct ProcgenFloorOutput {
    pub candidate: Candidate,
    pub source_geometry: Geometry2dArtifact,
    pub source_plan: PieceBuildPlan,
    pub result: rusty_procgen_preflight::core::CatalogAwareGenerationResult,
    pub provenance: FloorGenerationProvenance,
}

pub(crate) fn run_procgen(
    inputs: &AuthoredGenerationInputs,
) -> Result<ProcgenFloorOutput, FloorAdmissionError> {
    let rule_seed = derived_seed(inputs.seed, 1)?;
    let geometry_seed = derived_seed(inputs.seed, 2)?;
    let realization_seed = derived_seed(inputs.seed, 3)?;

    ProcgenCore::validate_geometry_policy(&inputs.geometry_policy)
        .map_err(|detail| FloorAdmissionError::new("geometry_policy_rejected", detail))?;
    let catalog_inspection = ProcgenCore::inspect_catalog(&inputs.catalog);
    if !catalog_inspection.diagnostics.is_empty() {
        return Err(FloorAdmissionError::new(
            "floor_catalog_rejected",
            diagnostic_summary(&catalog_inspection.diagnostics),
        ));
    }

    let base = ProcgenCore::create_candidate(&inputs.intent, inputs.seed);
    let application = ProcgenCore::apply_rule(&base, GraphRule::LockKeyLoop, rule_seed);
    if application.disposition != RuleDisposition::Accepted {
        return Err(FloorAdmissionError::new(
            "procgen_rule_rejected",
            diagnostic_summary(&application.diagnostics),
        ));
    }
    let candidate = application.candidate;
    require_validation(
        "candidate_rejected",
        ProcgenCore::validate_candidate(&candidate),
    )?;
    let annotations = ProcgenCore::annotate_spatial_intent(&candidate)
        .map_err(|detail| FloorAdmissionError::new("spatial_intent_failed", detail))?;
    let intermediate = ProcgenCore::breakdown(&candidate, &annotations)
        .map_err(|detail| FloorAdmissionError::new("intermediate_failed", detail))?;
    require_validation(
        "intermediate_rejected",
        ProcgenCore::validate_intermediate(&intermediate),
    )?;
    let connections = ProcgenCore::plan_connections(&candidate, &intermediate)
        .map_err(|detail| FloorAdmissionError::new("connection_plan_failed", detail))?;
    let source_geometry = ProcgenCore::emit_geometry(
        &candidate,
        &intermediate,
        &connections,
        &inputs.geometry_policy,
        geometry_seed,
    )
    .map_err(|detail| FloorAdmissionError::new("geometry_generation_failed", detail))?;
    require_validation(
        "geometry_rejected",
        ProcgenCore::validate_geometry(&source_geometry),
    )?;
    let source_plan = ProcgenCore::emit_piece_plan(
        &candidate,
        &intermediate,
        &source_geometry,
        CorridorRealization::Catalog,
    )
    .map_err(|detail| FloorAdmissionError::new("piece_plan_failed", detail))?;
    let inert_provenance = CatalogAwareGenerationProvenance {
        candidate_ref: "memory/rusty-roguelike/candidate.json".to_owned(),
        geometry_ref: "memory/rusty-roguelike/geometry.json".to_owned(),
        piece_plan_ref: "memory/rusty-roguelike/piece-plan.json".to_owned(),
        catalog_ref: "memory/rusty-roguelike/floor-catalog.json".to_owned(),
        result_ref: "memory/rusty-roguelike/catalog-result.json".to_owned(),
    };
    let result = ProcgenCore::realize_catalog_aware(
        &candidate,
        &source_geometry,
        &source_plan,
        &inputs.catalog,
        &inputs.catalog_policy,
        &inert_provenance,
        realization_seed,
    )
    .map_err(|detail| FloorAdmissionError::new("procgen_generation_failed", detail))?;
    if !result.ok {
        let classification = result
            .exhausted_classification
            .as_deref()
            .unwrap_or("unclassified");
        return Err(FloorAdmissionError::new(
            format!("procgen_{classification}"),
            result
                .attempts
                .last()
                .map(|attempt| attempt.detail.clone())
                .unwrap_or_else(|| {
                    "Procgen rejected the generation without attempt evidence.".to_owned()
                }),
        ));
    }

    let selected_attempt = result.selected_attempt.ok_or_else(|| {
        FloorAdmissionError::new(
            "procgen_selected_attempt_missing",
            "an accepted Procgen result did not identify its selected attempt",
        )
    })?;
    let provenance = FloorGenerationProvenance {
        schema_version: 1,
        rusty_procgen_revision: RUSTY_PROCGEN_REVISION.to_owned(),
        seed: inputs.seed,
        rule_seed,
        geometry_seed,
        realization_seed,
        intent_hash: hash("intent_hash_failed", &inputs.intent)?,
        geometry_policy_hash: hash("geometry_policy_hash_failed", &inputs.geometry_policy)?,
        catalog_hash: hash("catalog_hash_failed", &inputs.catalog)?,
        catalog_policy_hash: hash("catalog_policy_hash_failed", &inputs.catalog_policy)?,
        candidate_hash: hash("candidate_hash_failed", &candidate)?,
        source_geometry_hash: hash("source_geometry_hash_failed", &source_geometry)?,
        source_piece_plan_hash: hash("source_piece_plan_hash_failed", &source_plan)?,
        procgen_result_hash: hash("procgen_result_hash_failed", &result)?,
        accepted_geometry_hash: hash(
            "accepted_geometry_hash_failed",
            result.geometry.as_ref().ok_or_else(|| {
                FloorAdmissionError::new(
                    "procgen_geometry_missing",
                    "an accepted Procgen result omitted geometry",
                )
            })?,
        )?,
        accepted_placement_hash: hash(
            "accepted_placement_hash_failed",
            result.placement.as_ref().ok_or_else(|| {
                FloorAdmissionError::new(
                    "procgen_placement_missing",
                    "an accepted Procgen result omitted placement",
                )
            })?,
        )?,
        selected_attempt,
    };
    Ok(ProcgenFloorOutput {
        candidate,
        source_geometry,
        source_plan,
        result,
        provenance,
    })
}

fn derived_seed(seed: u64, offset: u64) -> Result<u64, FloorAdmissionError> {
    seed.checked_add(offset).ok_or_else(|| {
        FloorAdmissionError::new(
            "generation_seed_range",
            "the floor seed leaves no room for deterministic stage seeds",
        )
    })
}

fn require_validation(
    code: &str,
    report: rusty_procgen_preflight::ValidationReport,
) -> Result<(), FloorAdmissionError> {
    if report.ok {
        Ok(())
    } else {
        Err(FloorAdmissionError::new(
            code,
            diagnostic_summary(&report.diagnostics),
        ))
    }
}

fn diagnostic_summary(diagnostics: &[rusty_procgen_preflight::Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(|diagnostic| format!("{}: {}", diagnostic.code, diagnostic.detail))
        .collect::<Vec<_>>()
        .join("; ")
}

fn hash<T: serde::Serialize>(code: &str, value: &T) -> Result<String, FloorAdmissionError> {
    ProcgenCore::canonical_hash(value).map_err(|detail| FloorAdmissionError::new(code, detail))
}
