pub mod checksum;
pub mod codec;
pub mod diagnostics;
pub mod reader;
pub mod record;
pub mod writer;
mod writer_append;
mod writer_rotation;
mod writer_runtime;
mod writer_state;

pub use codec::WalCodec;
pub use diagnostics::{WalDiagnostics, WalScanSummary};
pub use reader::{WalReader, WalScan};
pub use record::{
    AclogMagic, DecodedSection, DecodedWalRecord, SectionEntry, SectionTag, WalCodecVersion,
    WalFileHeader, WalFlags, WalRecord, WalRecordHeader, WalRecordType, WalSection, ACLOG_MAGIC,
    WAL_FILE_HEADER_LEN, WAL_FORMAT_VERSION, WAL_RECORD_HEADER_LEN, WAL_RECORD_MAGIC,
    WAL_SECTION_ENTRY_LEN,
};
pub use writer::{
    CommitAck, DurabilityMode, WalWriter, WalWriterHandle, WalWriterMetrics, WalWriterOptions,
};
