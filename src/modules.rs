use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::{
    ast::{ImportDecl, Program},
    diagnostics::Diagnostic,
    parse,
    source::SourceManager,
};

#[derive(Debug, Clone)]
pub struct ResolvedImport {
    pub declaration: ImportDecl,
    pub module: usize,
}

#[derive(Debug, Clone)]
pub struct Module {
    pub path: PathBuf,
    pub source: String,
    pub program: Program,
    pub imports: Vec<ResolvedImport>,
}

#[derive(Debug, Clone)]
pub struct ModuleGraph {
    pub modules: Vec<Module>,
    pub entry: usize,
    pub source_root: PathBuf,
}

pub struct ModuleResolver {
    source_root: PathBuf,
    sources: SourceManager,
    modules: Vec<Module>,
    resolved: HashMap<PathBuf, usize>,
}

impl ModuleResolver {
    pub fn load(entry: &Path) -> Result<ModuleGraph, Diagnostic> {
        let entry = entry.canonicalize().map_err(|error| {
            Diagnostic::plain(
                "File Error",
                format!("could not resolve {}: {error}", entry.display()),
                "Check that the entry file exists.",
            )
        })?;
        let source_root = entry.parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut resolver = Self {
            source_root,
            sources: SourceManager::default(),
            modules: vec![],
            resolved: HashMap::new(),
        };
        let entry_id = resolver.resolve(&entry, &mut vec![])?;
        Ok(ModuleGraph {
            modules: resolver.modules,
            entry: entry_id,
            source_root: resolver.source_root,
        })
    }

    fn resolve(&mut self, path: &Path, stack: &mut Vec<PathBuf>) -> Result<usize, Diagnostic> {
        let canonical = path.canonicalize().map_err(|error| {
            Diagnostic::plain(
                "Module Error",
                format!("could not resolve {}: {error}", path.display()),
                "Create the module file or correct the import path.",
            )
        })?;
        if !canonical.starts_with(&self.source_root) {
            return Err(Diagnostic::plain(
                "Module Error",
                format!("module path escapes the source root: {}", path.display()),
                "Imports must resolve inside the entry file's source directory.",
            ));
        }
        if let Some(id) = self.resolved.get(&canonical) {
            return Ok(*id);
        }
        if let Some(position) = stack.iter().position(|item| item == &canonical) {
            let mut cycle: Vec<String> = stack[position..]
                .iter()
                .map(|item| item.display().to_string())
                .collect();
            cycle.push(canonical.display().to_string());
            return Err(Diagnostic::plain(
                "Module Error",
                format!("cyclic import: {}", cycle.join(" -> ")),
                "Move shared declarations into a third module.",
            ));
        }

        stack.push(canonical.clone());
        let source_id = self.sources.load(&canonical)?;
        let file = self.sources.get(source_id).clone();
        let program = parse(&file.text, &file.path.display().to_string())?;
        let declarations = program.imports.clone();
        let mut imports = Vec::with_capacity(declarations.len());
        for declaration in declarations {
            let dependency_path = self.resolve_import_path(&declaration.module)?;
            let dependency = self.resolve(&dependency_path, stack)?;
            imports.push(ResolvedImport {
                declaration,
                module: dependency,
            });
        }
        stack.pop();

        let id = self.modules.len();
        self.modules.push(Module {
            path: canonical.clone(),
            source: file.text,
            program,
            imports,
        });
        self.resolved.insert(canonical, id);
        Ok(id)
    }

    fn resolve_import_path(&self, module: &[String]) -> Result<PathBuf, Diagnostic> {
        let relative = module
            .iter()
            .fold(PathBuf::new(), |path, part| path.join(part));
        let file = self.source_root.join(&relative).with_extension("shn");
        if file.is_file() {
            return Ok(file);
        }
        let directory_module = self.source_root.join(relative).join("mod.shn");
        if directory_module.is_file() {
            return Ok(directory_module);
        }
        Err(Diagnostic::plain(
            "Module Error",
            format!("module `{}` was not found", module.join(".")),
            format!("Create {} or a matching mod.shn file.", file.display()),
        ))
    }
}
