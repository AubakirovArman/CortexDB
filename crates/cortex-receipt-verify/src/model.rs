use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct VerifyInput {
    pub schema_version: String,
    pub pack: Value,
    pub determinism_input: Value,
    pub receipt: Receipt,
    pub public_key: PublicKeyInput,
    #[serde(default)]
    pub admitted_cells: Vec<AdmittedCellInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PublicKeyInput {
    pub key_id: String,
    pub public_key_hex: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdmittedCellInput {
    pub cell_id: u64,
    pub cell_content_hash: String,
    #[serde(default)]
    pub raw_content_hex: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Receipt {
    pub schema_version: String,
    pub header: ReceiptHeader,
    pub leaves: ReceiptLeaves,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReceiptHeader {
    pub schema_version: String,
    pub hash_alg: String,
    pub sig_alg: String,
    pub db_instance_id: String,
    pub key_id: String,
    pub created_unix_seconds: u64,
    pub access_root: String,
    pub provenance_root: String,
    pub cell_set_root: String,
    pub verification_root: String,
    pub budget_commitment: String,
    pub conflict_commitment: String,
    pub pack_root: String,
    pub determinism_hash: String,
    pub audit_chain_head: String,
    pub signature: ReceiptSignature,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReceiptSignature {
    pub key_id: String,
    pub sig_alg: String,
    pub public_key_hex: String,
    pub signature_hex: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ReceiptLeaves {
    #[serde(default)]
    pub access: Vec<Value>,
    #[serde(default)]
    pub provenance: Vec<Value>,
    #[serde(default)]
    pub cell_set: Vec<Value>,
    #[serde(default)]
    pub verification: Vec<Value>,
    #[serde(default)]
    pub budget: Vec<Value>,
    #[serde(default)]
    pub conflict: Vec<Value>,
}
