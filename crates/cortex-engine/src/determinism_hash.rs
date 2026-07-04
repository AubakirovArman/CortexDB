use cortex_crypto::{blake3_256_domain, hex_lower};
use serde_json::{json, Value};

use crate::canonical::canonical_json_bytes;
use crate::search::frozen_weights;

pub const DETERMINISM_HASH_SCHEMA: &str = "cortexdb.determinism_hash.input.v1";
pub const DETERMINISM_HASH_DOMAIN: &str = "cortexdb.determinism_hash.v1";
pub const FROZEN_RANKING_WEIGHTS_HASH_DOMAIN: &str =
    "cortexdb.ranking.frozen_weights_artifact_hash.v1";

const FROZEN_RANKING_WEIGHTS_ARTIFACT: &[u8] =
    include_bytes!("../fixtures/ranking_frozen_weights_v1.json");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrozenWeightsIdentity {
    pub version: String,
    pub artifact_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeterminismHashInput<'a> {
    pub query: &'a str,
    pub agent_view_digest: Option<&'a str>,
    pub context_options_digest: Option<&'a str>,
    pub bitmap_program_digest: Option<&'a str>,
    pub frozen_weights_version: &'a str,
    pub frozen_weights_artifact_hash: &'a str,
    /// A3.3: the per-collection ANN serving epoch. `None` for the default
    /// (unconditional exact-recall) serving path — in which case the hash input is
    /// byte-identical to the frozen v1 goldens. `Some(epoch)` only when a
    /// collection is under guarded sampling, so the epoch enters the signed
    /// determinism surface additively (no golden churn on the default path).
    pub serving_epoch: Option<u64>,
    /// C3-5-embref: the store embedding *profile* identity
    /// `emb1:<model>:<dimension>:<metric>`. `None` (the default, and every
    /// keyword-only deployment) leaves the hash input byte-identical to
    /// pre-C3-5-embref. `Some(ref)` only when receipt embedding-ref binding is
    /// enabled and a profile is configured, so a verifier re-executing a
    /// hybrid/semantic query reproduces the candidate set under the same profile
    /// (see ADR-embedding-ref-receipt-visibility).
    pub embedding_ref: Option<&'a str>,
}

pub fn frozen_ranking_weights_identity() -> FrozenWeightsIdentity {
    FrozenWeightsIdentity {
        version: frozen_weights::VERSION.to_owned(),
        artifact_hash: frozen_ranking_weights_artifact_hash(),
    }
}

pub fn frozen_ranking_weights_artifact_hash() -> String {
    frozen_ranking_weights_artifact_hash_for_bytes(FROZEN_RANKING_WEIGHTS_ARTIFACT)
}

pub fn frozen_ranking_weights_artifact_hash_for_bytes(bytes: &[u8]) -> String {
    hash_bytes(FROZEN_RANKING_WEIGHTS_HASH_DOMAIN, bytes)
}

pub fn determinism_hash(input: &DeterminismHashInput<'_>) -> String {
    hash_value(
        DETERMINISM_HASH_DOMAIN,
        &determinism_hash_input_value(input),
    )
}

pub fn determinism_hash_input_value(input: &DeterminismHashInput<'_>) -> Value {
    let mut value = json!({
        "schema_version": DETERMINISM_HASH_SCHEMA,
        "query": input.query,
        "agent_view_digest": input.agent_view_digest,
        "context_options_digest": input.context_options_digest,
        "bitmap_program_digest": input.bitmap_program_digest,
        "frozen_weights": {
            "version": input.frozen_weights_version,
            "artifact_hash": input.frozen_weights_artifact_hash,
        },
    });
    // A3.3: the ANN serving epoch enters the signed determinism surface ONLY when a
    // collection is under guarded sampling (`Some`). When absent (`None`, the
    // default serving path), the object is byte-identical to the frozen v1
    // goldens — an additive change with no golden churn. A verifier that sees an
    // `ann_serving_epoch` re-executes under that serving mode; one that does not
    // re-executes the unconditional exact path, exactly as before.
    if let Some(epoch) = input.serving_epoch {
        value
            .as_object_mut()
            .expect("determinism input value is a JSON object")
            .insert("ann_serving_epoch".to_owned(), json!(epoch));
    }
    // C3-5-embref: the embedding profile ref enters the signed surface ONLY when
    // receipt embedding-ref binding is enabled with a profile present (`Some`).
    // When absent (`None`, the default and every keyword deployment), the object
    // is byte-identical to the pre-C3-5-embref goldens — additive, no churn. A
    // verifier that sees an `embedding_ref` re-executes under that embedding
    // profile; one that does not re-executes with no profile binding, as before.
    if let Some(embedding_ref) = input.embedding_ref {
        value
            .as_object_mut()
            .expect("determinism input value is a JSON object")
            .insert("embedding_ref".to_owned(), json!(embedding_ref));
    }
    value
}

