use super::run;

#[test]
fn empty_ingestion_commands_return_null_first_cell_id() {
    let path = unique_path("cortexdb-cli-empty-ingest-db");
    let input_dir = unique_path("cortexdb-cli-empty-ingest-inputs");
    std::fs::create_dir_all(&input_dir).unwrap();
    let path_arg = path.to_string_lossy().into_owned();

    let text_file = input_dir.join("empty.txt");
    let json_file = input_dir.join("empty.json");
    let csv_file = input_dir.join("empty.csv");
    std::fs::write(&text_file, "").unwrap();
    std::fs::write(&json_file, "{}").unwrap();
    std::fs::write(&csv_file, "").unwrap();

    let text_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-text".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        text_file.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert_eq!(text_output, "ingested_chunks=0 first_cell_id=null");

    let json_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-json".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        json_file.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert_eq!(json_output, "ingested_facts=0 first_cell_id=null");

    let csv_output = run(vec![
        "cortexdb".to_owned(),
        "ingest-csv".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        csv_file.to_string_lossy().into_owned(),
    ])
    .unwrap();
    assert_eq!(csv_output, "ingested_rows=0 first_cell_id=null");

    let _ = std::fs::remove_dir_all(path);
    let _ = std::fs::remove_dir_all(input_dir);
}

fn unique_path(prefix: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "{prefix}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}
