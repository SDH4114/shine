#![allow(clippy::result_large_err)]

use std::{
    env,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::ExitCode,
};

use shine_lang::{check_path as check_program_path, load, run_path as run_program_path};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<(), shine_lang::diagnostics::Diagnostic> {
    let args: Vec<String> = env::args().collect();
    let executable = PathBuf::from(&args[0]);
    if executable.file_stem() != Some(OsStr::new("shine")) {
        let source_path = executable.with_extension("shn");
        if source_path.exists() {
            return run_path(&source_path);
        }
        let bundle = executable.with_extension("shine-src");
        let entry_marker = bundle.join(".entry");
        if entry_marker.is_file() {
            let entry = std::fs::read_to_string(&entry_marker).map_err(file_error)?;
            return run_path(&bundle.join(entry.trim()));
        }
    }
    let Some(command) = args.get(1).map(String::as_str) else {
        print_help();
        return Ok(());
    };
    match command {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "version" | "--version" | "-V" => {
            println!("Shine {VERSION}");
            Ok(())
        }
        "run" => run_path(required_path(&args, "run")?),
        "check" => check_path(required_path(&args, "check")?),
        "build" => build_path(required_path(&args, "build")?),
        "fmt" => format_path(required_path(&args, "fmt")?),
        "new" => new_project(required_path(&args, "new")?),
        "test" => test_project(
            args.get(2)
                .map(PathBuf::from)
                .as_deref()
                .unwrap_or(Path::new(".")),
        ),
        other => Err(shine_lang::diagnostics::Diagnostic::plain(
            "CLI Error",
            format!("unknown command `{other}`"),
            "Run `shine help` to see available commands.",
        )),
    }
}

fn required_path<'a>(
    args: &'a [String],
    command: &str,
) -> Result<&'a Path, shine_lang::diagnostics::Diagnostic> {
    args.get(2).map(Path::new).ok_or_else(|| {
        shine_lang::diagnostics::Diagnostic::plain(
            "CLI Error",
            format!("`shine {command}` requires a path"),
            format!("Run `shine {command} <path>`."),
        )
    })
}

fn run_path(path: &Path) -> Result<(), shine_lang::diagnostics::Diagnostic> {
    run_program_path(path)
}
fn check_path(path: &Path) -> Result<(), shine_lang::diagnostics::Diagnostic> {
    let hir = check_program_path(path)?;
    println!(
        "Checked {} successfully ({} module(s)).",
        path.display(),
        hir.module_count
    );
    Ok(())
}

fn build_path(path: &Path) -> Result<(), shine_lang::diagnostics::Diagnostic> {
    let hir = check_program_path(path)?;
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("program");
    let dir = PathBuf::from("target/shine");
    std::fs::create_dir_all(&dir).map_err(file_error)?;
    let current = env::current_exe().map_err(file_error)?;
    let binary = dir.join(stem);
    std::fs::copy(current, &binary).map_err(file_error)?;
    if hir.module_count == 1 {
        let source = load(path)?;
        std::fs::write(dir.join(format!("{stem}.shn")), source).map_err(file_error)?;
    } else {
        let bundle = dir.join(format!("{stem}.shine-src"));
        std::fs::create_dir_all(&bundle).map_err(file_error)?;
        for (relative, source) in &hir.bundle_sources {
            let destination = bundle.join(relative);
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent).map_err(file_error)?;
            }
            std::fs::write(destination, source).map_err(file_error)?;
        }
        std::fs::write(
            bundle.join(".entry"),
            hir.entry_relative.display().to_string(),
        )
        .map_err(file_error)?;
    }
    println!(
        "Built {} (tree-walking executable bundle, {} module(s)).",
        binary.display(),
        hir.module_count
    );
    Ok(())
}

fn format_path(path: &Path) -> Result<(), shine_lang::diagnostics::Diagnostic> {
    let source = load(path)?;
    shine_lang::parse(&source, &path.display().to_string())?;
    let formatted = format_source(&source);
    std::fs::write(path, formatted).map_err(file_error)?;
    println!("Formatted {}.", path.display());
    Ok(())
}

