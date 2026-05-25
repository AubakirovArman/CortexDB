use super::BindError;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindDiagnostic {
    pub code: &'static str,
    pub safe_message: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BindDiagnosticExport {
    pub code: &'static str,
    pub message: &'static str,
    pub internal_detail: Option<String>,
}

impl BindDiagnostic {
    pub fn from_error(error: &BindError) -> Self {
        Self {
            code: error.code(),
            safe_message: error.safe_message(),
        }
    }

    pub fn safe_export(&self) -> BindDiagnosticExport {
        BindDiagnosticExport {
            code: self.code,
            message: self.safe_message,
            internal_detail: None,
        }
    }

    pub fn internal_export(&self, detail: impl Into<String>) -> BindDiagnosticExport {
        BindDiagnosticExport {
            code: self.code,
            message: self.safe_message,
            internal_detail: Some(detail.into()),
        }
    }
}
