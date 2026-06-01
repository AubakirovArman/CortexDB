use super::run;

#[test]
fn context_json_contains_cells_budget_anomalies() {
    let path = unique_path("cortexdb-cli-context-json");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha budget".to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "context".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments AND status = "ready" LIMIT 10 CANDIDATES;"#.to_owned(),
        "--json".to_owned(),
    ])
    .unwrap();
    assert!(output.contains(r#""schema_version":"context_pack.v1""#));
    assert!(output.contains(r#""token_budget_tokens""#));
    assert!(output.contains(r#""cells""#));
    assert!(output.contains(r#""anomalies""#));
    assert!(output.contains(r#""cell_id":1"#));
    assert!(output.contains(r#""citation":"doc-a""#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn context_json_escapes_payload() {
    let path = unique_path("cortexdb-cli-context-escape");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\nsource=doc-a\nalpha \"budget\"\nwith newlines"
            .to_owned(),
    ])
    .unwrap();

    let output = run(vec![
        "cortexdb".to_owned(),
        "context".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"RETRIEVE CONTEXT FOR TASK "budget" IN BRAIN investment_projects WHERE space = project:investments LIMIT 10 CANDIDATES;"#.to_owned(),
        "--json".to_owned(),
    ])
    .unwrap();
    assert!(output.contains(r#"\"budget\""#));
    assert!(output.contains(r#"\nwith newlines"#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn verify_json_supported() {
    let path = unique_path("cortexdb-cli-verify-supported");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "remember".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"REMEMBER "ABC budget approved" IN SCOPE project:investments AS TYPE decision TTL 60 SECONDS;"#.to_owned(),
    ])
    .unwrap();

    let verify = run(vec![
        "cortexdb".to_owned(),
        "verify".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"VERIFY FACT "ABC budget approved" IN BRAIN investment_projects;"#.to_owned(),
        "--json".to_owned(),
    ])
    .unwrap();
    assert!(verify.contains(r#""verdict":"supported""#));
    assert!(verify.contains(r#""supporting""#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn verify_json_mixed_evidence_with_numeric_conflict() {
    let path = unique_path("cortexdb-cli-verify-mixed");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\ntype=fact\nsource=report_q1.pdf#page=3\nproject=Solar Plant\nmetric=budget\nvalue=1.2\ncurrency=KZT\n\nSolar Plant report highlights. The total approved budget for the Solar Plant project in first quarter is 1.2B KZT.".to_owned(),
    ])
    .unwrap();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "2".to_owned(),
        "scope=project:investments\nstatus=ready\ntype=fact\nsource=report_q2.pdf#page=5\nproject=Solar Plant\nmetric=budget\nvalue=1400000000\ncurrency=KZT\n\nSolar Plant Q2 update. Following recent expansions, the budget for Solar Plant has been adjusted to 1.4B KZT.".to_owned(),
    ])
    .unwrap();

    let verify = run(vec![
        "cortexdb".to_owned(),
        "verify".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;"#.to_owned(),
        "--json".to_owned(),
    ])
    .unwrap();
    println!("VERIFY RESPONSE: {}", verify);
    assert!(verify.contains(r#""verdict":"mixed_evidence""#));
    assert!(verify.contains(r#""supporting""#));
    assert!(verify.contains(r#""contradicting""#));
    assert!(verify.contains(r#""numeric_conflicts""#));
    assert!(verify.contains(r#""metric":"budget""#));
    assert!(verify.contains(r#""left":"1.2B KZT""#));
    assert!(verify.contains(r#""right":"1.4B KZT""#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn verify_json_does_not_infer_billions_from_decimal_percent() {
    let path = unique_path("cortexdb-cli-verify-percent");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\ntype=fact\nsource=risk.txt\nmetric=risk\nvalue=2\ncurrency=%\n\nRisk changed to 2%.".to_owned(),
    ])
    .unwrap();

    let verify = run(vec![
        "cortexdb".to_owned(),
        "verify".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"VERIFY FACT "Project risk is 1.2%" IN BRAIN investment_projects;"#.to_owned(),
        "--json".to_owned(),
    ])
    .unwrap();

    assert!(verify.contains(r#""left":"1.2 %""#));
    assert!(!verify.contains(r#""left":"1.2B"#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn verify_json_insufficient_evidence() {
    let path = unique_path("cortexdb-cli-verify-insufficient");
    let path_arg = path.to_string_lossy().into_owned();

    let verify = run(vec![
        "cortexdb".to_owned(),
        "verify".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"VERIFY FACT "Some random claim that has no evidence" IN BRAIN investment_projects;"#
            .to_owned(),
        "--json".to_owned(),
    ])
    .unwrap();
    assert!(verify.contains(r#""verdict":"insufficient""#));

    let _ = std::fs::remove_dir_all(path);
}

#[test]
fn verify_exports_markdown_and_audit_formats() {
    let path = unique_path("cortexdb-cli-verify-export");
    let path_arg = path.to_string_lossy().into_owned();
    run(vec![
        "cortexdb".to_owned(),
        "put".to_owned(),
        path_arg.clone(),
        "1".to_owned(),
        "scope=project:investments\nstatus=ready\ntype=fact\nsource=report.pdf\nsource_trust_q16=60000\nmetric=budget\n\nSolar Plant budget changed to 1.4B KZT.".to_owned(),
    ])
    .unwrap();

    let markdown = run(vec![
        "cortexdb".to_owned(),
        "verify".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;"#.to_owned(),
        "--format".to_owned(),
        "markdown".to_owned(),
    ])
    .unwrap();
    assert!(markdown.starts_with("# CortexDB Verification Report"));
    assert!(markdown.contains("## Numeric Conflicts"));

    let audit = run(vec![
        "cortexdb".to_owned(),
        "verify".to_owned(),
        path_arg.clone(),
        "project:investments".to_owned(),
        r#"VERIFY FACT "Solar Plant budget is 1.2B KZT" IN BRAIN investment_projects;"#.to_owned(),
        "--format".to_owned(),
        "audit".to_owned(),
    ])
    .unwrap();
    assert!(audit.starts_with("CortexDB Verification Audit v1"));
    assert!(audit.contains("numeric_conflict.0.metric=budget"));

    let _ = std::fs::remove_dir_all(path);
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
