#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn new(main_name: impl Into<String>) -> Self {
        Self {
            files: vec![SourceFile {
                id: 0,
                name: main_name.into(),
            }],
        }
    }

    pub fn add_file(&mut self, name: impl Into<String>) -> u32 {
        let id = self.files.len() as u32;
        self.files.push(SourceFile {
            id,
            name: name.into(),
        });
        id
    }

    pub fn file(&self, id: u32) -> Option<&SourceFile> {
        self.files.get(id as usize).filter(|file| file.id == id)
    }

    pub fn files(&self) -> &[SourceFile] {
        &self.files
    }
}

impl Default for SourceMap {
    fn default() -> Self {
        Self::new("<input>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_source_has_stable_zero_id() {
        let mut map = SourceMap::new("demo.asm");
        let include = map.add_file("macros/raster.asm");
        assert_eq!(map.file(0).unwrap().name, "demo.asm");
        assert_eq!(include, 1);
        assert_eq!(map.file(include).unwrap().name, "macros/raster.asm");
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpansionKind {
    Include,
    Macro,
    Generated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpansionFrame {
    pub kind: ExpansionKind,
    pub name: String,
    pub call_site: crate::SourceSpan,
    pub definition_site: Option<crate::SourceSpan>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExpansionTrace {
    pub frames: Vec<ExpansionFrame>,
}

impl ExpansionTrace {
    pub fn push(&mut self, frame: ExpansionFrame) {
        self.frames.push(frame);
    }

    pub fn outermost_call_site(&self) -> Option<&crate::SourceSpan> {
        self.frames.first().map(|frame| &frame.call_site)
    }

    pub fn innermost_call_site(&self) -> Option<&crate::SourceSpan> {
        self.frames.last().map(|frame| &frame.call_site)
    }
}

#[cfg(test)]
mod expansion_tests {
    use super::*;

    #[test]
    fn expansion_trace_preserves_call_chain() {
        let mut trace = ExpansionTrace::default();
        trace.push(ExpansionFrame {
            kind: ExpansionKind::Macro,
            name: "stable_raster".into(),
            call_site: crate::SourceSpan {
                file_id: 0,
                line: 73,
                column_start: 5,
                column_end: 18,
            },
            definition_site: Some(crate::SourceSpan {
                file_id: 1,
                line: 21,
                column_start: 1,
                column_end: 14,
            }),
        });
        assert_eq!(trace.outermost_call_site().unwrap().line, 73);
        assert_eq!(trace.innermost_call_site().unwrap().line, 73);
    }
}
