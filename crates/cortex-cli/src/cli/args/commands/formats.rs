use clap::ValueEnum;

#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub(in crate::cli) enum ContextOutputFormat {
    Summary,
    Json,
    Prompt,
    Markdown,
}

impl ContextOutputFormat {
    pub(in crate::cli) fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Json => "json",
            Self::Prompt => "prompt",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Copy, Clone, Debug, ValueEnum)]
pub(in crate::cli) enum VerificationOutputFormat {
    Summary,
    Json,
    Markdown,
    Audit,
}

impl VerificationOutputFormat {
    pub(in crate::cli) fn as_str(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Audit => "audit",
        }
    }
}
