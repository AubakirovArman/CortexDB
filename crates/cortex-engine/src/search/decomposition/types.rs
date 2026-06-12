#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QuestionRequirementKind {
    Anchor,
    Slot,
    Subquestion,
    Question,
}

impl QuestionRequirementKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Anchor => "anchor",
            Self::Slot => "slot",
            Self::Subquestion => "subquestion",
            Self::Question => "question",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionRequirement {
    pub id: String,
    pub kind: QuestionRequirementKind,
    pub text: String,
    pub tokens: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QuestionDecomposition {
    pub question: String,
    pub requirements: Vec<QuestionRequirement>,
    pub anchors: Vec<String>,
    pub slots: Vec<String>,
    pub subquestions: Vec<String>,
    pub multi_requirement: bool,
}
