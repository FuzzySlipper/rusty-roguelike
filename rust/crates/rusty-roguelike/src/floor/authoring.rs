use rusty_procgen_preflight::core::CatalogAwareGenerationPolicy;
use rusty_procgen_preflight::{GeometryLayoutPolicy, SeedIntent, ShapeCatalog};

use super::FloorAdmissionError;

const INTENT_JSON: &str = include_str!("../../../../content/procgen/floor-intent.json");
const GEOMETRY_POLICY_JSON: &str = include_str!("../../../../content/procgen/geometry-policy.json");
const CATALOG_JSON: &str = include_str!("../../../../content/procgen/floor-catalog.json");
const CATALOG_POLICY_JSON: &str = include_str!("../../../../content/procgen/catalog-policy.json");

pub(crate) struct AuthoredGenerationInputs {
    pub seed: u64,
    pub intent: SeedIntent,
    pub geometry_policy: GeometryLayoutPolicy,
    pub catalog: ShapeCatalog,
    pub catalog_policy: CatalogAwareGenerationPolicy,
}

pub(crate) fn authored_inputs(seed: u64) -> Result<AuthoredGenerationInputs, FloorAdmissionError> {
    Ok(AuthoredGenerationInputs {
        seed,
        intent: decode("floor_intent_invalid", INTENT_JSON)?,
        geometry_policy: decode("geometry_policy_invalid", GEOMETRY_POLICY_JSON)?,
        catalog: decode("floor_catalog_invalid", CATALOG_JSON)?,
        catalog_policy: decode("catalog_policy_invalid", CATALOG_POLICY_JSON)?,
    })
}

fn decode<T: serde::de::DeserializeOwned>(
    code: &str,
    source: &str,
) -> Result<T, FloorAdmissionError> {
    serde_json::from_str(source).map_err(|error| FloorAdmissionError::new(code, error.to_string()))
}
