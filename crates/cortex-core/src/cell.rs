mod descriptor;
mod descriptor_impls;
mod metadata;
#[cfg(test)]
mod tests;
mod types;
mod wire;
pub use descriptor::CellDescriptor;
pub use metadata::{KnowledgeCell, KnowledgeCellMetadata};
pub use types::KnowledgeCellType;
