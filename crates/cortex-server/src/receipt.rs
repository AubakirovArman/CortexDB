use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use cortex_crypto::{blake3_256_domain, hex_lower};
use cortex_engine::{
    accountability::append_transparency_log_record,
    accountability::AccountabilityReceiptHeaderSigner, canonical::canonical_json_bytes,
    ContextPackReceiptEvidence, VerificationReport,
};
use serde_json::Value;

use crate::audit_chain::{tail as audit_chain_tail, AUDIT_CHAIN_ZERO_HASH};
use crate::config::{ReceiptSigningKey, ServerOptions};
use crate::database_identity::validate_database_instance_id;
use crate::receipt_signer::ReceiptExternalSigner;
use crate::responses::RouterError;

const ACCOUNTABILITY_RECEIPT_AUDIT_HASH_DOMAIN: &str =
    "cortexdb.audit.accountability_receipt_hash.v1";

#[derive(Clone, Debug)]
pub(crate) struct ReceiptEmissionContext {
    signer: ReceiptSigner,
    db_instance_id: String,
    audit_log_path: Option<PathBuf>,
    transparency_log_path: Option<PathBuf>,
}

impl ReceiptEmissionContext {
    pub(crate) fn from_options(options: &ServerOptions) -> Result<Option<Self>, RouterError> {
        let transparency_log_path =
            transparency_log_path_from_env().map_err(RouterError::Internal)?;
        let signer = match (
            options.receipt_signing_key.as_ref(),
            options.receipt_external_signer.as_ref(),
        ) {
            (None, None) => {
                if transparency_log_path.is_some() {
                    return Err(RouterError::Internal(
                        "CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE requires configured receipt signing"
                            .to_owned(),
                    ));
                }
                return Ok(None);
            }
            (Some(_), Some(_)) => {
                return Err(RouterError::Internal(
                    "set only one receipt signer mode: local seed or external signer".to_owned(),
                ));
            }
            (Some(signing_key), None) => ReceiptSigner::Local(signing_key.clone()),
            (None, Some(external_signer)) => ReceiptSigner::External(external_signer.clone()),
        };
        let db_instance_id = options.db_instance_id.as_deref().ok_or_else(|| {
            RouterError::Internal(
                "database instance identity is required for receipt signing".to_owned(),
            )
        })?;
        validate_database_instance_id(db_instance_id).map_err(RouterError::Internal)?;
        Ok(Some(Self {
            signer,
            db_instance_id: db_instance_id.to_owned(),
            audit_log_path: audit_log_path_from_options(options),
            transparency_log_path,
        }))
    }

    pub(crate) fn sign(
        &self,
        evidence: &ContextPackReceiptEvidence,
        verification_report: Option<&VerificationReport>,
    ) -> Result<Value, RouterError> {
        let audit_chain_head = self.audit_chain_head()?;
        let receipt = evidence.signed_receipt_value_with_signer(
            verification_report,
            &self.db_instance_id,
            current_unix_seconds(),
            &audit_chain_head,
            &self.signer,
        )?;
        if let Some(path) = &self.transparency_log_path {
            append_transparency_log_record(path, &receipt)?;
        }
        Ok(receipt)
    }

    fn audit_chain_head(&self) -> Result<String, RouterError> {
        let Some(path) = &self.audit_log_path else {
            return Ok(AUDIT_CHAIN_ZERO_HASH.to_owned());
        };
        audit_chain_tail(path)
            .map(|(_, head)| head)
            .map_err(|error| {
                RouterError::Internal(format!("audit chain head unavailable: {error}"))
            })
    }
}

#[derive(Clone, Debug)]
enum ReceiptSigner {
    Local(ReceiptSigningKey),
    External(ReceiptExternalSigner),
}

impl AccountabilityReceiptHeaderSigner for ReceiptSigner {
    fn key_id(&self) -> &str {
        match self {
            Self::Local(key) => key.key_id(),
            Self::External(signer) => signer.key_id(),
        }
    }

    fn public_key_hex(&self) -> String {
        match self {
            Self::Local(key) => key.public_key_hex(),
            Self::External(signer) => signer.public_key_hex().to_owned(),
        }
    }

    fn sign_receipt_header(&self, unsigned_header_bytes: &[u8]) -> Result<String, String> {
        match self {
            Self::Local(key) => Ok(key.to_crypto_key().sign(unsigned_header_bytes).to_hex()),
            Self::External(signer) => signer.sign_receipt_header(unsigned_header_bytes),
        }
    }
}

pub(crate) fn accountability_receipt_audit_hash(receipt: &Value) -> String {
    hex_lower(&blake3_256_domain(
        ACCOUNTABILITY_RECEIPT_AUDIT_HASH_DOMAIN,
        &canonical_json_bytes(receipt),
    ))
}

pub(crate) fn accountability_receipt_audit_hash_from_response_body(body: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(body).ok()?;
    let receipt = value.get("accountability_receipt")?;
    if receipt.is_null() {
        return None;
    }
    Some(accountability_receipt_audit_hash(receipt))
}

fn current_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn transparency_log_path_from_env() -> Result<Option<PathBuf>, String> {
    match std::env::var("CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE") {
        Ok(raw) => parse_transparency_log_path(&raw).map(Some),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(error) => Err(format!(
            "invalid CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE: {error}"
        )),
    }
}

fn audit_log_path_from_options(options: &ServerOptions) -> Option<PathBuf> {
    if options.audit_log_enabled {
        options.audit_log_path.clone()
    } else {
        None
    }
}

fn parse_transparency_log_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        Err("CORTEXDB_RECEIPT_TRANSPARENCY_LOG_FILE must not be empty".to_owned())
    } else {
        Ok(PathBuf::from(trimmed))
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_transparency_log_path, ReceiptSigner};
    use crate::ReceiptExternalSigner;
    use cortex_engine::accountability::AccountabilityReceiptHeaderSigner;
    use std::path::PathBuf;

    #[test]
    fn parse_transparency_log_path_rejects_empty_value() {
        assert!(parse_transparency_log_path(" ").is_err());
        assert_eq!(
            parse_transparency_log_path("/tmp/cortexdb-transparency.jsonl")
                .unwrap()
                .to_string_lossy(),
            "/tmp/cortexdb-transparency.jsonl"
        );
    }

    #[test]
    fn receipt_signer_local_does_not_require_external_command() {
        let key = crate::ReceiptSigningKey::from_seed_hex(
            "receipt-key.local",
            "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f",
        )
        .unwrap();
        let signer = ReceiptSigner::Local(key.clone());

        assert_eq!(signer.key_id(), "receipt-key.local");
        assert_eq!(signer.public_key_hex(), key.public_key_hex());
        assert_eq!(signer.sign_receipt_header(b"header").unwrap().len(), 128);
    }

    #[test]
    fn receipt_signer_external_exposes_public_binding_without_seed() {
        let signer = ReceiptExternalSigner::new(
            "receipt-key.external",
            "03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5",
            PathBuf::from("/bin/false"),
            Some("kms://test/receipt-key".to_owned()),
        )
        .unwrap();
        let signer = ReceiptSigner::External(signer);

        assert_eq!(signer.key_id(), "receipt-key.external");
        assert_eq!(
            signer.public_key_hex(),
            "03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5"
        );
    }
}
