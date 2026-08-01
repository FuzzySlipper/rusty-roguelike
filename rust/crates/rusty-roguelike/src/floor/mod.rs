mod admission;
mod authoring;
mod error;
mod generation;
mod types;

pub use error::FloorAdmissionError;
pub use generation::generate_authored_floor;
pub use types::{
    FloorBounds, FloorCell, FloorFeature, FloorFeatureKind, FloorGenerationProvenance,
    FloorGenerationRequest, FloorPortal, FloorRegion, FloorRegionKind, FloorState, GeneratedFloor,
};

#[cfg(test)]
mod tests;
