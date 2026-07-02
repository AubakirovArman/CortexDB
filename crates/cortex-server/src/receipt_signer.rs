use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use cortex_crypto::{hex_lower, ReceiptPublicKey, ReceiptSignature, RECEIPT_SIGNING_DOMAIN};
use cortex_engine::accountability::AccountabilityReceiptHeaderSigner;
use serde::{Deserialize, Serialize};

const EXTERNAL_SIGNER_REQUEST_SCHEMA: &str = "cortexdb.receipt_external_sign_request.v1";
const EXTERNAL_SIGNER_RESPONSE_SCHEMA: &str = "cortexdb.receipt_external_signature.v1";

#[derive(Clone, PartialEq, Eq)]
pub struct ReceiptExternalSigner {
    key_id: String,
    public_key_hex: String,
    command: PathBuf,
    signer_ref: Option<String>,
}

impl ReceiptExternalSigner {
    pub fn new(
        key_id: &str,
        public_key_hex: &str,
        command: PathBuf,
        signer_ref: Option<String>,
    ) -> Result<Self, String> {
        if command.as_os_str().is_empty() {
            return Err("receipt external signer command must not be empty".to_owned());
        }
        let public_key = ReceiptPublicKey::from_hex(key_id.trim(), public_key_hex.trim())
            .map_err(|error| format!("invalid receipt external signer public key: {error}"))?;
        Ok(Self {
            key_id: public_key.key_id().to_owned(),
            public_key_hex: public_key.to_hex(),
            command,
            signer_ref,
        })
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }

    pub fn public_key_hex(&self) -> &str {
        &self.public_key_hex
    }

    pub fn command(&self) -> &Path {
        &self.command
    }

    pub fn signer_ref(&self) -> Option<&str> {
        self.signer_ref.as_deref()
    }
}

impl fmt::Debug for ReceiptExternalSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReceiptExternalSigner")
            .field("key_id", &self.key_id)
            .field("public_key_hex", &self.public_key_hex)
            .field("command", &self.command)
            .field("signer_ref", &self.signer_ref.as_deref().unwrap_or(""))
            .finish()
    }
}

impl AccountabilityReceiptHeaderSigner for ReceiptExternalSigner {
    fn key_id(&self) -> &str {
        &self.key_id
    }

    fn public_key_hex(&self) -> String {
        self.public_key_hex.clone()
    }

    fn sign_receipt_header(&self, unsigned_header_bytes: &[u8]) -> Result<String, String> {
        let request = ExternalSignerRequest {
            schema_version: EXTERNAL_SIGNER_REQUEST_SCHEMA,
            key_id: &self.key_id,
            public_key_hex: &self.public_key_hex,
            signing_domain: RECEIPT_SIGNING_DOMAIN,
            signer_ref: self.signer_ref.as_deref(),
            canonical_header_hex: hex_lower(unsigned_header_bytes),
        };
        let body = serde_json::to_vec(&request)
            .map_err(|error| format!("receipt external signer request encode failed: {error}"))?;
        let mut child = Command::new(&self.command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| format!("receipt external signer command failed to start: {error}"))?;
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| "receipt external signer stdin unavailable".to_owned())?;
        stdin
            .write_all(&body)
            .map_err(|error| format!("receipt external signer stdin write failed: {error}"))?;
        drop(stdin);

        let output = child
            .wait_with_output()
            .map_err(|error| format!("receipt external signer command wait failed: {error}"))?;
        if !output.status.success() {
            return Err("receipt external signer command returned failure".to_owned());
        }
        let response: ExternalSignerResponse = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("receipt external signer stdout JSON invalid: {error}"))?;
        if response.schema_version != EXTERNAL_SIGNER_RESPONSE_SCHEMA {
            return Err("receipt external signer response schema_version is invalid".to_owned());
        }
        if response.key_id != self.key_id || response.public_key_hex != self.public_key_hex {
            return Err("receipt external signer response key binding mismatch".to_owned());
        }
        let signature = ReceiptSignature::from_hex(&response.signature_hex)
            .map_err(|error| format!("receipt external signer signature is invalid: {error}"))?;
        let public_key = ReceiptPublicKey::from_hex(&self.key_id, &self.public_key_hex)
            .map_err(|error| format!("receipt external signer public key is invalid: {error}"))?;
        public_key
            .verify(unsigned_header_bytes, &signature)
            .map_err(|_| "receipt external signer signature verification failed".to_owned())?;
        Ok(response.signature_hex)
    }
}

#[derive(Serialize)]
struct ExternalSignerRequest<'a> {
    schema_version: &'a str,
    key_id: &'a str,
    public_key_hex: &'a str,
    signing_domain: &'a str,
    signer_ref: Option<&'a str>,
    canonical_header_hex: String,
}

#[derive(Deserialize)]
struct ExternalSignerResponse {
    schema_version: String,
    key_id: String,
    public_key_hex: String,
    signature_hex: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PUBLIC_KEY_HEX: &str =
        "03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5";

    #[cfg(unix)]
    #[test]
    fn external_signer_rejects_invalid_signature_and_sends_contract_request() {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let request_path = dir.path().join("request.json");
        let script_path = dir.path().join("signer.sh");
        let script = format!(
            "#!/bin/sh\ncat > {}\nprintf '%s\\n' '{}'\n",
            sh_quote(&request_path),
            r#"{"schema_version":"cortexdb.receipt_external_signature.v1","key_id":"receipt-key.external","public_key_hex":"03a107bff3ce10be1d70dd18e74bc09967e9359b73eafcbc8ee3d22a69d5edb5","signature_hex":"00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000"}"#
        );
        fs::write(&script_path, script).unwrap();
        let mut permissions = fs::metadata(&script_path).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&script_path, permissions).unwrap();

        let signer = ReceiptExternalSigner::new(
            "receipt-key.external",
            TEST_PUBLIC_KEY_HEX,
            script_path,
            Some("kms://test/receipt-key".to_owned()),
        )
        .unwrap();
        let error = signer.sign_receipt_header(b"canonical header").unwrap_err();
        assert!(error.contains("signature verification failed"));

        let request: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(request_path).unwrap()).unwrap();
        assert_eq!(
            request["schema_version"],
            "cortexdb.receipt_external_sign_request.v1"
        );
        assert_eq!(request["key_id"], "receipt-key.external");
        assert_eq!(request["public_key_hex"], TEST_PUBLIC_KEY_HEX);
        assert_eq!(
            request["signing_domain"],
            "cortexdb.accountability_receipt.sign.v1"
        );
        assert_eq!(request["signer_ref"], "kms://test/receipt-key");
        assert_eq!(
            request["canonical_header_hex"],
            "63616e6f6e6963616c20686561646572"
        );
    }

    #[cfg(unix)]
    fn sh_quote(path: &Path) -> String {
        format!("'{}'", path.to_string_lossy().replace('\'', "'\\''"))
    }
}
