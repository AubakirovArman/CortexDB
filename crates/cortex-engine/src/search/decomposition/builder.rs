use std::collections::BTreeSet;

use super::super::tokenize;
use super::anchors::precise_anchors;
use super::normalize::{
    clean_part, compact_whitespace, normalize_for_key, normalize_for_substring,
};
use super::slots::expected_slots;
use super::split::split_subquestions;
use super::types::{QuestionDecomposition, QuestionRequirement, QuestionRequirementKind};

pub fn decompose_enterprise_rag_question(question: &str) -> QuestionDecomposition {
    let question = compact_whitespace(question);
    let mut builder = RequirementBuilder::default();

    for anchor in precise_anchors(&question).into_iter().take(12) {
        builder.push(QuestionRequirementKind::Anchor, anchor);
    }
    for slot in expected_slots(&question) {
        builder.push(QuestionRequirementKind::Slot, slot);
    }
    for subquestion in split_subquestions(&question).into_iter().take(8) {
        builder.push(QuestionRequirementKind::Subquestion, subquestion);
    }
    if builder.is_empty() {
        builder.push(QuestionRequirementKind::Question, question.clone());
    }

    let requirements = builder.finish();
    let anchors = texts_by_kind(&requirements, QuestionRequirementKind::Anchor);
    let slots = texts_by_kind(&requirements, QuestionRequirementKind::Slot);
    let subquestions = texts_by_kind(&requirements, QuestionRequirementKind::Subquestion);
    let non_anchor_count = requirements
        .iter()
        .filter(|item| item.kind != QuestionRequirementKind::Anchor)
        .count();
    let multi_requirement = subquestions.len() > 1 || non_anchor_count > 1;
    QuestionDecomposition {
        question,
        requirements,
        anchors,
        slots,
        subquestions,
        multi_requirement,
    }
}

pub fn covered_requirement_ids(decomposition: &QuestionDecomposition, text: &str) -> Vec<String> {
    let normalized_text = normalize_for_substring(text);
    let doc_terms = tokenize(text).into_iter().collect::<BTreeSet<_>>();
    let mut covered = Vec::new();
    for requirement in &decomposition.requirements {
        if requirement.tokens.is_empty() {
            continue;
        }
        if requirement.kind == QuestionRequirementKind::Anchor {
            let needle = normalize_for_substring(&requirement.text);
            if !needle.is_empty() && normalized_text.contains(&needle) {
                covered.push(requirement.id.clone());
                continue;
            }
        }
        let hits = requirement
            .tokens
            .iter()
            .filter(|term| doc_terms.contains(*term))
            .count();
        let required = if requirement.tokens.len() <= 2 {
            1
        } else {
            (requirement.tokens.len() * 45).div_ceil(100).max(2)
        };
        if hits >= required {
            covered.push(requirement.id.clone());
        }
    }
    covered
}

#[derive(Default)]
struct RequirementBuilder {
    requirements: Vec<QuestionRequirement>,
    seen: BTreeSet<String>,
}

impl RequirementBuilder {
    fn is_empty(&self) -> bool {
        self.requirements.is_empty()
    }

    fn push(&mut self, kind: QuestionRequirementKind, text: String) {
        let text = clean_part(&text);
        let tokens = requirement_tokens(&text);
        if text.is_empty() || tokens.is_empty() {
            return;
        }
        let key = format!("{}:{}", kind.as_str(), normalize_for_key(&text));
        if !self.seen.insert(key) {
            return;
        }
        self.requirements.push(QuestionRequirement {
            id: format!("u{:02}", self.requirements.len() + 1),
            kind,
            text,
            tokens,
        });
    }

    fn finish(self) -> Vec<QuestionRequirement> {
        self.requirements
    }
}

fn texts_by_kind(
    requirements: &[QuestionRequirement],
    kind: QuestionRequirementKind,
) -> Vec<String> {
    requirements
        .iter()
        .filter(|item| item.kind == kind)
        .map(|item| item.text.clone())
        .collect()
}

pub(super) fn requirement_tokens(text: &str) -> Vec<String> {
    tokenize(text)
        .into_iter()
        .filter(|term| !is_requirement_stopword(term))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn is_requirement_stopword(term: &str) -> bool {
    matches!(
        term,
        "who"
            | "what"
            | "which"
            | "where"
            | "when"
            | "why"
            | "how"
            | "give"
            | "tell"
            | "show"
            | "find"
            | "include"
            | "including"
            | "does"
            | "did"
            | "was"
            | "were"
            | "are"
            | "for"
            | "with"
            | "from"
            | "into"
            | "about"
            | "is"
            | "be"
            | "can"
            | "on"
            | "by"
            | "if"
            | "has"
            | "have"
            | "i"
            | "s"
            | "t"
            | "e"
            | "g"
            | "any"
            | "each"
            | "those"
            | "up"
            | "so"
            | "get"
            | "this"
            | "that"
            | "their"
            | "our"
    )
}
