use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::diagnostics::Diagnostic;

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub path: PathBuf,
    pub text: String,
}

#[derive(Debug, Default)]
pub struct SourceManager {
    files: Vec<SourceFile>,
    by_path: HashMap<PathBuf, usize>,
}

impl SourceManager {
    pub fn load(&mut self, path: &Path) -> Result<usize, Diagnostic> {
        let canonical = path.canonicalize().map_err(|error| {
            Diagnostic::plain(
                "Module Error",
                format!("could not resolve {}: {error}", path.display()),
                "Check the module path and file name.",
            )
        })?;
        if let Some(id) = self.by_path.get(&canonical) {
            return Ok(*id);
        }
        let text = std::fs::read_to_string(&canonical).map_err(|error| {
            Diagnostic::plain(
                "File Error",
                format!("could not read {}: {error}", canonical.display()),
                "Check that the file exists and is readable.",
            )
        })?;
        let id = self.files.len();
        self.files.push(SourceFile {
            path: canonical.clone(),
            text,
        });
        self.by_path.insert(canonical, id);
        Ok(id)
    }

    pub fn get(&self, id: usize) -> &SourceFile {
        &self.files[id]
    }
}
