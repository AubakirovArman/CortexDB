use std::collections::BTreeMap;
use std::path::Path;

use crate::error::StorageResult;

mod codec;
mod reader;

pub use reader::VectorIndexReader;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VectorIndex {
    pub vectors: BTreeMap<u32, Vec<i16>>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VectorDimensionReport {
    pub vector_count: usize,
    pub expected_dimension: Option<usize>,
    pub mismatched_vectors: usize,
    pub zero_dimension_vectors: usize,
}

impl VectorIndex {
    pub fn write(&self, path: impl AsRef<Path>) -> StorageResult<()> {
        codec::write(self, path.as_ref())
    }

    pub fn read(path: impl AsRef<Path>) -> StorageResult<Self> {
        codec::read(path.as_ref())
    }

    pub fn dimension_report(&self) -> VectorDimensionReport {
        let expected_dimension = self.vectors.values().next().map(Vec::len);
        let mut report = VectorDimensionReport {
            vector_count: self.vectors.len(),
            expected_dimension,
            ..VectorDimensionReport::default()
        };
        for vector in self.vectors.values() {
            if vector.is_empty() {
                report.zero_dimension_vectors += 1;
            }
            if expected_dimension.is_some_and(|dimension| vector.len() != dimension) {
                report.mismatched_vectors += 1;
            }
        }
        report
    }
}

impl VectorDimensionReport {
    pub fn is_valid(self) -> bool {
        self.mismatched_vectors == 0 && self.zero_dimension_vectors == 0
    }

    pub fn summary(self) -> String {
        format!(
            "vectors={} expected_dimension={} mismatched_vectors={} zero_dimension_vectors={}",
            self.vector_count,
            self.expected_dimension
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned()),
            self.mismatched_vectors,
            self.zero_dimension_vectors
        )
    }
}
