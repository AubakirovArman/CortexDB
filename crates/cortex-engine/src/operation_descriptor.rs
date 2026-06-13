use cortex_core::CellDescriptor;
use cortex_storage::wal::{SectionTag, WalRecord, WalSection};

use crate::operation::DbOperation;

pub(crate) fn descriptor_from_operation_with_metadata(
    operation: &DbOperation,
    metadata: &[u8],
) -> Option<CellDescriptor> {
    let mut descriptor = descriptor_from_operation_payload(operation)?;
    if !metadata.is_empty() {
        let metadata_descriptor = CellDescriptor::from_metadata_section_lossy(metadata);
        descriptor.overlay_metadata_descriptor(&metadata_descriptor);
    }
    Some(descriptor)
}

pub(crate) fn wal_descriptor_bytes_from_operation_with_metadata(
    operation: &DbOperation,
    metadata: &[u8],
) -> Vec<u8> {
    descriptor_from_operation_with_metadata(operation, metadata)
        .unwrap_or_else(|| CellDescriptor::from_metadata_section_lossy(metadata))
        .encode_section_v1()
}

pub(crate) fn upsert_wal_descriptor_section(record: &mut WalRecord, descriptor_bytes: Vec<u8>) {
    if let Some(section) = record
        .sections
        .iter_mut()
        .find(|section| section.tag == SectionTag::CellDescriptor)
    {
        section.data = descriptor_bytes;
    } else {
        record.sections.push(WalSection::new(
            SectionTag::CellDescriptor,
            descriptor_bytes,
        ));
    }
}

fn descriptor_from_operation_payload(operation: &DbOperation) -> Option<CellDescriptor> {
    match operation {
        DbOperation::PutCell { payload, .. } | DbOperation::PatchCell { payload, .. } => {
            Some(CellDescriptor::from_payload_lossy(payload))
        }
        DbOperation::TombstoneCell { .. } => None,
    }
}
