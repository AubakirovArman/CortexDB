use crate::error::{EngineError, EngineResult};

pub(super) fn usize_to_u64(value: usize, message: &'static str) -> EngineResult<u64> {
    u64::try_from(value).map_err(|_| slo_invariant(message))
}

pub(super) fn require_hash(name: &str, value: &str) -> EngineResult<()> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return fail(format!("{name} must be 64 hex chars"));
    }
    Ok(())
}

pub(super) fn require_https_url(name: &str, value: &str) -> EngineResult<()> {
    let trimmed = value.trim();
    if trimmed.is_empty() || !trimmed.starts_with("https://") {
        return fail(format!("{name} must be a non-empty https URL"));
    }
    Ok(())
}

pub(super) fn required_label(name: &str, value: &str) -> EngineResult<()> {
    if value.trim().is_empty() {
        return fail(format!("{name} is required"));
    }
    Ok(())
}

pub(super) fn fail<T>(message: impl Into<String>) -> EngineResult<T> {
    Err(slo_invariant(message))
}

pub(super) fn slo_invariant(message: impl Into<String>) -> EngineError {
    EngineError::StorageInvariant(message.into())
}
