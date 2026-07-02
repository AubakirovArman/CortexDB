use crate::{hex_lower, hmac_sha256, sha256_domain, MacKey};

pub const AUDIT_CHAIN_ID: &str = "cortexdb.audit.chain.v2";
pub const AUDIT_CHAIN_ZERO_HASH: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";

const EVENT_HASH_DOMAIN: &str = "cortexdb.audit.event.v2";
const EVENT_MAC_DOMAIN: &str = "cortexdb.audit.event_mac.v2";

pub fn event_hash(fields: &[(&str, String)]) -> String {
    hex_lower(&sha256_domain(
        EVENT_HASH_DOMAIN,
        &canonical_event_bytes(fields),
    ))
}

pub fn event_mac(key: &MacKey, fields: &[(&str, String)]) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(EVENT_MAC_DOMAIN.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&canonical_event_bytes(fields));
    hex_lower(&hmac_sha256(key, &bytes))
}

pub fn is_hex_hash(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn canonical_event_bytes(fields: &[(&str, String)]) -> Vec<u8> {
    let mut out = Vec::new();
    for (key, value) in fields {
        out.extend_from_slice(key.as_bytes());
        out.push(0x1f);
        out.extend_from_slice(value.as_bytes());
        out.push(0x1e);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MacKey;

    #[test]
    fn audit_event_hash_is_sha256_width_and_order_sensitive() {
        let fields = [
            ("chain_id", AUDIT_CHAIN_ID.to_owned()),
            ("sequence", "1".to_owned()),
            ("prev_hash", AUDIT_CHAIN_ZERO_HASH.to_owned()),
        ];
        let hash = event_hash(&fields);
        assert!(is_hex_hash(&hash));
        assert_ne!(
            hash,
            event_hash(&[
                ("sequence", "1".to_owned()),
                ("chain_id", AUDIT_CHAIN_ID.to_owned()),
                ("prev_hash", AUDIT_CHAIN_ZERO_HASH.to_owned()),
            ])
        );
    }

    #[test]
    fn audit_event_mac_is_keyed_and_sha256_width() {
        let key_a = MacKey::new([1_u8; 32]);
        let key_b = MacKey::new([2_u8; 32]);
        let fields = [("sequence", "1".to_owned())];
        let mac_a = event_mac(&key_a, &fields);
        let mac_b = event_mac(&key_b, &fields);
        assert!(is_hex_hash(&mac_a));
        assert_ne!(mac_a, mac_b);
    }
}
