use std::sync::{Arc, Mutex};

use crate::StorageError;

#[derive(Clone, Debug)]
pub(super) struct WalWriterState {
    inner: Arc<Mutex<WalWriterStateInner>>,
}

#[derive(Clone, Debug, Default)]
struct WalWriterStateInner {
    last_error: Option<String>,
}

impl WalWriterState {
    pub(super) fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(WalWriterStateInner::default())),
        }
    }

    pub(super) fn record_error(&self, error: &StorageError) {
        self.record_message(error.to_string());
    }

    pub(super) fn record_message(&self, message: impl Into<String>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.last_error = Some(message.into());
        }
    }

    pub(super) fn closed_error(&self) -> StorageError {
        StorageError::wal_writer_closed(
            self.inner
                .lock()
                .ok()
                .and_then(|inner| inner.last_error.clone())
                .unwrap_or_else(|| "writer thread stopped without reporting a reason".to_owned()),
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::StorageError;

    use super::WalWriterState;

    #[test]
    fn closed_error_uses_last_recorded_reason() {
        let state = WalWriterState::new();
        state.record_message("disk full");
        assert_eq!(
            state.closed_error().to_string(),
            "WAL writer is closed: disk full"
        );

        state.record_error(&StorageError::Io(std::io::Error::other("device full")));
        assert_eq!(
            state.closed_error().to_string(),
            "WAL writer is closed: io error: device full"
        );
    }
}
