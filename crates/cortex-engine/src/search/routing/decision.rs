use super::SearchQueryIntent;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchRouteStrategy {
    Keyword,
    VectorAnn,
    VectorExact,
    Hybrid,
}

impl SearchRouteStrategy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Keyword => "keyword",
            Self::VectorAnn => "vector_ann",
            Self::VectorExact => "vector_exact",
            Self::Hybrid => "hybrid",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SearchRouteDecision {
    pub requested_mode: String,
    pub selected_strategy: SearchRouteStrategy,
    pub reason: &'static str,
    pub text_available: bool,
    pub vector_available: bool,
    pub intent: SearchQueryIntent,
    pub policy: super::SearchRoutePolicy,
}

impl SearchRouteDecision {
    pub fn search_mode(&self) -> &'static str {
        self.selected_strategy.as_str()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchRouteInput<'a> {
    pub requested_mode: &'a str,
    pub algorithm: &'a str,
    pub text_available: bool,
    pub vector_available: bool,
}
