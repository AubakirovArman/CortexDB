use super::{VerificationGuard, VerificationNumericConflict, VerificationReport};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerificationReportExportFormat {
    Markdown,
    Audit,
}

impl VerificationReport {
    pub fn export(&self, format: VerificationReportExportFormat) -> String {
        match format {
            VerificationReportExportFormat::Markdown => self.to_markdown(),
            VerificationReportExportFormat::Audit => self.to_audit_text(),
        }
    }

    pub fn to_markdown(&self) -> String {
        let mut lines = vec![
            "# CortexDB Verification Report".to_owned(),
            String::new(),
            format!("- fact: `{}`", escape_inline(&self.fact)),
            format!("- status: `{}`", self.status.as_str()),
            format!("- supporting_evidence: `{}`", self.evidence.len()),
            format!(
                "- contradicting_evidence: `{}`",
                self.contradicting_evidence.len()
            ),
            format!("- guards: `{}`", self.guards.len()),
            format!("- numeric_conflicts: `{}`", self.numeric_conflicts.len()),
            String::new(),
            "## Supporting Evidence".to_owned(),
        ];
        if self.evidence.is_empty() {
            lines.push("- none".to_owned());
        } else {
            for evidence in &self.evidence {
                lines.push(format!(
                    "- cell_id=`{}` matched_terms=`{}` source_trust=`{}` (`{}`) citation=`{}`",
                    evidence.cell_id.0,
                    evidence.matched_terms,
                    evidence.source_trust_category.as_str(),
                    evidence.source_trust_q16,
                    option_or_null(evidence.citation.as_deref())
                ));
            }
        }
        lines.push(String::new());
        lines.push("## Contradicting Evidence".to_owned());
        if self.contradicting_evidence.is_empty() {
            lines.push("- none".to_owned());
        } else {
            for evidence in &self.contradicting_evidence {
                lines.push(format!(
                    "- cell_id=`{}` matched_terms=`{}` source_trust=`{}` (`{}`) citation=`{}`",
                    evidence.cell_id.0,
                    evidence.matched_terms,
                    evidence.source_trust_category.as_str(),
                    evidence.source_trust_q16,
                    option_or_null(evidence.citation.as_deref())
                ));
            }
        }
        push_numeric_conflicts(&mut lines, &self.numeric_conflicts);
        push_guards(&mut lines, &self.guards);
        lines.join("\n")
    }

    pub fn to_audit_text(&self) -> String {
        let mut lines = vec![
            "CortexDB Verification Audit v1".to_owned(),
            format!("fact={}", escape_value(&self.fact)),
            format!("status={}", self.status.as_str()),
            format!("supporting_count={}", self.evidence.len()),
            format!("contradicting_count={}", self.contradicting_evidence.len()),
            format!("guard_count={}", self.guards.len()),
            format!("numeric_conflict_count={}", self.numeric_conflicts.len()),
        ];
        for (index, evidence) in self.evidence.iter().enumerate() {
            lines.push(format!("supporting.{index}.cell_id={}", evidence.cell_id.0));
            lines.push(format!(
                "supporting.{index}.matched_terms={}",
                evidence.matched_terms
            ));
            lines.push(format!(
                "supporting.{index}.source_trust_q16={}",
                evidence.source_trust_q16
            ));
            lines.push(format!(
                "supporting.{index}.source_trust_category={}",
                evidence.source_trust_category.as_str()
            ));
            lines.push(format!(
                "supporting.{index}.citation={}",
                option_or_null(evidence.citation.as_deref())
            ));
        }
        for (index, evidence) in self.contradicting_evidence.iter().enumerate() {
            lines.push(format!(
                "contradicting.{index}.cell_id={}",
                evidence.cell_id.0
            ));
            lines.push(format!(
                "contradicting.{index}.matched_terms={}",
                evidence.matched_terms
            ));
            lines.push(format!(
                "contradicting.{index}.source_trust_q16={}",
                evidence.source_trust_q16
            ));
            lines.push(format!(
                "contradicting.{index}.source_trust_category={}",
                evidence.source_trust_category.as_str()
            ));
            lines.push(format!(
                "contradicting.{index}.citation={}",
                option_or_null(evidence.citation.as_deref())
            ));
        }
        for (index, conflict) in self.numeric_conflicts.iter().enumerate() {
            lines.push(format!(
                "numeric_conflict.{index}.cell_id={}",
                conflict.cell_id.0
            ));
            lines.push(format!(
                "numeric_conflict.{index}.metric={}",
                escape_value(&conflict.metric)
            ));
            lines.push(format!(
                "numeric_conflict.{index}.left={}",
                escape_value(&conflict.left)
            ));
            lines.push(format!(
                "numeric_conflict.{index}.right={}",
                escape_value(&conflict.right)
            ));
        }
        for (index, guard) in self.guards.iter().enumerate() {
            lines.push(format!(
                "guard.{index}.cell_id={}",
                guard
                    .cell_id
                    .map(|cell_id| cell_id.0.to_string())
                    .unwrap_or_else(|| "null".to_owned())
            ));
            lines.push(format!("guard.{index}.code={}", guard.code.as_str()));
            lines.push(format!(
                "guard.{index}.message={}",
                escape_value(&guard.message)
            ));
        }
        lines.join("\n")
    }
}

fn push_numeric_conflicts(lines: &mut Vec<String>, conflicts: &[VerificationNumericConflict]) {
    lines.push(String::new());
    lines.push("## Numeric Conflicts".to_owned());
    if conflicts.is_empty() {
        lines.push("- none".to_owned());
        return;
    }
    for conflict in conflicts {
        lines.push(format!(
            "- cell_id=`{}` metric=`{}` left=`{}` right=`{}`",
            conflict.cell_id.0,
            escape_inline(&conflict.metric),
            escape_inline(&conflict.left),
            escape_inline(&conflict.right)
        ));
    }
}

fn push_guards(lines: &mut Vec<String>, guards: &[VerificationGuard]) {
    lines.push(String::new());
    lines.push("## Guards".to_owned());
    if guards.is_empty() {
        lines.push("- none".to_owned());
        return;
    }
    for guard in guards {
        lines.push(format!(
            "- code=`{}` cell_id=`{}` message={}",
            guard.code.as_str(),
            guard
                .cell_id
                .map(|cell_id| cell_id.0.to_string())
                .unwrap_or_else(|| "null".to_owned()),
            escape_inline(&guard.message)
        ));
    }
}

fn option_or_null(value: Option<&str>) -> String {
    value.unwrap_or("null").replace('\n', "\\n")
}

fn escape_inline(value: &str) -> String {
    value.replace('`', "\\`").replace('\n', " ")
}

fn escape_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\n', "\\n")
}
