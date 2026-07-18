use super::transport::decode_value;
use super::AsyncCortexDbClient;
use crate::http::path;
use crate::{
    CellLookupResponse, CheckpointResponse, HealthResponse, PutCellResponse, SdkResult,
    StatsResponse, ValidationResponse, WriteBatchRequest, WriteBatchResponse,
};

impl AsyncCortexDbClient {
    pub async fn health(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/health").await
    }

    pub async fn health_response(&self) -> SdkResult<HealthResponse> {
        decode_value(self.health().await?)
    }

    pub async fn stats(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/stats").await
    }

    pub async fn stats_response(&self) -> SdkResult<StatsResponse> {
        decode_value(self.stats().await?)
    }

    pub async fn validate(&self) -> SdkResult<serde_json::Value> {
        self.get("/v1/validate").await
    }

    pub async fn validate_response(&self) -> SdkResult<ValidationResponse> {
        decode_value(self.validate().await?)
    }

    pub async fn put_cell(&self, cell_id: u64, payload: &str) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/cell", &[("cell_id", &cell_id.to_string())]),
            payload,
        )
        .await
    }

    pub async fn put_cell_response(
        &self,
        cell_id: u64,
        payload: &str,
    ) -> SdkResult<PutCellResponse> {
        decode_value(self.put_cell(cell_id, payload).await?)
    }

    pub async fn write_batch(&self, request: &WriteBatchRequest) -> SdkResult<serde_json::Value> {
        let body = serde_json::to_string(request)?;
        self.post("/v1/batch", &body).await
    }

    pub async fn write_batch_response(
        &self,
        request: &WriteBatchRequest,
    ) -> SdkResult<WriteBatchResponse> {
        decode_value(self.write_batch(request).await?)
    }

    pub async fn get_cell(&self, cell_id: u64) -> SdkResult<serde_json::Value> {
        self.get(&path("/v1/cell", &[("cell_id", &cell_id.to_string())]))
            .await
    }

    pub async fn get_cell_response(&self, cell_id: u64) -> SdkResult<CellLookupResponse> {
        decode_value(self.get_cell(cell_id).await?)
    }

    pub async fn tombstone_cell(&self, cell_id: u64) -> SdkResult<serde_json::Value> {
        self.delete(&path("/v1/cell", &[("cell_id", &cell_id.to_string())]))
            .await
    }

    pub async fn tombstone_cell_response(&self, cell_id: u64) -> SdkResult<PutCellResponse> {
        decode_value(self.tombstone_cell(cell_id).await?)
    }

    pub async fn flush(&self) -> SdkResult<serde_json::Value> {
        self.post("/v1/flush", "").await
    }

    pub async fn flush_response(&self) -> SdkResult<CheckpointResponse> {
        decode_value(self.flush().await?)
    }

    pub async fn compact(&self) -> SdkResult<serde_json::Value> {
        self.post("/v1/compact", "").await
    }

    pub async fn compact_response(&self) -> SdkResult<CheckpointResponse> {
        decode_value(self.compact().await?)
    }
}
