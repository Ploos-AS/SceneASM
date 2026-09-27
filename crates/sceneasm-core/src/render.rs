use crate::diagnostic::{Diagnostic, Severity};
use crate::source::SourceMap;

pub fn render(diagnostic: &Diagnostic, sources: &SourceMap) -> String {
    let severity = match diagnostic.severity {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
        Severity::Hint => "hint",
    };
    let mut out = format!("{severity}[{}]: {}", diagnostic.code, diagnostic.message);

    if let Some(primary) = &diagnostic.primary {
        let file = sources.file(primary.span.file_id)
            .map(|file| file.name.as_str())
            .unwrap_or("<unknown>");
        out.push_str(&format!(
            "\n --> {file}:{}:{}-{}",
            primary.span.line, primary.span.column_start, primary.span.column_end
        ));
        if let Some(message) = &primary.message {
            out.push_str(&format!("\n  = {message}"));
        }
    }

    if let Some(timing) = &diagnostic.timing {
        out.push_str(&format!(
            "\n  = timing: raster {}, nominal {}, stall +{}, actual {} cycles",
            timing.raster_line, timing.nominal_cycles, timing.stalled_cycles, timing.actual_cycles
        ));
    }

    for note in &diagnostic.notes {
        out.push_str(&format!("\n  = note: {note}"));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{Diagnostic, TimingDiagnostic};
    use crate::SourceSpan;

    #[test]
    fn renders_source_and_timing_without_losing_structure() {
        let sources = SourceMap::new("demo.asm");
        let diagnostic = Diagnostic::info("C64_VIC_STALL", "VIC-II stalls instruction")
            .with_primary(
                SourceSpan { file_id: 0, line: 37, column_start: 5, column_end: 17 },
                "instruction stalls here",
            )
            .with_timing(TimingDiagnostic {
                raster_line: 51,
                nominal_cycles: 4,
                stalled_cycles: 3,
                actual_cycles: 7,
            });
        let rendered = render(&diagnostic, &sources);
        assert!(rendered.contains("info[C64_VIC_STALL]"));
        assert!(rendered.contains("demo.asm:37:5-17"));
        assert!(rendered.contains("stall +3"));
    }
}
