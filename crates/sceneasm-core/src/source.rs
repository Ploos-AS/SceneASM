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
        Self { files: vec![SourceFile { id: 0, name: main_name.into() }] }
    }

    pub fn add_file(&mut self, name: impl Into<String>) -> u32 {
        let id = self.files.len() as u32;
        self.files.push(SourceFile { id, name: name.into() });
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
    fn default() -> Self { Self::new("<input>") }
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
