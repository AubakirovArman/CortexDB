use cortex_crypto::ReceiptSigningKey;
use serde_json::{json, Value};

use crate::accountability::{
    accountability_receipt_body, accountability_receipt_header_value,
    sign_accountability_receipt_header_with_signer, AccountabilityDeterminismInput,
    AccountabilityReceiptHeaderSigner,
};
use crate::database::{CapturedAccessDenialSet, RetrievedCell};
use crate::determinism_hash::determinism_hash;
use crate::error::EngineResult;
use crate::verification::VerificationReport;

use super::ContextPack;

#[derive(Clone, Debug, PartialEq)]
pub struct ContextPackReceiptEvidence {
    pub pack: ContextPack,
    retrieved_cells: Vec<RetrievedCell>,
    captured_access_denials: CapturedAccessDenialSet,
    determinism_input: AccountabilityDeterminismInput,
}

impl ContextPackReceiptEvidence {
    pub(crate) fn new(
        pack: ContextPack,
        retrieved_cells: Vec<RetrievedCell>,
        captured_access_denials: CapturedAccessDenialSet,
        determinism_input: AccountabilityDeterminismInput,
    ) -> Self {
        Self {
            pack,
            retrieved_cells,
            captured_access_denials,
            determinism_input,
        }
    }

    pub fn signed_receipt_value(
        &self,
        verification_report: Option<&VerificationReport>,
        db_instance_id: &str,
        created_unix_seconds: u64,
        audit_chain_head: &str,
        signing_key: &ReceiptSigningKey,
    ) -> EngineResult<Value> {
        self.signed_receipt_value_with_signer(
            verification_report,
            db_instance_id,
            created_unix_seconds,
            audit_chain_head,
            signing_key,
        )
    }

    pub fn signed_receipt_value_with_signer<S>(
        &self,
        verification_report: Option<&VerificationReport>,
        db_instance_id: &str,
        created_unix_seconds: u64,
        audit_chain_head: &str,
        signer: &S,
    ) -> EngineResult<Value>
    where
        S: AccountabilityReceiptHeaderSigner + ?Sized,
    {
        let body = accountability_receipt_body(
            &self.pack,
            &self.retrieved_cells,
            &self.captured_access_denials,
            verification_report,
            &self.determinism_input,
        )?;
        let header = sign_accountability_receipt_header_with_signer(
            &body,
            db_instance_id,
            created_unix_seconds,
            audit_chain_head,
            signer,
        )?;
        Ok(json!({
            "schema_version": "accountability_receipt.v1",
            "header": accountability_receipt_header_value(&header),
            "leaves": {
                "access": body.leaves.access,
                "provenance": body.leaves.provenance,
                "cell_set": body.leaves.cell_set,
                "verification": body.leaves.verification,
                "budget": body.leaves.budget,
                "conflict": body.leaves.conflict,
            },
        }))
    }

    pub fn determinism_hash(&self) -> String {
        determinism_hash(&self.determinism_input.as_hash_input())
    }
}
