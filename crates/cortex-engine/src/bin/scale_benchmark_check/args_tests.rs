use super::Args;

#[test]
fn parse_payload_bytes_override() {
    let args = Args::parse(["--payload-bytes", "128"].into_iter().map(str::to_owned)).unwrap();
    assert_eq!(args.payload_bytes, Some(128));
}

#[test]
fn parse_rejects_zero_payload_bytes() {
    let error = match Args::parse(["--payload-bytes", "0"].into_iter().map(str::to_owned)) {
        Ok(_) => panic!("expected --payload-bytes=0 to fail"),
        Err(error) => error,
    };
    assert!(error.contains("--payload-bytes must be positive"));
}

#[test]
fn parse_direct_checkpoint_and_reopen_only() {
    let args = Args::parse(
        ["--direct-checkpoint", "--reopen-only"]
            .into_iter()
            .map(str::to_owned),
    )
    .unwrap();
    assert!(args.direct_checkpoint);
    assert!(args.reopen_only);
}
