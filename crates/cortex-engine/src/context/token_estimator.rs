#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContextTokenProfile {
    #[default]
    CortexApproxV2,
    OpenAiGpt4o,
    DeepSeekChat,
    GoogleGemmaIt,
    BgeM3,
}

impl ContextTokenProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CortexApproxV2 => "cortex_approx_v2",
            Self::OpenAiGpt4o => "openai_gpt4o",
            Self::DeepSeekChat => "deepseek_chat",
            Self::GoogleGemmaIt => "google_gemma_it",
            Self::BgeM3 => "bge_m3",
        }
    }

    pub fn from_model_name(model: &str) -> Self {
        let normalized = model.to_ascii_lowercase();
        if normalized.contains("bge-m3") || normalized.contains("bge_m3") {
            Self::BgeM3
        } else if normalized.contains("gpt-4o") || normalized.contains("4o") {
            Self::OpenAiGpt4o
        } else if normalized.contains("deepseek") {
            Self::DeepSeekChat
        } else if normalized.contains("gemma") {
            Self::GoogleGemmaIt
        } else {
            Self::CortexApproxV2
        }
    }

    fn spec(self) -> TokenProfileSpec {
        match self {
            Self::CortexApproxV2 => TokenProfileSpec {
                ascii_chars_per_token: 4,
                non_ascii_chars_per_token: 2,
                punctuation_chars_per_token: 4,
                newline_chars_per_token: 2,
            },
            Self::OpenAiGpt4o => TokenProfileSpec {
                ascii_chars_per_token: 4,
                non_ascii_chars_per_token: 2,
                punctuation_chars_per_token: 4,
                newline_chars_per_token: 2,
            },
            Self::DeepSeekChat => TokenProfileSpec {
                ascii_chars_per_token: 4,
                non_ascii_chars_per_token: 2,
                punctuation_chars_per_token: 3,
                newline_chars_per_token: 2,
            },
            Self::GoogleGemmaIt => TokenProfileSpec {
                ascii_chars_per_token: 4,
                non_ascii_chars_per_token: 2,
                punctuation_chars_per_token: 3,
                newline_chars_per_token: 2,
            },
            Self::BgeM3 => TokenProfileSpec {
                ascii_chars_per_token: 3,
                non_ascii_chars_per_token: 1,
                punctuation_chars_per_token: 3,
                newline_chars_per_token: 2,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct TokenProfileSpec {
    ascii_chars_per_token: u32,
    non_ascii_chars_per_token: u32,
    punctuation_chars_per_token: u32,
    newline_chars_per_token: u32,
}

pub fn estimate_tokens(payload: &[u8]) -> u32 {
    estimate_tokens_for_profile(payload, ContextTokenProfile::default())
}

pub fn estimate_tokens_for_profile(payload: &[u8], profile: ContextTokenProfile) -> u32 {
    if payload.is_empty() {
        return 0;
    }
    let Ok(text) = std::str::from_utf8(payload) else {
        return estimate_invalid_utf8_tokens(payload.len());
    };
    let spec = profile.spec();
    let mut tokens = 0u32;
    let mut ascii_run = 0u32;
    let mut non_ascii_run = 0u32;
    let mut punctuation_run = 0u32;
    let mut newline_run = 0u32;

    for ch in text.chars() {
        if ch == '\n' || ch == '\r' {
            flush_runs(
                &mut tokens,
                &mut ascii_run,
                &mut non_ascii_run,
                &mut punctuation_run,
                &spec,
            );
            newline_run = newline_run.saturating_add(1);
        } else if ch.is_whitespace() {
            flush_runs(
                &mut tokens,
                &mut ascii_run,
                &mut non_ascii_run,
                &mut punctuation_run,
                &spec,
            );
            flush_newlines(&mut tokens, &mut newline_run, &spec);
        } else if ch.is_ascii_alphanumeric() {
            flush_non_ascii_and_punctuation(
                &mut tokens,
                &mut non_ascii_run,
                &mut punctuation_run,
                &spec,
            );
            flush_newlines(&mut tokens, &mut newline_run, &spec);
            ascii_run = ascii_run.saturating_add(1);
        } else if ch.is_ascii_punctuation() {
            flush_ascii_and_non_ascii(&mut tokens, &mut ascii_run, &mut non_ascii_run, &spec);
            flush_newlines(&mut tokens, &mut newline_run, &spec);
            punctuation_run = punctuation_run.saturating_add(1);
        } else {
            flush_ascii_and_punctuation(&mut tokens, &mut ascii_run, &mut punctuation_run, &spec);
            flush_newlines(&mut tokens, &mut newline_run, &spec);
            non_ascii_run = non_ascii_run.saturating_add(1);
        }
    }

    flush_runs(
        &mut tokens,
        &mut ascii_run,
        &mut non_ascii_run,
        &mut punctuation_run,
        &spec,
    );
    flush_newlines(&mut tokens, &mut newline_run, &spec);
    tokens.max(1)
}

fn estimate_invalid_utf8_tokens(byte_len: usize) -> u32 {
    let bytes = match u32::try_from(byte_len) {
        Ok(value) => value,
        Err(_) => return u32::MAX,
    };
    ceil_div(bytes, 4).max(1)
}

fn flush_runs(
    tokens: &mut u32,
    ascii_run: &mut u32,
    non_ascii_run: &mut u32,
    punctuation_run: &mut u32,
    spec: &TokenProfileSpec,
) {
    flush_ascii_and_non_ascii(tokens, ascii_run, non_ascii_run, spec);
    flush_punctuation(tokens, punctuation_run, spec);
}

fn flush_non_ascii_and_punctuation(
    tokens: &mut u32,
    non_ascii_run: &mut u32,
    punctuation_run: &mut u32,
    spec: &TokenProfileSpec,
) {
    flush_non_ascii(tokens, non_ascii_run, spec);
    flush_punctuation(tokens, punctuation_run, spec);
}

fn flush_ascii_and_non_ascii(
    tokens: &mut u32,
    ascii_run: &mut u32,
    non_ascii_run: &mut u32,
    spec: &TokenProfileSpec,
) {
    flush_ascii(tokens, ascii_run, spec);
    flush_non_ascii(tokens, non_ascii_run, spec);
}

fn flush_ascii_and_punctuation(
    tokens: &mut u32,
    ascii_run: &mut u32,
    punctuation_run: &mut u32,
    spec: &TokenProfileSpec,
) {
    flush_ascii(tokens, ascii_run, spec);
    flush_punctuation(tokens, punctuation_run, spec);
}

fn flush_ascii(tokens: &mut u32, run: &mut u32, spec: &TokenProfileSpec) {
    add_run(tokens, run, spec.ascii_chars_per_token);
}

fn flush_non_ascii(tokens: &mut u32, run: &mut u32, spec: &TokenProfileSpec) {
    add_run(tokens, run, spec.non_ascii_chars_per_token);
}

fn flush_punctuation(tokens: &mut u32, run: &mut u32, spec: &TokenProfileSpec) {
    add_run(tokens, run, spec.punctuation_chars_per_token);
}

fn flush_newlines(tokens: &mut u32, run: &mut u32, spec: &TokenProfileSpec) {
    add_run(tokens, run, spec.newline_chars_per_token);
}

fn add_run(tokens: &mut u32, run: &mut u32, chars_per_token: u32) {
    if *run == 0 {
        return;
    }
    *tokens = tokens.saturating_add(ceil_div(*run, chars_per_token.max(1)));
    *run = 0;
}

fn ceil_div(value: u32, divisor: u32) -> u32 {
    value.saturating_add(divisor.saturating_sub(1)) / divisor
}

#[cfg(test)]
mod tests {
    use super::{estimate_tokens_for_profile, ContextTokenProfile};

    #[test]
    fn profiles_are_deterministic() {
        let payload = "scope=project:investments\nҚазақстан renewable energy budget.";
        assert_eq!(
            estimate_tokens_for_profile(payload.as_bytes(), ContextTokenProfile::BgeM3),
            estimate_tokens_for_profile(payload.as_bytes(), ContextTokenProfile::BgeM3)
        );
    }

    #[test]
    fn multilingual_profile_is_stricter_for_non_ascii_text() {
        let payload = "Қазақстан энергетика жобасы";
        assert!(
            estimate_tokens_for_profile(payload.as_bytes(), ContextTokenProfile::BgeM3)
                > estimate_tokens_for_profile(payload.as_bytes(), ContextTokenProfile::OpenAiGpt4o)
        );
    }
}
