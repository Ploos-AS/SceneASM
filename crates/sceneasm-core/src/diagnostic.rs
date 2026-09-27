use crate::SourceSpan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub span: SourceSpan,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimingDiagnostic {
    pub raster_line: u16,
    pub nominal_cycles: u16,
    pub stalled_cycles: u16,
    pub actual_cycles: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub primary: Option<Label>,
    pub secondary: Vec<Label>,
    pub notes: Vec<String>,
    pub timing: Option<TimingDiagnostic>,
}

impl Diagnostic {
    pub fn error(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            primary: None,
            secondary: Vec::new(),
            notes: Vec::new(),
            timing: None,
        }
    }

    pub fn with_primary(mut self, span: SourceSpan, message: impl Into<String>) -> Self {
        self.primary = Some(Label { span, message: Some(message.into()) });
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_timing(mut self, timing: TimingDiagnostic) -> Self {
        self.timing = Some(timing);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_timing_diagnostic_keeps_editor_data() {
        let span = SourceSpan { file_id: 0, line: 37, column_start: 5, column_end: 17 };
        let diagnostic = Diagnostic::error("C64_TIMING", "VIC-II stalls instruction")
            .with_primary(span.clone(), "instruction is stretched here")
            .with_note("consider moving work outside the badline window")
            .with_timing(TimingDiagnostic {
                raster_line: 51,
                nominal_cycles: 4,
                stalled_cycles: 3,
                actual_cycles: 7,
            });
        assert_eq!(diagnostic.primary.unwrap().span, span);
        assert_eq!(diagnostic.timing.unwrap().actual_cycles, 7);
    }
}
