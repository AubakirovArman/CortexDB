use super::super::wire::{
    push_optional_string_field, push_optional_u16_field, push_optional_u64_field,
    push_string_field, push_u8_field, read_u32,
};
use super::CellDescriptor;

const SECTION_MAGIC_V1: [u8; 8] = *b"ACDESC1\0";

impl CellDescriptor {
    pub fn encode_section_v1(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&SECTION_MAGIC_V1);
        push_string_field(&mut out, 1, &self.scope);
        push_string_field(&mut out, 2, &self.status);
        push_u8_field(&mut out, 3, self.cell_type.to_u8());
        push_optional_string_field(&mut out, 4, self.memory_type.as_deref());
        push_optional_u64_field(&mut out, 5, self.ttl_seconds);
        push_optional_u64_field(&mut out, 6, self.created_unix_seconds);
        push_optional_u16_field(&mut out, 7, self.source_trust_q16);
        push_optional_string_field(&mut out, 8, self.source.as_deref());
        push_optional_string_field(&mut out, 9, self.citation.as_deref());
        push_optional_string_field(&mut out, 10, self.content_hash.as_deref());
        push_optional_string_field(&mut out, 11, self.parent_id.as_deref());
        push_optional_string_field(&mut out, 12, self.valid_from.as_deref());
        push_optional_string_field(&mut out, 13, self.valid_to.as_deref());
        push_optional_string_field(&mut out, 14, self.source_id.as_deref());
        push_optional_string_field(&mut out, 15, self.source_url.as_deref());
        push_optional_string_field(&mut out, 16, self.document_id.as_deref());
        push_optional_u64_field(&mut out, 17, self.page.map(u64::from));
        push_optional_u64_field(&mut out, 18, self.row.map(u64::from));
        push_optional_string_field(&mut out, 19, self.cell_range.as_deref());
        push_optional_string_field(&mut out, 20, self.json_path.as_deref());
        push_optional_u16_field(&mut out, 21, self.confidence_q16);
        push_optional_string_field(&mut out, 22, self.session_id.as_deref());
        push_optional_string_field(&mut out, 23, self.session_kind.as_deref());
        out
    }

    pub fn decode_section_v1(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < SECTION_MAGIC_V1.len()
            || bytes[..SECTION_MAGIC_V1.len()] != SECTION_MAGIC_V1
        {
            return None;
        }

        let mut cursor = SECTION_MAGIC_V1.len();
        let mut descriptor = Self::default();
        while cursor < bytes.len() {
            let tag = *bytes.get(cursor)?;
            cursor += 1;
            let len = read_u32(bytes, &mut cursor)? as usize;
            let end = cursor.checked_add(len)?;
            let value = bytes.get(cursor..end)?;
            cursor = end;
            descriptor.apply_binary_field(tag, value)?;
        }
        Some(descriptor)
    }
}
