use super::transport::decode_value;
use super::CortexDbClient;
use crate::http::path;
use crate::{
    CellLookupResponse, HealthResponse, PutCellResponse, SdkResult, StatsResponse,
    ValidationResponse,
};

impl CortexDbClient {
    pub fn health(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/health")
    }

    pub fn health_response(&self) -> SdkResult<HealthResponse> {
        decode_value(self.health()?)
    }

    pub fn stats(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/stats")
    }

    pub fn stats_response(&self) -> SdkResult<StatsResponse> {
        decode_value(self.stats()?)
    }

    pub fn validate(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/validate")
    }

    pub fn validate_response(&self) -> SdkResult<ValidationResponse> {
        decode_value(self.validate()?)
    }

    pub fn put_cell(&self, cell_id: u64, payload: &str) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/cell", &[("cell_id", &cell_id.to_string())]),
            payload,
        )
    }

    pub fn put_cell_response(&self, cell_id: u64, payload: &str) -> SdkResult<PutCellResponse> {
        decode_value(self.put_cell(cell_id, payload)?)
    }

    pub fn get_cell(&self, cell_id: u64) -> SdkResult<serde_json::Value> {
        self.get(&path("/v1/cell", &[("cell_id", &cell_id.to_string())]))
    }

    pub fn get_cell_response(&self, cell_id: u64) -> SdkResult<CellLookupResponse> {
        decode_value(self.get_cell(cell_id)?)
    }

    pub fn tombstone_cell(&self, cell_id: u64) -> SdkResult<serde_json::Value> {
        self.delete(&path("/v1/cell", &[("cell_id", &cell_id.to_string())]))
    }

    pub fn flush(&self) -> SdkResult<serde_json::Value> {
        self.post("/v1/flush", "")
    }

    pub fn compact(&self) -> SdkResult<serde_json::Value> {
        self.post("/v1/compact", "")
    }
}
