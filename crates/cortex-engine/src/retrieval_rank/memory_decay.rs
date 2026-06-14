use crate::query::CellMetadata;

pub(super) fn memory_decay_q16(metadata: &CellMetadata, now_unix_seconds: u64) -> u64 {
    if metadata.cell_type != "memory" {
        return u64::from(u16::MAX);
    }
    let Some(ttl) = metadata.ttl_seconds else {
        return u64::from(u16::MAX);
    };
    let Some(created) = metadata.created_unix_seconds else {
        return u64::from(u16::MAX);
    };
    let expires_at = created.saturating_add(ttl);
    if ttl == 0 || now_unix_seconds >= expires_at {
        return 0;
    }
    if now_unix_seconds <= created {
        return u64::from(u16::MAX);
    }
    let remaining = expires_at - now_unix_seconds;
    ((u128::from(remaining) * u128::from(u16::MAX) + u128::from(ttl / 2)) / u128::from(ttl)) as u64
}
