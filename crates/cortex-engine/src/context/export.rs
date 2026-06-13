use super::ContextPack;

mod json_export;
mod markdown;
mod prompt;
mod text;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextPackExportFormat {
    Json,
    Prompt,
    Markdown,
}

impl ContextPack {
    pub fn export(&self, format: ContextPackExportFormat) -> String {
        match format {
            ContextPackExportFormat::Json => self.to_json(),
            ContextPackExportFormat::Prompt => self.to_agent_prompt(),
            ContextPackExportFormat::Markdown => self.to_markdown(),
        }
    }

    pub fn to_json(&self) -> String {
        json_export::to_json(self)
    }

    pub fn to_agent_prompt(&self) -> String {
        prompt::to_agent_prompt(self)
    }

    pub fn to_markdown(&self) -> String {
        markdown::to_markdown(self)
    }
}
