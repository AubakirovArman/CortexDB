use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;

use super::types::{CorpusSynonymCandidate, CorpusSynonymDictionary, CorpusSynonymEntry};

const ACSYN_MAGIC: &str = "CORTEXDB_ACSYN_V1";
pub(super) const ACSYN_FILE_NAME: &str = "corpus.acsyn";

pub fn write_acsyn_dictionary(
    path: &Path,
    dictionary: &CorpusSynonymDictionary,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_path = path.with_extension("acsyn.tmp");
    {
        let mut file = fs::File::create(&tmp_path)?;
        writeln!(file, "{ACSYN_MAGIC}")?;
        for entry in &dictionary.entries {
            write!(file, "{}\t{}", entry.term, entry.document_frequency)?;
            for synonym in &entry.synonyms {
                write!(
                    file,
                    "\t{}:{}:{}",
                    synonym.term, synonym.score_q16, synonym.cooccurrence_count
                )?;
            }
            writeln!(file)?;
        }
        file.sync_all()?;
    }
    fs::rename(&tmp_path, path)?;
    if let Some(parent) = path.parent() {
        fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

pub fn read_acsyn_dictionary(path: &Path) -> std::io::Result<CorpusSynonymDictionary> {
    let file = fs::File::open(path)?;
    let mut lines = BufReader::new(file).lines();
    let magic = lines.next().transpose()?.unwrap_or_default();
    if magic.trim() != ACSYN_MAGIC {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid ACSYN magic",
        ));
    }
    let mut entries = Vec::new();
    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let mut fields = line.split('\t');
        let term = fields.next().unwrap_or_default().to_owned();
        let document_frequency = fields
            .next()
            .and_then(|value| value.parse::<u32>().ok())
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid ACSYN df")
            })?;
        let mut synonyms = Vec::new();
        for field in fields {
            let parts = field.split(':').collect::<Vec<_>>();
            if parts.len() != 3 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "invalid ACSYN synonym entry",
                ));
            }
            synonyms.push(CorpusSynonymCandidate {
                term: parts[0].to_owned(),
                score_q16: parts[1].parse::<u16>().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid ACSYN score")
                })?,
                cooccurrence_count: parts[2].parse::<u32>().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid ACSYN count")
                })?,
            });
        }
        entries.push(CorpusSynonymEntry {
            term,
            document_frequency,
            synonyms,
        });
    }
    Ok(CorpusSynonymDictionary { entries })
}
