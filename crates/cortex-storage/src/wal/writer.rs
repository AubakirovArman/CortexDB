#[derive(Clone, Debug)]
pub struct WalWriterHandle;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitAck {
    pub durable_lsn: u64,
}
