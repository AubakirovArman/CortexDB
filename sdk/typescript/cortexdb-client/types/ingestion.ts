import type { JsonObject } from "../errors";

export interface IngestResponse {
  rows_ingested: number;
  chunks_ingested: number;
  facts_ingested: number;
  first_cell_id: number | null;
  job_id: number | null;
  validation_report: JsonObject;
}

export type IngestionJobStatus = "queued" | "running" | "completed" | "failed" | "cancelled";

export interface IngestionJobResponse {
  job_id: number;
  label: string;
  status: IngestionJobStatus;
  total_items: number | null;
  completed_items: number;
  failed_items: number;
  last_cell_id: number | null;
  message: string | null;
  retry_count: number;
  max_retries: number;
}

export interface DeleteJobResponse {
  deleted: boolean;
}
