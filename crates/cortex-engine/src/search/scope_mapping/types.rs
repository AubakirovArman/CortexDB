#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum QueryScopeField {
    Source,
    Scope,
    Project,
    Team,
    Owner,
    Topic,
    Entity,
}

impl QueryScopeField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Scope => "scope",
            Self::Project => "project",
            Self::Team => "team",
            Self::Owner => "owner",
            Self::Topic => "topic",
            Self::Entity => "entity",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryScopeDirective {
    pub field: QueryScopeField,
    pub value: String,
    pub confidence_q16: u16,
    pub hard_filter: bool,
    pub terms: Vec<String>,
    pub reason: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryScopeMapping {
    pub query: String,
    pub directives: Vec<QueryScopeDirective>,
}

impl QueryScopeMapping {
    pub fn has_scope_filter(&self) -> bool {
        self.directives.iter().any(|directive| {
            matches!(
                directive.field,
                QueryScopeField::Source
                    | QueryScopeField::Scope
                    | QueryScopeField::Project
                    | QueryScopeField::Team
                    | QueryScopeField::Topic
                    | QueryScopeField::Entity
            )
        })
    }

    pub fn source_filters(&self) -> Vec<String> {
        self.values_for_field(QueryScopeField::Source)
    }

    pub fn project_filters(&self) -> Vec<String> {
        self.values_for_field(QueryScopeField::Project)
    }

    pub fn scope_filters(&self) -> Vec<String> {
        self.values_for_field(QueryScopeField::Scope)
    }

    fn values_for_field(&self, field: QueryScopeField) -> Vec<String> {
        self.directives
            .iter()
            .filter(|directive| directive.field == field)
            .map(|directive| directive.value.clone())
            .collect()
    }
}
