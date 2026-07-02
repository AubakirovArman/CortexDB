use cortex_crypto::{
    ReceiptKeyRing, ReceiptPublicKey, ReceiptSignature, ReceiptSigningKey, RECEIPT_SIGNING_DOMAIN,
};
use serde_json::{json, Value};

use super::receipt::AccountabilityReceiptBody;
use crate::canonical::canonical_json_bytes;
use crate::error::{EngineError, EngineResult};

pub const ACCOUNTABILITY_RECEIPT_SCHEMA_VERSION: &str = "accountability_receipt.v1";
pub const ACCOUNTABILITY_RECEIPT_HASH_ALG: &str = "blake3-256";
pub const ACCOUNTABILITY_RECEIPT_SIG_ALG: &str = "ed25519";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountabilityReceiptHeader {
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
    pub signature: AccountabilityReceiptSignature,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountabilityReceiptSignature {
    pub key_id: String,
    pub sig_alg: String,
    pub public_key_hex: String,
    pub signature_hex: String,
}

pub trait AccountabilityReceiptHeaderSigner {
    fn key_id(&self) -> &str;
    fn public_key_hex(&self) -> String;
    fn sign_receipt_header(&self, unsigned_header_bytes: &[u8]) -> Result<String, String>;
}

impl AccountabilityReceiptHeaderSigner for ReceiptSigningKey {
    fn key_id(&self) -> &str {
        self.key_id()
    }

    fn public_key_hex(&self) -> String {
        self.public_key().to_hex()
    }

    fn sign_receipt_header(&self, unsigned_header_bytes: &[u8]) -> Result<String, String> {
        Ok(self.sign(unsigned_header_bytes).to_hex())
    }
}

pub fn sign_accountability_receipt_header(
    body: &AccountabilityReceiptBody,
    db_instance_id: &str,
    created_unix_seconds: u64,
    audit_chain_head: &str,
    signing_key: &ReceiptSigningKey,
) -> EngineResult<AccountabilityReceiptHeader> {
    sign_accountability_receipt_header_with_signer(
        body,
        db_instance_id,
        created_unix_seconds,
        audit_chain_head,
        signing_key,
    )
}

pub fn sign_accountability_receipt_header_with_signer<S>(
    body: &AccountabilityReceiptBody,
    db_instance_id: &str,
    created_unix_seconds: u64,
    audit_chain_head: &str,
    signer: &S,
) -> EngineResult<AccountabilityReceiptHeader>
where
    S: AccountabilityReceiptHeaderSigner + ?Sized,
{
    if db_instance_id.trim().is_empty() || !is_hex_hash(audit_chain_head) {
        return Err(EngineError::InvalidOperation);
    }

    let unsigned = unsigned_header_value(
        body,
        db_instance_id,
        created_unix_seconds,
        audit_chain_head,
        signer.key_id(),
    );
    let unsigned_bytes = canonical_json_bytes(&unsigned);
    let public_key_hex = signer.public_key_hex();
    let signature_hex = signer
        .sign_receipt_header(&unsigned_bytes)
        .map_err(|_| EngineError::InvalidOperation)?;
    let public_key = ReceiptPublicKey::from_hex(signer.key_id(), &public_key_hex)
        .map_err(|_| EngineError::InvalidOperation)?;
    let signature =
        ReceiptSignature::from_hex(&signature_hex).map_err(|_| EngineError::InvalidOperation)?;
    public_key
        .verify(&unsigned_bytes, &signature)
        .map_err(|_| EngineError::InvalidOperation)?;

    Ok(AccountabilityReceiptHeader {
        schema_version: ACCOUNTABILITY_RECEIPT_SCHEMA_VERSION.to_owned(),
        hash_alg: ACCOUNTABILITY_RECEIPT_HASH_ALG.to_owned(),
        sig_alg: ACCOUNTABILITY_RECEIPT_SIG_ALG.to_owned(),
        db_instance_id: db_instance_id.to_owned(),
        key_id: signer.key_id().to_owned(),
        created_unix_seconds,
        access_root: body.access_root.clone(),
        provenance_root: body.provenance_root.clone(),
        cell_set_root: body.cell_set_root.clone(),
        verification_root: body.verification_root.clone(),
        budget_commitment: body.budget_commitment.clone(),
        conflict_commitment: body.conflict_commitment.clone(),
        pack_root: body.pack_root.clone(),
        determinism_hash: body.determinism_hash.clone(),
        audit_chain_head: audit_chain_head.to_owned(),
        signature: AccountabilityReceiptSignature {
            key_id: signer.key_id().to_owned(),
            sig_alg: ACCOUNTABILITY_RECEIPT_SIG_ALG.to_owned(),
            public_key_hex,
            signature_hex,
        },
    })
}

pub fn canonical_accountability_receipt_header_bytes(
    header: &AccountabilityReceiptHeader,
) -> Vec<u8> {
    canonical_json_bytes(&unsigned_header_value_from_header(header))
}

pub fn accountability_receipt_header_value(header: &AccountabilityReceiptHeader) -> Value {
    json!({
        "schema_version": header.schema_version,
        "hash_alg": header.hash_alg,
        "sig_alg": header.sig_alg,
        "db_instance_id": header.db_instance_id,
        "key_id": header.key_id,
        "created_unix_seconds": header.created_unix_seconds,
        "access_root": header.access_root,
        "provenance_root": header.provenance_root,
        "cell_set_root": header.cell_set_root,
        "verification_root": header.verification_root,
        "budget_commitment": header.budget_commitment,
        "conflict_commitment": header.conflict_commitment,
        "pack_root": header.pack_root,
        "determinism_hash": header.determinism_hash,
        "audit_chain_head": header.audit_chain_head,
        "signature": {
            "key_id": header.signature.key_id,
            "sig_alg": header.signature.sig_alg,
            "public_key_hex": header.signature.public_key_hex,
            "signature_hex": header.signature.signature_hex,
        },
    })
}

pub fn verify_accountability_receipt_header(
    header: &AccountabilityReceiptHeader,
    keyring: &ReceiptKeyRing,
) -> EngineResult<()> {
    if header.schema_version != ACCOUNTABILITY_RECEIPT_SCHEMA_VERSION
        || header.hash_alg != ACCOUNTABILITY_RECEIPT_HASH_ALG
        || header.sig_alg != ACCOUNTABILITY_RECEIPT_SIG_ALG
        || header.signature.sig_alg != ACCOUNTABILITY_RECEIPT_SIG_ALG
        || header.key_id != header.signature.key_id
        || header.db_instance_id.trim().is_empty()
        || !is_hex_hash(&header.audit_chain_head)
    {
        return Err(EngineError::InvalidOperation);
    }

    let trusted_key = keyring
        .trusted_public_keys()
        .iter()
        .find(|key| key.key_id() == header.key_id)
        .ok_or(EngineError::InvalidOperation)?;
    if trusted_key.to_hex() != header.signature.public_key_hex {
        return Err(EngineError::InvalidOperation);
    }

    let signature = ReceiptSignature::from_hex(&header.signature.signature_hex)
        .map_err(|_| EngineError::InvalidOperation)?;
    keyring
        .verify(
            &header.key_id,
            &canonical_accountability_receipt_header_bytes(header),
            &signature,
        )
        .map_err(|_| EngineError::InvalidOperation)
}

fn unsigned_header_value(
    body: &AccountabilityReceiptBody,
    db_instance_id: &str,
    created_unix_seconds: u64,
    audit_chain_head: &str,
    key_id: &str,
) -> Value {
    json!({
        "schema_version": ACCOUNTABILITY_RECEIPT_SCHEMA_VERSION,
        "hash_alg": ACCOUNTABILITY_RECEIPT_HASH_ALG,
        "sig_alg": ACCOUNTABILITY_RECEIPT_SIG_ALG,
        "signing_domain": RECEIPT_SIGNING_DOMAIN,
        "db_instance_id": db_instance_id,
        "key_id": key_id,
        "created_unix_seconds": created_unix_seconds,
        "access_root": body.access_root,
        "provenance_root": body.provenance_root,
        "cell_set_root": body.cell_set_root,
        "verification_root": body.verification_root,
        "budget_commitment": body.budget_commitment,
        "conflict_commitment": body.conflict_commitment,
        "pack_root": body.pack_root,
        "determinism_hash": body.determinism_hash,
        "audit_chain_head": audit_chain_head,
    })
}

fn unsigned_header_value_from_header(header: &AccountabilityReceiptHeader) -> Value {
    json!({
        "schema_version": header.schema_version,
        "hash_alg": header.hash_alg,
        "sig_alg": header.sig_alg,
        "signing_domain": RECEIPT_SIGNING_DOMAIN,
        "db_instance_id": header.db_instance_id,
        "key_id": header.key_id,
        "created_unix_seconds": header.created_unix_seconds,
        "access_root": header.access_root,
        "provenance_root": header.provenance_root,
        "cell_set_root": header.cell_set_root,
        "verification_root": header.verification_root,
        "budget_commitment": header.budget_commitment,
        "conflict_commitment": header.conflict_commitment,
        "pack_root": header.pack_root,
        "determinism_hash": header.determinism_hash,
        "audit_chain_head": header.audit_chain_head,
    })
}

fn is_hex_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
