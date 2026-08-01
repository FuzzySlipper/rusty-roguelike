mod bootstrap;
mod floor;

pub const RUSTY_ENGINE_REVISION: &str = "fb608e323a8b44a55195f5720101224ff37fd5db";
pub const RUSTY_PROCGEN_REVISION: &str = "1540ed9deb43cb259b94778cca2c2188ac635f03";

pub use bootstrap::{bootstrap_readout, generated_typescript, BootstrapReadout};
pub use floor::{
    generate_authored_floor, FloorAdmissionError, FloorBounds, FloorCell, FloorFeature,
    FloorFeatureKind, FloorGenerationProvenance, FloorGenerationRequest, FloorPortal, FloorRegion,
    FloorRegionKind, FloorState, GeneratedFloor,
};
