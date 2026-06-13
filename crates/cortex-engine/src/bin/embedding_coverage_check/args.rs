use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Args {
    pub(crate) expected_ids_path: Option<PathBuf>,
    pub(crate) expected_manifest_path: Option<PathBuf>,
    pub(crate) uuid_index_path: Option<PathBuf>,
    pub(crate) embeddings_path: PathBuf,
    pub(crate) output_path: Option<PathBuf>,
    pub(crate) retry_ids_output_path: Option<PathBuf>,
    pub(crate) expected_dimension: Option<usize>,
    pub(crate) expected_model: Option<String>,
    pub(crate) min_coverage_basis_points: u32,
}

impl Args {
    pub(crate) fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut expected_ids_path = None;
        let mut expected_manifest_path = None;
        let mut uuid_index_path = None;
        let mut embeddings_path = None;
        let mut output_path = None;
        let mut retry_ids_output_path = None;
        let mut expected_dimension = None;
        let mut expected_model = None;
        let mut min_coverage_basis_points = 9_950;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--expected-ids" => {
                    expected_ids_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--expected-manifest" => {
                    expected_manifest_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--uuid-index" => {
                    uuid_index_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--embeddings" => {
                    embeddings_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--output" => output_path = Some(PathBuf::from(next_value(&mut args, &arg)?)),
                "--retry-ids-output" => {
                    retry_ids_output_path = Some(PathBuf::from(next_value(&mut args, &arg)?));
                }
                "--expected-dimension" => {
                    expected_dimension = Some(parse_usize(&next_value(&mut args, &arg)?, &arg)?);
                }
                "--expected-model" => {
                    expected_model = Some(next_value(&mut args, &arg)?);
                }
                "--min-coverage-bps" => {
                    min_coverage_basis_points = parse_u32(&next_value(&mut args, &arg)?, &arg)?;
                }
                "--help" | "-h" => return Err(usage()),
                value => return Err(format!("unknown option: {value}")),
            }
        }
        let expected_source_count = usize::from(expected_ids_path.is_some())
            + usize::from(expected_manifest_path.is_some())
            + usize::from(uuid_index_path.is_some());
        if expected_source_count > 1 {
            return Err(
                "use only one of --expected-ids, --expected-manifest, or --uuid-index".to_owned(),
            );
        }
        Ok(Self {
            expected_ids_path,
            expected_manifest_path,
            uuid_index_path,
            embeddings_path: embeddings_path
                .ok_or_else(|| "--embeddings is required".to_owned())?,
            output_path,
            retry_ids_output_path,
            expected_dimension,
            expected_model,
            min_coverage_basis_points,
        })
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, option: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{option} requires a value"))
}

fn parse_usize(value: &str, option: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid {option} value {value:?}: {error}"))
}

fn parse_u32(value: &str, option: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|error| format!("invalid {option} value {value:?}: {error}"))
}

fn usage() -> String {
    "usage: embedding_coverage_check (--expected-ids PATH | --expected-manifest PATH | --uuid-index PATH) --embeddings PATH [--output PATH] [--retry-ids-output PATH] [--expected-dimension N] [--expected-model NAME] [--min-coverage-bps N]".to_owned()
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::Args;

    #[test]
    fn parses_expected_ids_mode() {
        let args = Args::parse([
            "--expected-ids".to_owned(),
            "ids.txt".to_owned(),
            "--embeddings".to_owned(),
            "vectors.jsonl".to_owned(),
            "--expected-dimension".to_owned(),
            "1024".to_owned(),
            "--min-coverage-bps".to_owned(),
            "9950".to_owned(),
        ])
        .unwrap();

        assert_eq!(args.expected_ids_path, Some(PathBuf::from("ids.txt")));
        assert_eq!(args.embeddings_path, PathBuf::from("vectors.jsonl"));
        assert_eq!(args.expected_dimension, Some(1024));
        assert_eq!(args.min_coverage_basis_points, 9_950);
    }

    #[test]
    fn rejects_both_expected_id_sources() {
        assert!(Args::parse([
            "--expected-ids".to_owned(),
            "ids.txt".to_owned(),
            "--uuid-index".to_owned(),
            "uuid.json".to_owned(),
            "--embeddings".to_owned(),
            "vectors.jsonl".to_owned(),
        ])
        .is_err());
    }

    #[test]
    fn parses_retry_ids_output() {
        let args = Args::parse([
            "--expected-ids".to_owned(),
            "ids.txt".to_owned(),
            "--embeddings".to_owned(),
            "vectors.jsonl".to_owned(),
            "--retry-ids-output".to_owned(),
            "retry.txt".to_owned(),
            "--expected-model".to_owned(),
            "bge-m3".to_owned(),
        ])
        .unwrap();

        assert_eq!(args.retry_ids_output_path, Some(PathBuf::from("retry.txt")));
        assert_eq!(args.expected_model, Some("bge-m3".to_owned()));
    }

    #[test]
    fn parses_expected_manifest_mode() {
        let args = Args::parse([
            "--expected-manifest".to_owned(),
            "manifest.jsonl".to_owned(),
            "--embeddings".to_owned(),
            "vectors.jsonl".to_owned(),
        ])
        .unwrap();

        assert_eq!(
            args.expected_manifest_path,
            Some(PathBuf::from("manifest.jsonl"))
        );
    }
}
