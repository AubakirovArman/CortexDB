use super::types::EnterpriseRagQuestionType;

#[derive(Clone, Debug, Default)]
pub(super) struct CategoryScores {
    values: Vec<(EnterpriseRagQuestionType, u32, Vec<&'static str>)>,
}

impl CategoryScores {
    pub(super) fn add(
        &mut self,
        question_type: EnterpriseRagQuestionType,
        score: u32,
        signal: &'static str,
    ) {
        if score == 0 {
            return;
        }
        if let Some((_, value, signals)) = self
            .values
            .iter_mut()
            .find(|(existing, _, _)| *existing == question_type)
        {
            *value = value.saturating_add(score);
            signals.push(signal);
        } else {
            self.values.push((question_type, score, vec![signal]));
        }
    }

    pub(super) fn best_type(&self) -> EnterpriseRagQuestionType {
        self.values
            .iter()
            .max_by_key(|(question_type, score, _)| (*score, priority(*question_type)))
            .map(|(question_type, _, _)| *question_type)
            .unwrap_or(EnterpriseRagQuestionType::Basic)
    }

    pub(super) fn confidence_q16(&self, question_type: EnterpriseRagQuestionType) -> u16 {
        let best = self.score(question_type);
        if best == 0 {
            return 32_768;
        }
        let second = self
            .values
            .iter()
            .filter(|(candidate, _, _)| *candidate != question_type)
            .map(|(_, score, _)| *score)
            .max()
            .unwrap_or(0);
        let total = best.saturating_add(second).max(1);
        ((u64::from(best) * 65_535) / u64::from(total)) as u16
    }

    fn score(&self, question_type: EnterpriseRagQuestionType) -> u32 {
        self.values
            .iter()
            .find(|(candidate, _, _)| *candidate == question_type)
            .map(|(_, score, _)| *score)
            .unwrap_or(0)
    }

    pub(super) fn signals(&self, question_type: EnterpriseRagQuestionType) -> Vec<&'static str> {
        self.values
            .iter()
            .find(|(candidate, _, _)| *candidate == question_type)
            .map(|(_, _, signals)| signals.clone())
            .unwrap_or_default()
    }
}

fn priority(question_type: EnterpriseRagQuestionType) -> u8 {
    match question_type {
        EnterpriseRagQuestionType::InfoNotFound => 10,
        EnterpriseRagQuestionType::Miscellaneous => 9,
        EnterpriseRagQuestionType::HighLevel => 8,
        EnterpriseRagQuestionType::Completeness => 7,
        EnterpriseRagQuestionType::ConflictingInfo => 6,
        EnterpriseRagQuestionType::Constrained => 5,
        EnterpriseRagQuestionType::ProjectRelated => 4,
        EnterpriseRagQuestionType::IntraDocumentReasoning => 3,
        EnterpriseRagQuestionType::Semantic => 2,
        EnterpriseRagQuestionType::Basic => 1,
    }
}
