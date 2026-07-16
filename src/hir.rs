use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::{
    ast::{AssignTarget, Block, ClassMember, Expr, ImportKind, LoopKind, Program, Stmt},
    diagnostics::Diagnostic,
    lexer::Lexer,
    modules::ModuleGraph,
    token::TokenKind,
};

#[derive(Debug, Clone)]
pub struct HirProgram {
    pub program: Program,
    pub entry_source: String,
    pub entry_file: String,
    pub module_count: usize,
    pub entry_relative: PathBuf,
    pub bundle_sources: Vec<(PathBuf, String)>,
}

pub fn lower(graph: ModuleGraph) -> Result<HirProgram, Diagnostic> {
    let mut globals = Vec::with_capacity(graph.modules.len());
    let mut exports = Vec::with_capacity(graph.modules.len());
    for (module_id, module) in graph.modules.iter().enumerate() {
        let mut names = HashMap::new();
        for statement in &module.program.statements {
            let name = match statement {
                Stmt::Function { name, .. } | Stmt::Class { name, .. } => Some(name),
                Stmt::Assign {
                    target: AssignTarget::Name(name, _),
                    ..
                } => Some(name),
                _ => None,
            };
            if let Some(name) = name {
                let linked = if module_id == graph.entry {
                    name.clone()
                } else {
                    format!("__shine_m{module_id}_{name}")
                };
                names.insert(name.clone(), linked);
            }
        }
        let public: HashSet<String> = module.program.exports.iter().cloned().collect();
        for name in &public {
            if !names.contains_key(name) {
                return Err(module_error(
                    format!(
                        "module {} exports unknown name `{name}`",
                        module.path.display()
                    ),
                    "Export a function, variable, or constant declared in that module.",
                ));
            }
        }
        globals.push(names);
        exports.push(public);
    }

    let mut linked_statements = vec![];
    for (module_id, module) in graph.modules.iter().enumerate() {
        let mut imported = HashMap::new();
        let mut namespaces = HashMap::new();
        for resolved in &module.imports {
            match &resolved.declaration.kind {
                ImportKind::Module { alias } => {
                    let local = alias
                        .clone()
                        .unwrap_or_else(|| resolved.declaration.module.last().unwrap().clone());
                    ensure_free_name(&local, &globals[module_id], &imported, &namespaces)?;
                    namespaces.insert(local, resolved.module);
                }
                ImportKind::Symbol { name, alias } => {
                    if !exports[resolved.module].contains(name) {
                        return Err(module_error(
                            format!(
                                "`{name}` is private in module `{}`",
                                resolved.declaration.module.join(".")
                            ),
                            "Add `export` to the declaration or import a public name.",
                        ));
                    }
                    let local = alias.clone().unwrap_or_else(|| name.clone());
                    ensure_free_name(&local, &globals[module_id], &imported, &namespaces)?;
                    imported.insert(local, globals[resolved.module][name].clone());
                }
            }
        }

        let mut renamer = Renamer {
            own_globals: &globals[module_id],
            imported: &imported,
            namespaces: &namespaces,
            all_globals: &globals,
            all_exports: &exports,
            scopes: vec![HashSet::new()],
        };
        for statement in module.program.statements.clone() {
            linked_statements.push(renamer.statement(statement, true)?);
        }
    }

    let entry = &graph.modules[graph.entry];
    let entry_relative = entry
        .path
        .strip_prefix(&graph.source_root)
        .unwrap_or(&entry.path)
        .to_path_buf();
    let bundle_sources = graph
        .modules
        .iter()
        .map(|module| {
            let relative = module
                .path
                .strip_prefix(&graph.source_root)
                .unwrap_or(&module.path)
                .to_path_buf();
            (relative, module.source.clone())
        })
        .collect();
    Ok(HirProgram {
        program: Program {
            imports: vec![],
            exports: vec![],
            statements: linked_statements,
        },
        entry_source: entry.source.clone(),
        entry_file: entry.path.display().to_string(),
        module_count: graph.modules.len(),
        entry_relative,
        bundle_sources,
    })
}

