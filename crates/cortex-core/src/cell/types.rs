#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KnowledgeCellType {
    DocumentBlock,
    Table,
    Fact,
    Entity,
    Relation,
    Memory,
    Feedback,
    Tool,
    SourceRef,
    Raw,
}

impl KnowledgeCellType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DocumentBlock => "document_block",
            Self::Table => "table",
            Self::Fact => "fact",
            Self::Entity => "entity",
            Self::Relation => "relation",
            Self::Memory => "memory",
            Self::Feedback => "feedback",
            Self::Tool => "tool",
            Self::SourceRef => "source_ref",
            Self::Raw => "raw",
        }
    }

    pub(super) fn to_u8(self) -> u8 {
        match self {
            Self::DocumentBlock => 1,
            Self::Table => 2,
            Self::Fact => 3,
            Self::Entity => 4,
            Self::Relation => 5,
            Self::Memory => 6,
            Self::Feedback => 7,
            Self::Tool => 8,
            Self::SourceRef => 9,
            Self::Raw => 10,
        }
    }

    pub(super) fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::DocumentBlock),
            2 => Some(Self::Table),
            3 => Some(Self::Fact),
            4 => Some(Self::Entity),
            5 => Some(Self::Relation),
            6 => Some(Self::Memory),
            7 => Some(Self::Feedback),
            8 => Some(Self::Tool),
            9 => Some(Self::SourceRef),
            10 => Some(Self::Raw),
            _ => None,
        }
    }
}

impl std::str::FromStr for KnowledgeCellType {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "document_block" => Ok(Self::DocumentBlock),
            "table" => Ok(Self::Table),
            "fact" => Ok(Self::Fact),
            "entity" => Ok(Self::Entity),
            "relation" => Ok(Self::Relation),
            "memory" => Ok(Self::Memory),
            "feedback" => Ok(Self::Feedback),
            "tool" => Ok(Self::Tool),
            "source_ref" => Ok(Self::SourceRef),
            "raw" => Ok(Self::Raw),
            _ => Err(()),
        }
    }
}
