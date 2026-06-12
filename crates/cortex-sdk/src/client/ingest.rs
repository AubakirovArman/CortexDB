use super::transport::decode_value;
use super::CortexDbClient;
use crate::http::path;
use crate::{DeleteJobResponse, IngestResponse, IngestionJobResponse, SdkResult};

impl CortexDbClient {
    pub fn ingest_text(
        &self,
        scope: &str,
        source: &str,
        text: &str,
    ) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/ingest/text", &[("scope", scope), ("source", source)]),
            text,
        )
    }

    pub fn ingest_text_response(
        &self,
        scope: &str,
        source: &str,
        text: &str,
    ) -> SdkResult<IngestResponse> {
        decode_value(self.ingest_text(scope, source, text)?)
    }

    pub fn ingest_json(
        &self,
        scope: &str,
        source: &str,
        document: &str,
    ) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/ingest/json", &[("scope", scope), ("source", source)]),
            document,
        )
    }

    pub fn ingest_json_response(
        &self,
        scope: &str,
        source: &str,
        document: &str,
    ) -> SdkResult<IngestResponse> {
        decode_value(self.ingest_json(scope, source, document)?)
    }

    pub fn ingest_csv(
        &self,
        scope: &str,
        source: &str,
        document: &str,
    ) -> SdkResult<serde_json::Value> {
        self.post(
            &path("/v1/ingest/csv", &[("scope", scope), ("source", source)]),
            document,
        )
    }

    pub fn ingest_csv_response(
        &self,
        scope: &str,
        source: &str,
        document: &str,
    ) -> SdkResult<IngestResponse> {
        decode_value(self.ingest_csv(scope, source, document)?)
    }

    pub fn ingestion_job(&self, job_id: u64) -> SdkResult<serde_json::Value> {
        self.get(&format!("/v1/ingest/jobs/{job_id}"))
    }

    pub fn ingestion_job_response(&self, job_id: u64) -> SdkResult<IngestionJobResponse> {
        decode_value(self.ingestion_job(job_id)?)
    }

    pub fn delete_ingestion_job(&self, job_id: u64) -> SdkResult<DeleteJobResponse> {
        decode_value(self.delete(&format!("/v1/ingest/jobs/{job_id}"))?)
    }

    pub fn retry_ingestion_job(&self, job_id: u64) -> SdkResult<IngestionJobResponse> {
        decode_value(self.post(&format!("/v1/ingest/jobs/{job_id}/retry"), "")?)
    }
}
