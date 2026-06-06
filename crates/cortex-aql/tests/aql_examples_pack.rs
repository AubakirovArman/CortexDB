use std::fs;
use std::path::{Path, PathBuf};

use cortex_aql::parse_aql;

#[test]
fn aql_examples_pack_parses_all_domain_examples() {
    let examples_root = repo_root().join("examples/aql");
    let mut files = Vec::new();
    collect_aql_files(&examples_root, &mut files);
    files.sort();

    assert_eq!(
        files.len(),
        16,
        "expected four examples for each of four domains"
    );
    assert_domain_present(&files, "investment_projects");
    assert_domain_present(&files, "legal_policies");
    assert_domain_present(&files, "support_tickets");
    assert_domain_present(&files, "technical_docs");

    for path in files {
        let query = fs::read_to_string(&path).expect("example query should be readable");
        parse_aql(&query).unwrap_or_else(|error| {
            panic!("{} should parse as AQL v0.4: {error:?}", path.display())
        });
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("cortex-aql crate should live under crates/")
        .to_path_buf()
}

fn collect_aql_files(root: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(root).expect("examples/aql should exist") {
        let path = entry.expect("example entry should be readable").path();
        if path.is_dir() {
            collect_aql_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "aql") {
            files.push(path);
        }
    }
}

fn assert_domain_present(files: &[PathBuf], domain: &str) {
    assert!(
        files.iter().any(|path| path
            .components()
            .any(|component| component.as_os_str() == domain)),
        "missing {domain} AQL examples"
    );
}
