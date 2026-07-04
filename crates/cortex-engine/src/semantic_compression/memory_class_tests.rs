//! memory-class + semantic-compression unit tests (moved from semantic_compression.rs; behavior unchanged).

use super::{memory_class, MemoryClass};
use cortex_core::{CellDescriptor, KnowledgeCellType};

fn memory(memory_type: Option<&str>) -> CellDescriptor {
    CellDescriptor {
        cell_type: KnowledgeCellType::Memory,
        memory_type: memory_type.map(str::to_owned),
        ..CellDescriptor::default()
    }
}

#[test]
fn decision_and_preference_are_semantic() {
    assert_eq!(
        memory_class(&memory(Some("decision"))),
        Some(MemoryClass::Semantic)
    );
    assert_eq!(
        memory_class(&memory(Some("preference"))),
        Some(MemoryClass::Semantic)
    );
    // Case-insensitive, matching MemoryType::from_str.
    assert_eq!(
        memory_class(&memory(Some("Decision"))),
        Some(MemoryClass::Semantic)
    );
}

#[test]
fn observations_workflow_errors_and_untyped_are_episodic() {
    for raw in [
        "observation",
        "workflow_result",
        "workflowresult",
        "error_log",
        "errorlog",
    ] {
        assert_eq!(
            memory_class(&memory(Some(raw))),
            Some(MemoryClass::Episodic),
            "{raw} should be episodic"
        );
    }
    // Untyped memory is an episodic session cell.
    assert_eq!(memory_class(&memory(None)), Some(MemoryClass::Episodic));
}

#[test]
fn non_memory_and_unknown_subtypes_are_unclassified() {
    // Not a memory cell.
    let fact = CellDescriptor {
        cell_type: KnowledgeCellType::Fact,
        memory_type: Some("decision".to_owned()),
        ..CellDescriptor::default()
    };
    assert_eq!(memory_class(&fact), None);
    // Unknown explicit subtype: never a consolidation candidate.
    assert_eq!(memory_class(&memory(Some("mystery"))), None);
}
