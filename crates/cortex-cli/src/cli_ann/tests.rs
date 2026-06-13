use super::parse_ann_policy;
use super::parse_no_fallback_profile;
use super::parse_no_fallback_rollout_policy;

#[test]
fn parse_ann_policy_default_values() {
    let policy = parse_ann_policy(None, None, None, None, false).unwrap();
    assert!(policy.fallback);
    assert_eq!(policy.min_recall_q16, Some(49_151));
    assert!(!policy.require_slo);
}

#[test]
fn parse_ann_policy_custom_values() {
    let policy = parse_ann_policy(
        Some("false".to_owned()),
        Some(123),
        Some("75%".to_owned()),
        Some(100),
        true,
    )
    .unwrap();
    assert!(!policy.fallback);
    assert_eq!(policy.fallback_scan_cap, Some(123));
    assert_eq!(policy.min_recall_q16, Some(49_151));
    assert_eq!(policy.max_visited_candidates, Some(100));
    assert!(policy.require_slo);
}

#[test]
fn parse_ann_policy_rejects_invalid_bool() {
    let err = parse_ann_policy(Some("maybe".to_owned()), None, None, None, false).unwrap_err();
    assert!(err.contains("fallback must be true/false"));
}

#[test]
fn parse_no_fallback_rollout_policy_requires_explicit_rollout() {
    let err = parse_no_fallback_rollout_policy(false, Some("1.0".to_owned())).unwrap_err();
    assert!(err.contains("--no-fallback-rollout"));
    assert!(parse_no_fallback_rollout_policy(false, None)
        .unwrap()
        .is_none());
}

#[test]
fn parse_no_fallback_rollout_policy_accepts_threshold() {
    let policy = parse_no_fallback_rollout_policy(true, Some("100%".to_owned()))
        .unwrap()
        .unwrap();
    assert!(policy.rollout_enabled);
    assert_eq!(policy.min_recall_q16, 65_535);
}

#[test]
fn parse_no_fallback_profile_accepts_bool_and_threshold() {
    let policy = parse_no_fallback_profile(
        "true".to_owned(),
        Some("100%".to_owned()),
        "false".to_owned(),
    )
    .unwrap();
    assert!(policy.rollout_enabled);
    assert_eq!(policy.min_recall_q16, 65_535);
    assert!(!policy.require_upper_layers);
}
