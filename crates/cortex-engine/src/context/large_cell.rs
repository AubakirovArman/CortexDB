use cortex_core::CellId;

use super::token_estimator::estimate_tokens_for_profile;
use super::{ContextPackAnomaly, ContextPackAnomalyCode, ContextPackOptions};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ContextLargeCellPolicy {
    #[default]
    PreserveFirst,
    Truncate,
    Exclude,
    SummarizePlaceholder,
    SourceOnlyReference,
}

impl ContextLargeCellPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PreserveFirst => "preserve_first",
            Self::Truncate => "truncate",
            Self::Exclude => "exclude",
            Self::SummarizePlaceholder => "summarize_placeholder",
            Self::SourceOnlyReference => "source_only_reference",
        }
    }
}

pub(crate) enum LargeCellDecision {
    Include {
        payload: Vec<u8>,
        estimated_tokens: u32,
        anomaly: ContextPackAnomaly,
    },
    Exclude {
        anomaly: ContextPackAnomaly,
    },
}

pub(crate) struct LargeCellRequest<'a> {
    pub cell_id: CellId,
    pub payload: &'a [u8],
    pub citation: Option<&'a str>,
    pub citations_required: bool,
    pub original_tokens: u32,
    pub remaining_tokens: u32,
    pub options: &'a ContextPackOptions,
}

pub(crate) fn apply_large_cell_policy(request: LargeCellRequest<'_>) -> LargeCellDecision {
    match request.options.large_cell_policy {
        ContextLargeCellPolicy::PreserveFirst | ContextLargeCellPolicy::Exclude => exclude(
            request.cell_id,
            request.options.large_cell_policy,
            request.original_tokens,
            request.remaining_tokens,
        ),
        ContextLargeCellPolicy::Truncate => truncate(&request),
        ContextLargeCellPolicy::SummarizePlaceholder => {
            placeholder(&request, "summary_placeholder")
        }
        ContextLargeCellPolicy::SourceOnlyReference => {
            placeholder(&request, "source_only_reference")
        }
    }
}

pub(crate) fn estimate_cell_tokens(
    payload: &[u8],
    citation: Option<&str>,
    citations_required: bool,
    options: &ContextPackOptions,
) -> u32 {
    let citation_overhead = if citations_required && citation.is_some() {
        options.citation_overhead_tokens
    } else {
        0
    };
    estimate_tokens_for_profile(payload, options.token_profile).saturating_add(citation_overhead)
}

fn truncate(request: &LargeCellRequest<'_>) -> LargeCellDecision {
    if request.remaining_tokens == 0 {
        return exclude(
            request.cell_id,
            ContextLargeCellPolicy::Truncate,
            request.original_tokens,
            0,
        );
    }
    let marker = b"\n[context_pack_truncated=true]\n";
    let mut best = Vec::new();
    let mut low = 0usize;
    let mut high = request.payload.len();
    while low <= high {
        let mid = low + (high - low) / 2;
        let candidate = truncate_bytes(request.payload, mid, marker);
        let tokens = estimate_cell_tokens(
            &candidate,
            request.citation,
            request.citations_required,
            request.options,
        );
        if tokens <= request.remaining_tokens {
            best = candidate;
            low = mid.saturating_add(1);
        } else if mid == 0 {
            break;
        } else {
            high = mid - 1;
        }
    }
    include_or_exclude(request, ContextLargeCellPolicy::Truncate, best)
}

fn placeholder(request: &LargeCellRequest<'_>, kind: &str) -> LargeCellDecision {
    let text = String::from_utf8_lossy(request.payload);
    let mut out = String::new();
    out.push_str(kind);
    out.push_str("=true\n");
    out.push_str(&format!("original_cell_id={}\n", request.cell_id.0));
    out.push_str(&format!(
        "original_estimated_tokens={}\n",
        request.original_tokens
    ));
    if let Some(value) = request.citation {
        out.push_str("source=");
        out.push_str(value);
        out.push('\n');
    }
    for line in text.lines() {
        if line.starts_with("doc_id=")
            || line.starts_with("document_id=")
            || line.starts_with("chunk_id=")
            || line.starts_with("source_id=")
            || line.starts_with("source_url=")
            || line.starts_with("url=")
            || line.starts_with("title=")
        {
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("reason=large_cell_policy\n");
    include_or_exclude(request, request.options.large_cell_policy, out.into_bytes())
}

fn include_or_exclude(
    request: &LargeCellRequest<'_>,
    policy: ContextLargeCellPolicy,
    payload: Vec<u8>,
) -> LargeCellDecision {
    let estimated_tokens = estimate_cell_tokens(
        &payload,
        request.citation,
        request.citations_required,
        request.options,
    );
    if payload.is_empty() || estimated_tokens > request.remaining_tokens {
        exclude(
            request.cell_id,
            policy,
            request.original_tokens,
            request.remaining_tokens,
        )
    } else {
        LargeCellDecision::Include {
            payload,
            estimated_tokens,
            anomaly: anomaly(
                request.cell_id,
                policy,
                request.original_tokens,
                request.remaining_tokens,
                true,
            ),
        }
    }
}

fn exclude(
    cell_id: CellId,
    policy: ContextLargeCellPolicy,
    original_tokens: u32,
    remaining_tokens: u32,
) -> LargeCellDecision {
    LargeCellDecision::Exclude {
        anomaly: anomaly(cell_id, policy, original_tokens, remaining_tokens, false),
    }
}

fn anomaly(
    cell_id: CellId,
    policy: ContextLargeCellPolicy,
    original_tokens: u32,
    remaining_tokens: u32,
    included: bool,
) -> ContextPackAnomaly {
    let action = if included { "included" } else { "excluded" };
    ContextPackAnomaly {
        cell_id: Some(cell_id),
        code: ContextPackAnomalyCode::TokenOverload,
        message: "large cell exceeded the available token budget".to_owned(),
        why_excluded: Some(format!(
            "large_cell_policy={} {}; original_estimated_tokens={} remaining_tokens={}",
            policy.as_str(),
            action,
            original_tokens,
            remaining_tokens
        )),
    }
}

fn truncate_bytes(payload: &[u8], len: usize, marker: &[u8]) -> Vec<u8> {
    let mut end = len.min(payload.len());
    while end > 0 && std::str::from_utf8(&payload[..end]).is_err() {
        end -= 1;
    }
    let mut out = payload[..end].to_vec();
    out.extend_from_slice(marker);
    out
}
