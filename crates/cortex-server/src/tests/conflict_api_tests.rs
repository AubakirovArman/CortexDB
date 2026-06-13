use crate::handle_http;

#[test]
fn v1_conflicts_returns_incremental_conflict_index() {
    let dir = tempfile::tempdir().unwrap();
    let first = concat!(
        "POST /v1/cell?cell_id=1 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=verified\ntype=fact\n",
        "source=ifc\nsource_trust_q16=50000\n\n",
        "project=Mirny\nmetric=budget\nvalue=1.2B KZT\n",
        "Mirny budget is 1.2B KZT"
    );
    assert!(handle_http(dir.path(), first).contains(r#""seq":1"#));
    let second = concat!(
        "POST /v1/cell?cell_id=2 HTTP/1.1\r\n\r\n",
        "scope=project:investments\nstatus=verified\ntype=fact\n",
        "source=world_bank\nsource_trust_q16=50000\n\n",
        "project=Mirny\nmetric=budget\nvalue=1.4B KZT\n",
        "Mirny budget is 1.4B KZT"
    );
    assert!(handle_http(dir.path(), second).contains(r#""seq":2"#));

    let response = handle_http(
        dir.path(),
        "GET /v1/conflicts?scope=project:investments HTTP/1.1\r\n\r\n",
    );
    assert!(response.contains(r#""schema_version":"conflict_index.v1""#));
    assert!(response.contains(r#""conflict_count":1"#));
    assert!(response.contains(r#""metric":"budget""#));
    assert!(response.contains(r#""entity":"Mirny""#));
}