fn format_source(source: &str) -> String {
    let mut out = String::new();
    let mut indent = 0usize;
    let mut triple = false;
    for raw in source.lines() {
        let trimmed = raw.trim();
        if triple {
            out.push_str(raw);
            out.push('\n');
            if raw.matches("\"\"\"").count() % 2 == 1 {
                triple = false;
            }
            continue;
        }
        if trimmed.starts_with('}') {
            indent = indent.saturating_sub(1)
        }
        if !trimmed.is_empty() {
            out.push_str(&"    ".repeat(indent));
            out.push_str(trimmed.trim_end_matches(';'));
        }
        out.push('\n');
        if raw.matches("\"\"\"").count() % 2 == 1 {
            triple = true;
        }
        let opens = count_code_char(trimmed, '{');
        let closes = count_code_char(trimmed, '}');
        indent = indent
            .saturating_add(opens)
            .saturating_sub(closes.min(opens));
    }
    out
}
fn count_code_char(line: &str, target: char) -> usize {
    let mut quoted = false;
    let mut escape = false;
    let mut n = 0;
    for c in line.chars() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' {
            escape = true;
            continue;
        }
        if c == '"' {
            quoted = !quoted
        } else if !quoted && c == target {
            n += 1
        }
    }
    n
}

fn new_project(path: &Path) -> Result<(), shine_lang::diagnostics::Diagnostic> {
    if path.exists() {
        return Err(shine_lang::diagnostics::Diagnostic::plain(
            "Project Error",
            format!("{} already exists", path.display()),
            "Choose a new project directory.",
        ));
    }
    let name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or("shine-project");
    std::fs::create_dir_all(path.join("src")).map_err(file_error)?;
    std::fs::create_dir_all(path.join("tests")).map_err(file_error)?;
    std::fs::write(
        path.join("shine.toml"),
        format!("[project]\nname = \"{name}\"\nversion = \"0.1.0\"\nentry = \"src/main.shn\"\n"),
    )
    .map_err(file_error)?;
    std::fs::write(
        path.join("src/main.shn"),
        "fn main() {\n    print(\"Hello, Shine\")\n}\n",
    )
    .map_err(file_error)?;
    println!("Created Shine project `{name}` at {}.", path.display());
    Ok(())
}

fn test_project(root: &Path) -> Result<(), shine_lang::diagnostics::Diagnostic> {
    let dir = root.join("tests");
    if !dir.exists() {
        return Err(shine_lang::diagnostics::Diagnostic::plain(
            "Test Error",
            format!("{} does not exist", dir.display()),
            "Create a tests directory with .shn files.",
        ));
    }
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .map_err(file_error)?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension() == Some(OsStr::new("shn")))
        .collect();
    files.sort();
    if files.is_empty() {
        println!("No .shn tests found in {}.", dir.display());
        return Ok(());
    }
    let mut passed = 0;
    for file in &files {
        run_program_path(file)?;
        passed += 1;
        println!("pass {}", file.display())
    }
    println!("\n{passed} test(s) passed.");
    Ok(())
}
fn file_error(e: std::io::Error) -> shine_lang::diagnostics::Diagnostic {
    shine_lang::diagnostics::Diagnostic::plain(
        "File Error",
        e.to_string(),
        "Check the path and file permissions.",
    )
}

fn print_help() {
    println!("Shine {VERSION}\n\nA simple language for mathematics, science, data, and console apps.\n\nUsage:\n  shine new <project>    Create a project\n  shine run <file.shn>   Run a program\n  shine check <file.shn> Check syntax, names, and fixed types\n  shine build <file.shn> Build an executable bundle\n  shine fmt <file.shn>   Format a source file\n  shine test [project]   Run tests/*.shn\n  shine help             Show this help\n  shine version          Show the version");
}
