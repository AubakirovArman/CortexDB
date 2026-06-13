export interface HealthResponse {
  status: string;
  version: string;
  server_version: string;
}

export interface StatsResponse {
  current_seq: number;
  checkpoint_seq: number;
  live_segments: number;
  retired_segments: number;
  memtable_cells: number;
  memtable_versions: number;
  memtable_payload_bytes: number;
  estimated_memtable_bytes: number;
  estimated_index_bytes: number;
  estimated_context_pack_bytes: number;
  estimated_total_memory_bytes: number;
  live_segment_bytes: number;
  retired_segment_bytes: number;
  total_segment_bytes: number;
  durable_storage_bytes: number;
  live_segment_payload_bytes: number;
  logical_payload_bytes: number;
  space_amplification_q16: number;
  write_amplification_q16: number;
  compaction_pressure_q16: number;
  wal_size_bytes: number;
  wal_writer_records: number;
  wal_writer_bytes: number;
  wal_writer_fsyncs: number;
  wal_writer_batches: number;
}

export interface ValidationResponse {
  ok: boolean;
  manifest_ok: boolean;
  wal_ok: boolean;
  live_segments_checked: number;
  bitmap_indexes_checked: number;
  lexical_indexes_checked: number;
  vector_indexes_checked: number;
  hnsw_graphs_checked: number;
  cells_checked: number;
  wal_records_checked: number;
  wal_safe_truncate_offset: number;
  errors: string[];
}

export interface PutCellResponse {
  seq: number;
  cell_id: number;
}

export interface CellResponse {
  cell_id: number;
  payload: string;
}

export interface CellLookupResponse {
  cell: CellResponse | null;
}
