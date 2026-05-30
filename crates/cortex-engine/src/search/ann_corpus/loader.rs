use std::collections::{BTreeMap, BTreeSet};

use crate::error::{EngineError, EngineResult};

use super::{AnnCorpusGroundTruth, AnnCorpusQuery, AnnCorpusVector};

pub(super) struct AnnCorpus {
    pub(super) vectors: BTreeMap<u32, Vec<i16>>,
    pub(super) queries: Vec<AnnCorpusQuery>,
    pub(super) ground_truth: BTreeMap<String, Vec<u32>>,
    pub(super) dimension: usize,
}

pub(super) fn load_ann_corpus(
    vectors_jsonl: &str,
    queries_jsonl: &str,
    ground_truth_jsonl: &str,
) -> EngineResult<AnnCorpus> {
    let mut dimension = None;
    let vectors = parse_vectors(vectors_jsonl, &mut dimension)?;
    let queries = parse_queries(queries_jsonl, &mut dimension)?;
    let ground_truth = parse_ground_truth(ground_truth_jsonl)?;
    validate_references(&vectors, &queries, &ground_truth)?;
    Ok(AnnCorpus {
        vectors,
        queries,
        ground_truth,
        dimension: dimension.unwrap_or(0),
    })
}

fn parse_vectors(
    input: &str,
    dimension: &mut Option<usize>,
) -> EngineResult<BTreeMap<u32, Vec<i16>>> {
    let mut vectors = BTreeMap::new();
    for (line_index, line) in jsonl_lines(input) {
        let entry: AnnCorpusVector = parse_line(line_index, line)?;
        if entry.candidate == 0 {
            return Err(invalid_corpus(format!(
                "vectors line {line_index}: candidate id 0"
            )));
        }
        validate_dimension(&entry.vector, dimension, "vectors", line_index)?;
        if vectors.insert(entry.candidate, entry.vector).is_some() {
            return Err(invalid_corpus(format!(
                "vectors line {line_index}: duplicate candidate {}",
                entry.candidate
            )));
        }
    }
    if vectors.is_empty() {
        return Err(invalid_corpus("vectors file is empty"));
    }
    Ok(vectors)
}

fn parse_queries(input: &str, dimension: &mut Option<usize>) -> EngineResult<Vec<AnnCorpusQuery>> {
    let mut queries = Vec::new();
    let mut names = BTreeSet::new();
    for (line_index, line) in jsonl_lines(input) {
        let entry: AnnCorpusQuery = parse_line(line_index, line)?;
        if entry.name.trim().is_empty() {
            return Err(invalid_corpus(format!(
                "queries line {line_index}: empty query name"
            )));
        }
        if entry.limit == 0 {
            return Err(invalid_corpus(format!(
                "queries line {line_index}: query limit is zero"
            )));
        }
        validate_dimension(&entry.vector, dimension, "queries", line_index)?;
        if !names.insert(entry.name.clone()) {
            return Err(invalid_corpus(format!(
                "queries line {line_index}: duplicate query {}",
                entry.name
            )));
        }
        queries.push(entry);
    }
    if queries.is_empty() {
        return Err(invalid_corpus("queries file is empty"));
    }
    Ok(queries)
}

fn parse_ground_truth(input: &str) -> EngineResult<BTreeMap<String, Vec<u32>>> {
    let mut ground_truth = BTreeMap::new();
    for (line_index, line) in jsonl_lines(input) {
        let entry: AnnCorpusGroundTruth = parse_line(line_index, line)?;
        if entry.name.trim().is_empty() {
            return Err(invalid_corpus(format!(
                "ground-truth line {line_index}: empty query name"
            )));
        }
        if entry.candidates.is_empty() {
            return Err(invalid_corpus(format!(
                "ground-truth line {line_index}: empty candidates"
            )));
        }
        if entry.candidates.contains(&0) {
            return Err(invalid_corpus(format!(
                "ground-truth line {line_index}: candidate id 0"
            )));
        }
        if ground_truth
            .insert(entry.name.clone(), entry.candidates)
            .is_some()
        {
            return Err(invalid_corpus(format!(
                "ground-truth line {line_index}: duplicate query {}",
                entry.name
            )));
        }
    }
    if ground_truth.is_empty() {
        return Err(invalid_corpus("ground-truth file is empty"));
    }
    Ok(ground_truth)
}

fn validate_references(
    vectors: &BTreeMap<u32, Vec<i16>>,
    queries: &[AnnCorpusQuery],
    ground_truth: &BTreeMap<String, Vec<u32>>,
) -> EngineResult<()> {
    for query in queries {
        let Some(truth) = ground_truth.get(&query.name) else {
            return Err(invalid_corpus(format!(
                "missing ground truth for query {}",
                query.name
            )));
        };
        for candidate in truth {
            if !vectors.contains_key(candidate) {
                return Err(invalid_corpus(format!(
                    "ground truth for query {} references unknown candidate {}",
                    query.name, candidate
                )));
            }
        }
    }
    Ok(())
}

fn validate_dimension(
    vector: &[i16],
    dimension: &mut Option<usize>,
    file: &str,
    line: usize,
) -> EngineResult<()> {
    if vector.is_empty() {
        return Err(invalid_corpus(format!("{file} line {line}: empty vector")));
    }
    match dimension {
        Some(expected) if *expected != vector.len() => Err(invalid_corpus(format!(
            "{file} line {line}: vector dimension {}, expected {}",
            vector.len(),
            expected
        ))),
        Some(_) => Ok(()),
        None => {
            *dimension = Some(vector.len());
            Ok(())
        }
    }
}

fn parse_line<T>(line_index: usize, line: &str) -> EngineResult<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(line)
        .map_err(|error| invalid_corpus(format!("line {line_index}: {error}")))
}

fn jsonl_lines(input: &str) -> impl Iterator<Item = (usize, &str)> {
    input
        .lines()
        .enumerate()
        .map(|(index, line)| (index + 1, line.trim()))
        .filter(|(_, line)| !line.is_empty() && !line.starts_with('#'))
}

fn invalid_corpus(message: impl Into<String>) -> EngineError {
    EngineError::InvalidAnnCorpus(message.into())
}