fn ensure_free_name(
    name: &str,
    globals: &HashMap<String, String>,
    imported: &HashMap<String, String>,
    namespaces: &HashMap<String, usize>,
) -> Result<(), Diagnostic> {
    if globals.contains_key(name) || imported.contains_key(name) || namespaces.contains_key(name) {
        return Err(module_error(
            format!("imported name `{name}` conflicts with another top-level name"),
            "Use `as` to choose a unique import alias.",
        ));
    }
    Ok(())
}

struct Renamer<'a> {
    own_globals: &'a HashMap<String, String>,
    imported: &'a HashMap<String, String>,
    namespaces: &'a HashMap<String, usize>,
    all_globals: &'a [HashMap<String, String>],
    all_exports: &'a [HashSet<String>],
    scopes: Vec<HashSet<String>>,
}

impl Renamer<'_> {
    fn statement(&mut self, statement: Stmt, top_level: bool) -> Result<Stmt, Diagnostic> {
        Ok(match statement {
            Stmt::Assign {
                target,
                ty,
                value,
                constant,
                op,
                span,
            } => {
                let value = self.expression(value)?;
                let target = self.target(target, top_level)?;
                Stmt::Assign {
                    target,
                    ty,
                    value,
                    constant,
                    op,
                    span,
                }
            }
            Stmt::Function {
                name,
                params,
                return_type,
                body,
                span,
            } => {
                let name = self.own_globals.get(&name).cloned().unwrap_or(name);
                self.scopes
                    .push(params.iter().map(|param| param.name.clone()).collect());
                let body = self.block(body)?;
                self.scopes.pop();
                Stmt::Function {
                    name,
                    params,
                    return_type,
                    body,
                    span,
                }
            }
            Stmt::Class {
                name,
                members,
                span,
            } => {
                let name = self.own_globals.get(&name).cloned().unwrap_or(name);
                let members = members
                    .into_iter()
                    .map(|member| match member {
                        ClassMember::Field {
                            name,
                            value,
                            private,
                            span,
                        } => Ok(ClassMember::Field {
                            name,
                            value: self.expression(value)?,
                            private,
                            span,
                        }),
                        ClassMember::Method {
                            name,
                            params,
                            return_type,
                            body,
                            private,
                            span,
                        } => {
                            let mut locals: HashSet<String> =
                                params.iter().map(|param| param.name.clone()).collect();
                            locals.insert("self".into());
                            self.scopes.push(locals);
                            let body = self.block(body)?;
                            self.scopes.pop();
                            Ok(ClassMember::Method {
                                name,
                                params,
                                return_type,
                                body,
                                private,
                                span,
                            })
                        }
                    })
                    .collect::<Result<Vec<_>, Diagnostic>>()?;
                Stmt::Class {
                    name,
                    members,
                    span,
                }
            }
            Stmt::If {
                condition,
                then_block,
                else_branch,
                span,
            } => Stmt::If {
                condition: self.expression(condition)?,
                then_block: self.block(then_block)?,
                else_branch: else_branch
                    .map(|branch| self.statement(*branch, false).map(Box::new))
                    .transpose()?,
                span,
            },
            Stmt::Loop { kind, body, span } => {
                let kind = match kind {
                    LoopKind::Forever => LoopKind::Forever,
                    LoopKind::While(condition) => LoopKind::While(self.expression(condition)?),
                    LoopKind::For {
                        name,
                        iterable,
                        step,
                    } => {
                        let iterable = self.expression(iterable)?;
                        let step = step.map(|value| self.expression(value)).transpose()?;
                        self.scopes.push(HashSet::from([name.clone()]));
                        let body = self.block_contents(body)?;
                        self.scopes.pop();
                        return Ok(Stmt::Loop {
                            kind: LoopKind::For {
                                name,
                                iterable,
                                step,
                            },
                            body,
                            span,
                        });
                    }
                };
                Stmt::Loop {
                    kind,
                    body: self.block(body)?,
                    span,
                }
            }
            Stmt::Return(value, span) => {
                Stmt::Return(value.map(|value| self.expression(value)).transpose()?, span)
            }
            Stmt::Break(span) => Stmt::Break(span),
            Stmt::Continue(span) => Stmt::Continue(span),
            Stmt::Expr(expression) => Stmt::Expr(self.expression(expression)?),
        })
    }

    fn block(&mut self, block: Block) -> Result<Block, Diagnostic> {
        self.scopes.push(HashSet::new());
        let result = self.block_contents(block);
        self.scopes.pop();
        result
    }

    fn block_contents(&mut self, block: Block) -> Result<Block, Diagnostic> {
        let statements = block
            .statements
            .into_iter()
            .map(|statement| self.statement(statement, false))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Block {
            statements,
            span: block.span,
        })
    }

    fn target(
        &mut self,
        target: AssignTarget,
        top_level: bool,
    ) -> Result<AssignTarget, Diagnostic> {
        Ok(match target {
            AssignTarget::Name(name, span) => {
                let name = self.assignment_name(name, top_level);
                AssignTarget::Name(name, span)
            }
            AssignTarget::Destructure(names, span) => AssignTarget::Destructure(
                names
                    .into_iter()
                    .map(|name| self.assignment_name(name, top_level))
                    .collect(),
                span,
            ),
            AssignTarget::Index(object, index, span) => AssignTarget::Index(
                Box::new(self.expression(*object)?),
                Box::new(self.expression(*index)?),
                span,
            ),
            AssignTarget::Member(object, name, span) => {
                AssignTarget::Member(Box::new(self.expression(*object)?), name, span)
            }
        })
    }

    fn assignment_name(&mut self, name: String, top_level: bool) -> String {
        if top_level {
            return self.own_globals.get(&name).cloned().unwrap_or(name);
        }
        if self.is_local(&name) {
            return name;
        }
        if let Some(linked) = self.lookup_global(&name) {
            return linked.to_string();
        }
        self.scopes.last_mut().unwrap().insert(name.clone());
        name
    }

    fn expression(&mut self, expression: Expr) -> Result<Expr, Diagnostic> {
        Ok(match expression {
            Expr::String(text, span) => Expr::String(self.interpolated_text(&text)?, span),
            Expr::Name(name, span) => {
                if self.is_local(&name) {
                    Expr::Name(name, span)
                } else if let Some(linked) = self.lookup_global(&name) {
                    Expr::Name(linked.to_string(), span)
                } else {
                    Expr::Name(name, span)
                }
            }
            Expr::List(items, span) => Expr::List(
                items
                    .into_iter()
                    .map(|item| self.expression(item))
                    .collect::<Result<_, _>>()?,
                span,
            ),
            Expr::Unary { op, value, span } => Expr::Unary {
                op,
                value: Box::new(self.expression(*value)?),
                span,
            },
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => Expr::Binary {
                left: Box::new(self.expression(*left)?),
                op,
                right: Box::new(self.expression(*right)?),
                span,
            },
            Expr::Call { callee, args, span } => Expr::Call {
                callee: Box::new(self.expression(*callee)?),
                args: args
                    .into_iter()
                    .map(|arg| self.expression(arg))
                    .collect::<Result<_, _>>()?,
                span,
            },
            Expr::MemberCall {
                object,
                name,
                args,
                span,
            } => {
                if let Expr::Name(namespace, namespace_span) = object.as_ref() {
                    if let Some(module) = self.namespaces.get(namespace) {
                        if !self.all_exports[*module].contains(&name) {
                            return Err(module_error(
                                format!("`{name}` is private in imported module `{namespace}`"),
                                "Export that declaration or call a public function.",
                            ));
                        }
                        let linked = self.all_globals[*module].get(&name).ok_or_else(|| {
                            module_error(
                                format!("module `{namespace}` has no declaration `{name}`"),
                                "Check the imported member name.",
                            )
                        })?;
                        return Ok(Expr::Call {
                            callee: Box::new(Expr::Name(linked.clone(), *namespace_span)),
                            args: args
                                .into_iter()
                                .map(|arg| self.expression(arg))
                                .collect::<Result<_, _>>()?,
                            span,
                        });
                    }
                }
                Expr::MemberCall {
                    object: Box::new(self.expression(*object)?),
                    name,
                    args: args
                        .into_iter()
                        .map(|arg| self.expression(arg))
                        .collect::<Result<_, _>>()?,
                    span,
                }
            }
            Expr::Member { object, name, span } => Expr::Member {
                object: Box::new(self.expression(*object)?),
                name,
                span,
            },
            Expr::Index {
                object,
                index,
                span,
            } => Expr::Index {
                object: Box::new(self.expression(*object)?),
                index: Box::new(self.expression(*index)?),
                span,
            },
            Expr::Slice {
                object,
                start,
                end,
                span,
            } => Expr::Slice {
                object: Box::new(self.expression(*object)?),
                start: start
                    .map(|value| self.expression(*value).map(Box::new))
                    .transpose()?,
                end: end
                    .map(|value| self.expression(*value).map(Box::new))
                    .transpose()?,
                span,
            },
            Expr::Range { start, end, span } => Expr::Range {
                start: Box::new(self.expression(*start)?),
                end: Box::new(self.expression(*end)?),
                span,
            },
            literal => literal,
        })
    }

    fn interpolated_text(&self, text: &str) -> Result<String, Diagnostic> {
        let chars: Vec<char> = text.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] == '{' {
                out.push('{');
                out.push('{');
                i += 2;
                continue;
            }
            if chars[i] != '{' {
                out.push(chars[i]);
                i += 1;
                continue;
            }
            let start = i + 1;
            let mut depth = 1;
            i += 1;
            while i < chars.len() && depth > 0 {
                if chars[i] == '{' {
                    depth += 1;
                } else if chars[i] == '}' {
                    depth -= 1;
                }
                if depth > 0 {
                    i += 1;
                }
            }
            if depth != 0 {
                return Err(module_error(
                    "unterminated string interpolation while linking modules",
                    "Close the interpolation with `}`.",
                ));
            }
            let expression: String = chars[start..i].iter().collect();
            out.push('{');
            out.push_str(&self.rewrite_interpolation_expression(&expression)?);
            out.push('}');
            i += 1;
        }
        Ok(out)
    }

    fn rewrite_interpolation_expression(&self, expression: &str) -> Result<String, Diagnostic> {
        let tokens = Lexer::new(expression, "<interpolation>").scan()?;
        let mut replacements = vec![];
        let mut i = 0;
        while i < tokens.len() {
            if let TokenKind::Identifier(name) = &tokens[i].kind {
                if let Some(module) = self.namespaces.get(name) {
                    if i + 2 < tokens.len() && matches!(tokens[i + 1].kind, TokenKind::Dot) {
                        if let TokenKind::Identifier(member) = &tokens[i + 2].kind {
                            if !self.all_exports[*module].contains(member) {
                                return Err(module_error(
                                    format!("`{member}` is private in imported module `{name}`"),
                                    "Export that declaration or use a public name.",
                                ));
                            }
                            let linked =
                                self.all_globals[*module].get(member).ok_or_else(|| {
                                    module_error(
                                        format!("module `{name}` has no declaration `{member}`"),
                                        "Check the imported member name.",
                                    )
                                })?;
                            let start = tokens[i].span.start;
                            let end = tokens[i + 2].span.start + tokens[i + 2].span.length;
                            replacements.push((start, end, linked.clone()));
                            i += 3;
                            continue;
                        }
                    }
                }
                if !self.is_local(name) {
                    if let Some(linked) = self.lookup_global(name) {
                        replacements.push((
                            tokens[i].span.start,
                            tokens[i].span.start + tokens[i].span.length,
                            linked.to_string(),
                        ));
                    }
                }
            }
            i += 1;
        }
        if replacements.is_empty() {
            return Ok(expression.to_string());
        }
        let mut out = String::new();
        let mut cursor = 0;
        for (start, end, replacement) in replacements {
            out.push_str(&expression[cursor..start]);
            out.push_str(&replacement);
            cursor = end;
        }
        out.push_str(&expression[cursor..]);
        Ok(out)
    }

    fn lookup_global(&self, name: &str) -> Option<&str> {
        self.imported
            .get(name)
            .or_else(|| self.own_globals.get(name))
            .map(String::as_str)
    }

    fn is_local(&self, name: &str) -> bool {
        self.scopes.iter().rev().any(|scope| scope.contains(name))
    }
}

fn module_error(message: impl Into<String>, suggestion: impl Into<String>) -> Diagnostic {
    Diagnostic::plain("Module Error", message, suggestion)
}
