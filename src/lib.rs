#![allow(clippy::result_large_err)]

pub mod ast;
pub mod checker;
pub mod diagnostics;
pub mod evaluator;
pub mod hir;
pub mod lexer;
pub mod modules;
mod numeric_vm;
pub mod parser;
pub mod source;
pub mod token;

use std::path::Path;

use diagnostics::Diagnostic;
use evaluator::Evaluator;
use lexer::Lexer;
use parser::Parser;

pub fn parse(source: &str, file: &str) -> Result<ast::Program, Diagnostic> {
    let tokens = Lexer::new(source, file).scan()?;
    Parser::new(tokens, source, file).parse()
}

pub fn check_source(source: &str, file: &str) -> Result<(), Diagnostic> {
    let program = parse(source, file)?;
    require_path_for_imports(&program)?;
    checker::Checker::new(source, file).check(&program)
}

pub fn run_source(source: &str, file: &str) -> Result<(), Diagnostic> {
    let program = parse(source, file)?;
    require_path_for_imports(&program)?;
    let mut evaluator = Evaluator::new(source, file, true);
    evaluator.run(&program)
}

pub fn compile_path(path: &Path) -> Result<hir::HirProgram, Diagnostic> {
    let graph = modules::ModuleResolver::load(path)?;
    hir::lower(graph)
}

pub fn check_path(path: &Path) -> Result<hir::HirProgram, Diagnostic> {
    let hir = compile_path(path)?;
    checker::Checker::new(&hir.entry_source, &hir.entry_file).check(&hir.program)?;
    Ok(hir)
}

pub fn run_path(path: &Path) -> Result<(), Diagnostic> {
    let hir = check_path(path)?;
    let mut evaluator = Evaluator::new(&hir.entry_source, &hir.entry_file, true);
    evaluator.run(&hir.program)
}

fn require_path_for_imports(program: &ast::Program) -> Result<(), Diagnostic> {
    if program.imports.is_empty() {
        Ok(())
    } else {
        Err(Diagnostic::plain(
            "Module Error",
            "imports require a source file path",
            "Use the path-based compiler API or `shine run <file.shn>`.",
        ))
    }
}

pub fn load(path: &Path) -> Result<String, Diagnostic> {
    std::fs::read_to_string(path).map_err(|error| {
        Diagnostic::plain(
            "File Error",
            format!("could not read {}: {error}", path.display()),
            "Check that the file exists and is readable.",
        )
    })
}
