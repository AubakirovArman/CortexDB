use crate::{handle_http_with_options, ServerOptions};

#[test]
fn malformed_auth_tokens_file_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = dir.path().join("auth.tokens");
    std::fs::write(&token_file, "data-token-without-role-prefix\n").unwrap();
    let options = ServerOptions {
        auth_tokens_file: Some(token_file),
        ..Default::default()
    };

    let response = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer anything\r\n\r\n",
        &options,
    );

    assert!(
        !response.contains("200 OK"),
        "malformed token file must not authenticate requests: {response}"
    );
}

#[test]
fn empty_auth_tokens_file_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let token_file = dir.path().join("auth.tokens");
    std::fs::write(&token_file, "# no active tokens\n\n").unwrap();
    let options = ServerOptions {
        auth_tokens_file: Some(token_file),
        ..Default::default()
    };

    let response = handle_http_with_options(
        dir.path(),
        "GET /v1/health HTTP/1.1\r\nAuthorization: Bearer anything\r\n\r\n",
        &options,
    );

    assert!(
        !response.contains("200 OK"),
        "empty token file must not authenticate requests: {response}"
    );
}