fn hash_value(domain: &str, value: &Value) -> String {
    hash_bytes(domain, &canonical_json_bytes(value))
}

fn hash_bytes(domain: &str, bytes: &[u8]) -> String {
    hex_lower(&blake3_256_domain(domain, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input() -> DeterminismHashInput<'static> {
        DeterminismHashInput {
            query: "RETRIEVE CONTEXT FOR TASK \"ship\"",
            agent_view_digest: Some("a"),
            context_options_digest: Some("b"),
            bitmap_program_digest: Some("c"),
            frozen_weights_version: "ranking-frozen-weights-v1",
            frozen_weights_artifact_hash: "hash-a",
            serving_epoch: None,
            embedding_ref: None,
        }
    }

    #[test]
    fn determinism_hash_binds_frozen_weight_hash() {
        let input = sample_input();
        let mut changed = input.clone();
        changed.frozen_weights_artifact_hash = "hash-b";

        assert_ne!(determinism_hash(&input), determinism_hash(&changed));
    }

    // A3.3: the serving epoch is additive. `None` (the default serving path) must
    // reproduce the exact bytes the frozen v1 goldens were signed over — i.e. the
    // canonical input value must have no `ann_serving_epoch` key at all — so adding
    // the field cannot churn any committed accountability golden. `Some` must both
    // add the key and change the hash (so a verifier re-executes under that epoch).
    #[test]
    fn serving_epoch_is_additive_and_golden_safe() {
        let none = sample_input();
        // The None value is byte-identical to the pre-A3.3 object (no epoch key).
        let value = determinism_hash_input_value(&none);
        assert!(
            value
                .as_object()
                .unwrap()
                .get("ann_serving_epoch")
                .is_none(),
            "None serving_epoch must not add a key (golden-safety)"
        );

        let mut with_epoch = sample_input();
        with_epoch.serving_epoch = Some(7);
        let epoch_value = determinism_hash_input_value(&with_epoch);
        assert_eq!(
            epoch_value.as_object().unwrap().get("ann_serving_epoch"),
            Some(&serde_json::json!(7)),
        );
        assert_ne!(
            determinism_hash(&none),
            determinism_hash(&with_epoch),
            "a distinct serving epoch must change the determinism hash"
        );

        // Different epochs hash differently (monotonic transitions are visible).
        let mut with_epoch_8 = sample_input();
        with_epoch_8.serving_epoch = Some(8);
        assert_ne!(
            determinism_hash(&with_epoch),
            determinism_hash(&with_epoch_8)
        );
    }

    // C3-5-embref: the embedding profile ref is additive. `None` (the default and
    // every keyword deployment) must reproduce the exact bytes the pre-C3-5-embref
    // goldens were signed over — no `embedding_ref` key — so adding the field
    // cannot churn any committed accountability golden. `Some` must both add the
    // key and change the hash (so a verifier re-executes under that profile).
    #[test]
    fn embedding_ref_is_additive_and_golden_safe() {
        let none = sample_input();
        let value = determinism_hash_input_value(&none);
        assert!(
            value.as_object().unwrap().get("embedding_ref").is_none(),
            "None embedding_ref must not add a key (golden-safety)"
        );

        let mut with_ref = sample_input();
        with_ref.embedding_ref = Some("emb1:bge-m3:1024:cosine");
        let ref_value = determinism_hash_input_value(&with_ref);
        assert_eq!(
            ref_value.as_object().unwrap().get("embedding_ref"),
            Some(&serde_json::json!("emb1:bge-m3:1024:cosine")),
        );
        assert_ne!(
            determinism_hash(&none),
            determinism_hash(&with_ref),
            "a bound embedding profile must change the determinism hash"
        );

        // A different profile hashes differently (a model/metric change is visible).
        let mut with_other = sample_input();
        with_other.embedding_ref = Some("emb1:bge-m3:1024:dot_product");
        assert_ne!(determinism_hash(&with_ref), determinism_hash(&with_other));
    }

    // The two additive fields are independent: binding one must not mask the other.
    #[test]
    fn serving_epoch_and_embedding_ref_are_independently_additive() {
        let mut both = sample_input();
        both.serving_epoch = Some(3);
        both.embedding_ref = Some("emb1:bge-m3:1024:cosine");
        let obj = determinism_hash_input_value(&both);
        let obj = obj.as_object().unwrap();
        assert_eq!(obj.get("ann_serving_epoch"), Some(&serde_json::json!(3)));
        assert_eq!(
            obj.get("embedding_ref"),
            Some(&serde_json::json!("emb1:bge-m3:1024:cosine"))
        );

        let mut epoch_only = sample_input();
        epoch_only.serving_epoch = Some(3);
        assert_ne!(determinism_hash(&epoch_only), determinism_hash(&both));
    }

    #[test]
    fn frozen_weight_artifact_hash_is_stable_hex() {
        let hash = frozen_ranking_weights_artifact_hash();

        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
}
