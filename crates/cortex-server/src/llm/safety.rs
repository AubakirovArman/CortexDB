use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub(crate) struct LlmRuntimeSafetyConfig {
    pub enabled: bool,
    pub provider: String,
    pub model: String,
    pub max_prompt_bytes: u64,
    pub max_context_cells: u32,
    pub max_output_tokens: u32,
    pub request_timeout_ms: u64,
    pub queue_capacity: u32,
    pub max_concurrent_requests: u32,
    pub request_api_keys_allowed: bool,
    pub prompt_body_logging_enabled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LlmRuntimeSafetyFailure {
    MissingProvider,
    MissingModel,
    InvalidPromptLimit,
    InvalidContextCellLimit,
    InvalidOutputTokenLimit,
    InvalidTimeout,
    InvalidQueueCapacity,
    InvalidConcurrency,
    RequestApiKeysNotAllowed,
    PromptBodyLoggingNotAllowed,
}

pub(crate) fn validate_llm_runtime_safety_config(
    config: &LlmRuntimeSafetyConfig,
) -> Result<(), LlmRuntimeSafetyFailure> {
    if config.provider.trim().is_empty() {
        return Err(LlmRuntimeSafetyFailure::MissingProvider);
    }
    if config.model.trim().is_empty() {
        return Err(LlmRuntimeSafetyFailure::MissingModel);
    }
    if config.max_prompt_bytes == 0 || config.max_prompt_bytes > 256 * 1024 {
        return Err(LlmRuntimeSafetyFailure::InvalidPromptLimit);
    }
    if config.max_context_cells == 0 || config.max_context_cells > 128 {
        return Err(LlmRuntimeSafetyFailure::InvalidContextCellLimit);
    }
    if config.max_output_tokens == 0 || config.max_output_tokens > 8_192 {
        return Err(LlmRuntimeSafetyFailure::InvalidOutputTokenLimit);
    }
    if config.request_timeout_ms == 0 || config.request_timeout_ms > 120_000 {
        return Err(LlmRuntimeSafetyFailure::InvalidTimeout);
    }
    if config.queue_capacity == 0 || config.queue_capacity > 10_000 {
        return Err(LlmRuntimeSafetyFailure::InvalidQueueCapacity);
    }
    if config.max_concurrent_requests == 0 || config.max_concurrent_requests > 512 {
        return Err(LlmRuntimeSafetyFailure::InvalidConcurrency);
    }
    if config.request_api_keys_allowed {
        return Err(LlmRuntimeSafetyFailure::RequestApiKeysNotAllowed);
    }
    if config.prompt_body_logging_enabled {
        return Err(LlmRuntimeSafetyFailure::PromptBodyLoggingNotAllowed);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> LlmRuntimeSafetyConfig {
        LlmRuntimeSafetyConfig {
            enabled: false,
            provider: "test_double".to_owned(),
            model: "deterministic-echo-v1".to_owned(),
            max_prompt_bytes: 16 * 1024,
            max_context_cells: 32,
            max_output_tokens: 512,
            request_timeout_ms: 5_000,
            queue_capacity: 64,
            max_concurrent_requests: 4,
            request_api_keys_allowed: false,
            prompt_body_logging_enabled: false,
        }
    }

    #[test]
    fn llm_runtime_safety_config_accepts_bounded_disabled_runtime() {
        assert_eq!(validate_llm_runtime_safety_config(&config()), Ok(()));
    }

    #[test]
    fn llm_runtime_safety_config_rejects_unbounded_prompt_and_secret_policy() {
        let mut invalid = config();
        invalid.max_prompt_bytes = 256 * 1024 + 1;
        assert_eq!(
            validate_llm_runtime_safety_config(&invalid),
            Err(LlmRuntimeSafetyFailure::InvalidPromptLimit)
        );

        let mut invalid = config();
        invalid.request_api_keys_allowed = true;
        assert_eq!(
            validate_llm_runtime_safety_config(&invalid),
            Err(LlmRuntimeSafetyFailure::RequestApiKeysNotAllowed)
        );

        let mut invalid = config();
        invalid.prompt_body_logging_enabled = true;
        assert_eq!(
            validate_llm_runtime_safety_config(&invalid),
            Err(LlmRuntimeSafetyFailure::PromptBodyLoggingNotAllowed)
        );
    }
}
