use std::collections::HashMap;

use crate::{ast::*, diagnostics::Diagnostic, lexer::Lexer, parser::Parser, token::Span};

#[derive(PartialEq)]
enum StaticDictionaryKey {
    None,
    Bool(bool),
    Int(i64),
    Float(u64),
    String(String),
    Nan,
}

#[derive(Clone)]
struct Symbol {
    ty: Option<TypeRef>,
    constant: bool,
    function: Option<(Vec<Option<TypeRef>>, Option<TypeRef>)>,
}

pub struct Checker<'a> {
    source: &'a str,
    file: &'a str,
    scopes: Vec<HashMap<String, Symbol>>,
    return_type: Option<TypeRef>,
    loop_depth: usize,
}

impl<'a> Checker<'a> {
    pub fn new(source: &'a str, file: &'a str) -> Self {
        let mut prelude = HashMap::new();
        for name in ["PI", "TAU", "E", "PHI", "INF", "NAN"] {
            prelude.insert(
                name.into(),
                Symbol {
                    ty: Some(TypeRef::Float),
                    constant: true,
                    function: None,
                },
            );
        }
        Self {
            source,
            file,
            scopes: vec![prelude, HashMap::new()],
            return_type: None,
            loop_depth: 0,
        }
    }

    pub fn check(mut self, program: &Program) -> Result<(), Diagnostic> {
        for statement in &program.statements {
            if let Stmt::Function {
                name,
                params,
                return_type,
                span,
                ..
            } = statement
            {
                self.declare(
                    name,
                    Symbol {
                        ty: None,
                        constant: true,
                        function: Some((
                            params.iter().map(|p| p.ty.clone()).collect(),
                            return_type.clone(),
                        )),
                    },
                    *span,
                )?;
            } else if let Stmt::Class { name, span, .. } = statement {
                self.declare(
                    name,
                    Symbol {
                        ty: None,
                        constant: true,
                        function: None,
                    },
                    *span,
                )?;
            }
        }
        for statement in &program.statements {
            if let Stmt::Function {
                params,
                return_type,
                body,
                ..
            } = statement
            {
                self.push();
                for param in params {
                    self.declare(
                        &param.name,
                        Symbol {
                            ty: param.ty.clone(),
                            constant: false,
                            function: None,
                        },
                        param.span,
                    )?;
                }
                let old_return = self.return_type.clone();
                self.return_type = return_type.clone();
                self.check_block_contents(body)?;
                self.return_type = old_return;
                self.pop();
            } else if let Stmt::Class { members, .. } = statement {
                for member in members {
                    match member {
                        ClassMember::Field { value, .. } => {
                            self.expr(value)?;
                        }
                        ClassMember::Method {
                            params,
                            return_type,
                            body,
                            span,
                            ..
                        } => {
                            self.push();
                            self.declare(
                                "self",
                                Symbol {
                                    ty: None,
                                    constant: true,
                                    function: None,
                                },
                                *span,
                            )?;
                            for param in params {
                                self.declare(
                                    &param.name,
                                    Symbol {
                                        ty: param.ty.clone(),
                                        constant: false,
                                        function: None,
                                    },
                                    param.span,
                                )?;
                            }
                            let old_return = self.return_type.clone();
                            self.return_type = return_type.clone();
                            self.check_block_contents(body)?;
                            self.return_type = old_return;
                            self.pop();
                        }
                    }
                }
            } else {
                self.check_stmt(statement)?;
            }
        }
        Ok(())
    }

    fn check_stmt(&mut self, statement: &Stmt) -> Result<(), Diagnostic> {
        match statement {
            Stmt::Assign {
                target,
                ty,
                value,
                constant,
                op,
                span,
            } => {
                let value_type = self.expr(value)?;
                match target {
                    AssignTarget::Name(name, name_span) => {
                        let declaration = ty.is_some() || *constant;
                        if declaration {
                            if let (
                                Some(TypeRef::Dictionary(Some((expected_key, expected_value)))),
                                Expr::Dictionary(entries, _),
                            ) = (ty, value)
                            {
                                for (key, value) in entries {
                                    if let Some(actual) = self.expr(key)? {
                                        self.require_type(expected_key, &actual, key.span())?;
                                    }
                                    if let Some(actual) = self.expr(value)? {
                                        self.require_type(expected_value, &actual, value.span())?;
                                    }
                                }
                            }
                            if let (Some(expected), Some(actual)) = (ty, &value_type) {
                                self.require_type(expected, actual, *span)?;
                            }
                            self.declare(
                                name,
                                Symbol {
                                    ty: ty.clone().or(value_type),
                                    constant: *constant,
                                    function: None,
                                },
                                *name_span,
                            )?;
                        } else if let Some(symbol) = self.lookup(name).cloned() {
                            if symbol.constant {
                                return Err(self.const_error(name, *name_span));
                            }
                            if let (Some(expected), Some(actual)) = (&symbol.ty, &value_type) {
                                self.require_type(expected, actual, *span)?;
                            }
                            if !matches!(op, AssignOp::Set) {
                                match op {
                                    AssignOp::Add => self.require_addable(
                                        symbol.ty.as_ref(),
                                        value_type.as_ref(),
                                        *span,
                                    )?,
                                    AssignOp::Multiply => self.require_multipliable(
                                        symbol.ty.as_ref(),
                                        value_type.as_ref(),
                                        *span,
                                    )?,
                                    AssignOp::Subtract | AssignOp::Divide => self
                                        .check_numeric_pair(
                                            symbol.ty.as_ref(),
                                            value_type.as_ref(),
                                            *span,
                                        )?,
                                    AssignOp::Set => {}
                                }
                            }
                        } else if matches!(op, AssignOp::Set) {
                            self.declare(
                                name,
                                Symbol {
                                    ty: None,
                                    constant: false,
                                    function: None,
                                },
                                *name_span,
                            )?;
                        } else {
                            return Err(self.unknown(name, *name_span));
                        }
                    }
                    AssignTarget::Destructure(names, target_span) => {
                        if let Some(actual) = &value_type {
                            if !matches!(actual, TypeRef::List(_)) {
                                return Err(self.type_error("List", actual, *target_span));
                            }
                        }
                        for name in names {
                            if self.lookup(name).is_none() {
                                self.declare(
                                    name,
                                    Symbol {
                                        ty: None,
                                        constant: *constant,
                                        function: None,
                                    },
                                    *target_span,
                                )?;
                            } else if self.lookup(name).is_some_and(|s| s.constant) {
                                return Err(self.const_error(name, *target_span));
                            }
                        }
                    }
                    AssignTarget::Index(object, index, target_span) => {
                        let object_type = self.expr(object)?;
                        let index_type = self.expr(index)?;
                        if let Expr::Name(name, _) = object.as_ref() {
                            if self.lookup(name).is_some_and(|s| s.constant) {
                                return Err(self.const_error(name, *target_span));
                            }
                        }
                        if let Some(actual) = object_type {
                            match actual {
                                TypeRef::List(item) => {
                                    if let Some(actual) = index_type {
                                        self.require_type(&TypeRef::Int, &actual, index.span())?;
                                    }
                                    if let (Some(expected), Some(value_type)) = (item, &value_type)
                                    {
                                        self.require_type(&expected, value_type, *target_span)?;
                                    }
                                }
                                TypeRef::Dictionary(types) => {
                                    if let Some((key, item)) = types {
                                        if let Some(actual) = &index_type {
                                            self.require_type(&key, actual, index.span())?;
                                        }
                                        if let Some(actual) = &value_type {
                                            self.require_type(&item, actual, *target_span)?;
                                        }
                                    } else if let Some(actual) = &index_type {
                                        self.require_dictionary_key(actual, index.span())?;
                                    }
                                }
                                actual => {
                                    return Err(self.type_error(
                                        "List or Dictionary",
                                        &actual,
                                        *target_span,
                                    ));
                                }
                            }
                        }
                    }
                    AssignTarget::Member(object, _, _) => {
                        self.expr(object)?;
                    }
                }
                Ok(())
            }
            Stmt::If {
                condition,
                then_block,
                else_branch,
                ..
            } => {
                self.expr(condition)?;
                self.check_block(then_block)?;
                if let Some(branch) = else_branch {
                    self.check_stmt(branch)?;
                }
                Ok(())
            }
            Stmt::Loop { kind, body, .. } => {
                self.loop_depth += 1;
                self.push();
                match kind {
                    LoopKind::Forever => {}
                    LoopKind::While(condition) => {
                        self.expr(condition)?;
                    }
                    LoopKind::For {
                        name,
                        iterable,
                        step,
                    } => {
                        let iterable_type = self.expr(iterable)?;
                        let item_type = match iterable_type {
                            Some(TypeRef::List(item)) => item.map(|x| *x),
                            Some(TypeRef::Dictionary(Some((key, _)))) => Some(*key),
                            Some(TypeRef::Dictionary(None)) => None,
                            Some(TypeRef::String) => Some(TypeRef::String),
                            _ => Some(TypeRef::Int),
                        };
                        self.declare(
                            name,
                            Symbol {
                                ty: item_type,
                                constant: false,
                                function: None,
                            },
                            statement.span(),
                        )?;
                        if let Some(step) = step {
                            if let Some(actual) = self.expr(step)? {
                                self.require_type(&TypeRef::Int, &actual, step.span())?;
                            }
                        }
                    }
                }
                let result = self.check_block_contents(body);
                self.pop();
                self.loop_depth -= 1;
                result
            }
            Stmt::Return(value, span) => {
                let actual = if let Some(value) = value {
                    self.expr(value)?
                } else {
                    Some(TypeRef::None)
                };
                if let (Some(expected), Some(actual)) = (&self.return_type, actual) {
                    self.require_type(expected, &actual, *span)?;
                }
                Ok(())
            }
            Stmt::Break(span) | Stmt::Continue(span) => {
                if self.loop_depth == 0 {
                    Err(self.error(
                        "Control Error",
                        "loop control used outside a loop",
                        *span,
                        "break and continue only make sense inside loop.",
                        "Move it into a loop.",
                    ))
                } else {
                    Ok(())
                }
            }
            Stmt::Expr(expression) => {
                self.expr(expression)?;
                Ok(())
            }
            Stmt::Function { .. } | Stmt::Class { .. } => Ok(()),
        }
    }

    fn check_block(&mut self, block: &Block) -> Result<(), Diagnostic> {
        self.push();
        let result = self.check_block_contents(block);
        self.pop();
        result
    }
    fn check_block_contents(&mut self, block: &Block) -> Result<(), Diagnostic> {
        for statement in &block.statements {
            self.check_stmt(statement)?;
        }
        Ok(())
    }

    fn expr(&mut self, expression: &Expr) -> Result<Option<TypeRef>, Diagnostic> {
        match expression {
            Expr::None(_) => Ok(Some(TypeRef::None)),
            Expr::Bool(_, _) => Ok(Some(TypeRef::Bool)),
            Expr::Int(_, _) => Ok(Some(TypeRef::Int)),
            Expr::Float(_, _) => Ok(Some(TypeRef::Float)),
            Expr::String(text, span) => {
                self.check_interpolations(text, *span)?;
                Ok(Some(TypeRef::String))
            }
            Expr::Name(name, span) => self
                .lookup(name)
                .map(|s| s.ty.clone())
                .ok_or_else(|| self.unknown(name, *span)),
            Expr::List(items, _) => {
                let types = items
                    .iter()
                    .map(|item| self.expr(item))
                    .collect::<Result<Vec<_>, _>>()?;
                let first = types.first().cloned().flatten();
                let homogeneous = first.as_ref().is_some_and(|first| {
                    types
                        .iter()
                        .all(|ty| ty.as_ref().is_some_and(|ty| compatible(first, ty)))
                });
                Ok(Some(TypeRef::List(if homogeneous {
                    first.map(Box::new)
                } else {
                    None
                })))
            }
            Expr::Dictionary(entries, _) => {
                let mut key_types = Vec::with_capacity(entries.len());
                let mut value_types = Vec::with_capacity(entries.len());
                let mut literal_keys = vec![];
                for (key, value) in entries {
                    let key_span = key.span();
                    let key_type = self.expr(key)?;
                    if let Some(actual) = &key_type {
                        self.require_dictionary_key(actual, key.span())?;
                    }
                    if let Some(key) = static_dictionary_key(key) {
                        if matches!(key, StaticDictionaryKey::Nan) {
                            return Err(self.error(
                                "Key Error",
                                "NAN cannot be used as a Dictionary key",
                                key_span,
                                "NAN is not equal to itself and cannot identify one entry.",
                                "Use a finite Float, Int, Bool, String, or none key.",
                            ));
                        }
                        if literal_keys.contains(&key) {
                            return Err(self.error(
                                "Key Error",
                                "duplicate key in Dictionary literal",
                                key_span,
                                "A Dictionary literal may define each key only once.",
                                "Remove the duplicate entry or use a different key.",
                            ));
                        }
                        literal_keys.push(key);
                    }
                    key_types.push(key_type);
                    value_types.push(self.expr(value)?);
                }
                let key = homogeneous_type(&key_types);
                let value = homogeneous_type(&value_types);
                Ok(Some(TypeRef::Dictionary(match (key, value) {
                    (Some(key), Some(value)) => Some((Box::new(key), Box::new(value))),
                    _ => None,
                })))
            }
            Expr::Unary { op, value, span } => {
                let ty = self.expr(value)?;
                if matches!(op, UnaryOp::Not) {
                    Ok(Some(TypeRef::Bool))
                } else {
                    if let Some(actual) = &ty {
                        self.require_numeric(actual, *span)?;
                    }
                    Ok(ty)
                }
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let left_ty = self.expr(left)?;
                let right_ty = self.expr(right)?;
                use BinaryOp::*;
                match op {
                    In => {
                        if let Some(TypeRef::Dictionary(types)) = &right_ty {
                            if let Some((expected, _)) = types {
                                if let Some(actual) = &left_ty {
                                    self.require_type(expected, actual, left.span())?;
                                }
                            } else if let Some(actual) = &left_ty {
                                self.require_dictionary_key(actual, left.span())?;
                            }
                        }
                        Ok(Some(TypeRef::Bool))
                    }
                    Equal | NotEqual | Less | LessEqual | Greater | GreaterEqual | And | Or => {
                        Ok(Some(TypeRef::Bool))
                    }
                    Divide | Power => {
                        self.check_numeric_pair(left_ty.as_ref(), right_ty.as_ref(), *span)?;
                        Ok(Some(TypeRef::Float))
                    }
                    IntegerDivide | Remainder | Subtract => {
                        self.check_numeric_pair(left_ty.as_ref(), right_ty.as_ref(), *span)?;
                        Ok(merge_numeric(left_ty, right_ty))
                    }
                    Add => {
                        self.require_addable(left_ty.as_ref(), right_ty.as_ref(), *span)?;
                        Ok(merge_binary(left_ty, right_ty))
                    }
                    Multiply => {
                        self.require_multipliable(left_ty.as_ref(), right_ty.as_ref(), *span)?;
                        Ok(merge_multiply(left_ty, right_ty))
                    }
                }
            }
            Expr::Range { start, end, span } => {
                for value in [start, end] {
                    if let Some(actual) = self.expr(value)? {
                        self.require_type(&TypeRef::Int, &actual, *span)?;
                    }
                }
                Ok(None)
            }
            Expr::Index {
                object,
                index,
                span,
            } => {
                let object_ty = self.expr(object)?;
                let index_ty = self.expr(index)?;
                match object_ty {
                    Some(TypeRef::List(item)) => {
                        if let Some(actual) = index_ty {
                            self.require_type(&TypeRef::Int, &actual, index.span())?;
                        }
                        Ok(item.map(|x| *x))
                    }
                    Some(TypeRef::String) => {
                        if let Some(actual) = index_ty {
                            self.require_type(&TypeRef::Int, &actual, index.span())?;
                        }
                        Ok(Some(TypeRef::String))
                    }
                    Some(TypeRef::Dictionary(types)) => {
                        if let Some((key, value)) = types {
                            if let Some(actual) = index_ty {
                                self.require_type(&key, &actual, index.span())?;
                            }
                            Ok(Some(*value))
                        } else {
                            if let Some(actual) = index_ty {
                                self.require_dictionary_key(&actual, index.span())?;
                            }
                            Ok(None)
                        }
                    }
                    Some(actual) => {
                        Err(self.type_error("List, String, or Dictionary", &actual, *span))
                    }
                    None => Ok(None),
                }
            }
            Expr::Slice {
                object,
                start,
                end,
                span,
            } => {
                let object_ty = self.expr(object)?;
                for index in [start, end].into_iter().flatten() {
                    if let Some(actual) = self.expr(index)? {
                        self.require_type(&TypeRef::Int, &actual, index.span())?;
                    }
                }
                match object_ty {
                    Some(actual @ TypeRef::List(_)) | Some(actual @ TypeRef::String) => {
                        Ok(Some(actual))
                    }
                    Some(actual) => Err(self.type_error("List or String", &actual, *span)),
                    None => Ok(None),
                }
            }
            Expr::Call { callee, args, span } => {
                let arg_types = args
                    .iter()
                    .map(|arg| self.expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                if let Expr::Name(name, name_span) = callee.as_ref() {
                    if let Some(symbol) = self.lookup(name).cloned() {
                        if let Some((params, returns)) = symbol.function {
                            if params.len() != args.len() {
                                return Err(self.error(
                                    "Argument Error",
                                    format!(
                                        "expected {} arguments, received {}",
                                        params.len(),
                                        args.len()
                                    ),
                                    *span,
                                    "The call does not match the function parameters.",
                                    "Pass the required number of arguments.",
                                ));
                            }
                            for ((expected, actual), arg) in
                                params.iter().zip(arg_types.iter()).zip(args)
                            {
                                if let (Some(expected), Some(actual)) = (expected, actual) {
                                    self.require_type(expected, actual, arg.span())?;
                                }
                            }
                            return Ok(returns);
                        }
                        return Ok(None);
                    }
                    if is_builtin(name) {
                        return Ok(builtin_return(name, &arg_types));
                    }
                    return Err(self.unknown(name, *name_span));
                }
                self.expr(callee)?;
                Ok(None)
            }
            Expr::MemberCall {
                object,
                name,
                args,
                span,
            } => {
                let object_ty = self.expr(object)?;
                let argument_types = args
                    .iter()
                    .map(|arg| self.expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let Some(object_ty) = object_ty else {
                    return Ok(None);
                };
                if let Expr::Name(object_name, _) = object.as_ref() {
                    if is_mutating_method(name)
                        && self.lookup(object_name).is_some_and(|s| s.constant)
                    {
                        return Err(self.const_error(object_name, *span));
                    }
                }
                match &object_ty {
                    TypeRef::List(item) => {
                        if name == "add" {
                            if let Some(item) = item {
                                for (argument, actual) in args.iter().zip(argument_types.iter()) {
                                    if let Some(actual) = actual {
                                        self.require_type(item, actual, argument.span())?;
                                    }
                                }
                            }
                        }
                        let numeric_item = item.as_ref().and_then(|item| {
                            matches!(item.as_ref(), TypeRef::Int | TypeRef::Float)
                                .then(|| item.as_ref().clone())
                        });
                        Ok(match name.as_str() {
                            "have" | "remove" => Some(TypeRef::Bool),
                            "index" => None,
                            "len" => Some(TypeRef::Int),
                            "copy" | "unique" => Some(object_ty),
                            "sum" | "product" | "min" | "max" => {
                                numeric_item.or(Some(TypeRef::Number))
                            }
                            "mean" | "median" | "mode" | "variance" | "std" => Some(TypeRef::Float),
                            "add" | "clear" | "reverse" | "sort" => Some(TypeRef::None),
                            "del" => None,
                            _ => {
                                return Err(self.error(
                                    "Name Error",
                                    format!("List has no method `{name}`"),
                                    *span,
                                    "The requested list method does not exist.",
                                    "Check the list method name.",
                                ))
                            }
                        })
                    }
                    TypeRef::Dictionary(types) => {
                        let (minimum, maximum) = match name.as_str() {
                            "have" | "remove" => (1, 1),
                            "get" => (1, 2),
                            "len" | "clear" | "copy" | "keys" | "values" | "items" => (0, 0),
                            _ => return Err(self.error(
                                "Name Error",
                                format!("Dictionary has no method `{name}`"),
                                *span,
                                "The requested Dictionary method does not exist.",
                                "Use have, get, remove, len, clear, copy, keys, values, or items.",
                            )),
                        };
                        if args.len() < minimum || args.len() > maximum {
                            return Err(self.error(
                                "Argument Error",
                                format!(
                                    "{name} expects {} argument(s), received {}",
                                    if minimum == maximum {
                                        minimum.to_string()
                                    } else {
                                        format!("{minimum} or {maximum}")
                                    },
                                    args.len()
                                ),
                                *span,
                                "The Dictionary method has the wrong number of arguments.",
                                "Adjust the arguments to match the method.",
                            ));
                        }
                        if let Some(actual) = argument_types.first().and_then(Option::as_ref) {
                            if let Some((key, _)) = types {
                                self.require_type(key, actual, args[0].span())?;
                            } else {
                                self.require_dictionary_key(actual, args[0].span())?;
                            }
                        }
                        let (key_type, value_type) = match types {
                            Some((key, value)) => (Some((**key).clone()), Some((**value).clone())),
                            None => (None, None),
                        };
                        Ok(match name.as_str() {
                            "have" | "remove" => Some(TypeRef::Bool),
                            "get" => None,
                            "len" => Some(TypeRef::Int),
                            "clear" => Some(TypeRef::None),
                            "copy" => Some(object_ty),
                            "keys" => Some(TypeRef::List(key_type.map(Box::new))),
                            "values" => Some(TypeRef::List(value_type.map(Box::new))),
                            "items" => Some(TypeRef::List(None)),
                            _ => unreachable!(),
                        })
                    }
                    actual => Err(self.type_error("List or Dictionary", actual, *span)),
                }
            }
            Expr::Member { object, .. } => {
                self.expr(object)?;
                Ok(None)
            }
        }
    }

    fn check_interpolations(&mut self, text: &str, outer_span: Span) -> Result<(), Diagnostic> {
        let chars: Vec<char> = text.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] != '{' {
                i += 1;
                continue;
            }
            if i + 1 < chars.len() && chars[i + 1] == '{' {
                i += 2;
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
                return Err(self.error(
                    "String Error",
                    "unterminated interpolation",
                    outer_span,
                    "An opening `{` has no matching `}`.",
                    "Add the closing brace or use `{{` for a literal brace.",
                ));
            }
            let expression: String = chars[start..i].iter().collect();
            let parsed = Lexer::new(&expression, self.file)
                .scan()
                .and_then(|tokens| Parser::new(tokens, &expression, self.file).expression_only())
                .map_err(|_| {
                    self.error(
                        "String Error",
                        "invalid interpolation",
                        outer_span,
                        format!("`{expression}` is not a valid expression."),
                        "Put a valid Shine expression inside the braces.",
                    )
                })?;
            if let Err(mut error) = self.expr(&parsed) {
                error.span = Some(outer_span);
                return Err(error);
            }
            i += 1;
        }
        Ok(())
    }

    fn check_numeric_pair(
        &self,
        left: Option<&TypeRef>,
        right: Option<&TypeRef>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if let Some(ty) = left {
            self.require_numeric(ty, span)?;
        }
        if let Some(ty) = right {
            self.require_numeric(ty, span)?;
        }
        Ok(())
    }
    fn require_addable(
        &self,
        left: Option<&TypeRef>,
        right: Option<&TypeRef>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if left.is_none() || right.is_none() {
            return Ok(());
        }
        let (left, right) = (left.unwrap(), right.unwrap());
        if (is_numeric(left) && is_numeric(right))
            || compatible(left, right) && matches!(left, TypeRef::String | TypeRef::List(_))
        {
            Ok(())
        } else {
            Err(self.error(
                "Type Error",
                "operator received incompatible values",
                span,
                format!(
                    "{} and {} cannot be used together here.",
                    type_name(left),
                    type_name(right)
                ),
                "Use two numbers or compatible strings/lists.",
            ))
        }
    }

    fn require_multipliable(
        &self,
        left: Option<&TypeRef>,
        right: Option<&TypeRef>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if left.is_none() || right.is_none() {
            return Ok(());
        }
        let (left, right) = (left.unwrap(), right.unwrap());
        let repetition = (matches!(left, TypeRef::String | TypeRef::List(_))
            && matches!(right, TypeRef::Int))
            || (matches!(right, TypeRef::String | TypeRef::List(_))
                && matches!(left, TypeRef::Int));
        if (is_numeric(left) && is_numeric(right)) || repetition {
            Ok(())
        } else {
            Err(self.error(
                "Type Error",
                "operator received incompatible values",
                span,
                format!(
                    "{} and {} cannot be multiplied.",
                    type_name(left),
                    type_name(right)
                ),
                "Use two numbers, or multiply a String/List by an Int.",
            ))
        }
    }
    fn require_numeric(&self, actual: &TypeRef, span: Span) -> Result<(), Diagnostic> {
        if is_numeric(actual) {
            Ok(())
        } else {
            Err(self.type_error("Number", actual, span))
        }
    }
    fn require_dictionary_key(&self, actual: &TypeRef, span: Span) -> Result<(), Diagnostic> {
        if matches!(
            actual,
            TypeRef::String
                | TypeRef::Int
                | TypeRef::Float
                | TypeRef::Number
                | TypeRef::Bool
                | TypeRef::None
        ) {
            Ok(())
        } else {
            Err(self.error(
                "Type Error",
                format!("{} cannot be used as a Dictionary key", type_name(actual)),
                span,
                "Dictionary keys must be scalar values.",
                "Use a String, Int, Float, Bool, or none key.",
            ))
        }
    }
    fn require_type(
        &self,
        expected: &TypeRef,
        actual: &TypeRef,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if compatible(expected, actual) {
            Ok(())
        } else {
            Err(self.type_error(&type_name(expected), actual, span))
        }
    }
    fn declare(&mut self, name: &str, symbol: Symbol, span: Span) -> Result<(), Diagnostic> {
        let scope = self.scopes.last_mut().unwrap();
        if scope.contains_key(name) {
            return Err(self.error(
                "Name Error",
                format!("`{name}` is already defined in this scope"),
                span,
                "Names must be unique inside one scope.",
                "Reassign it or choose another name.",
            ));
        }
        scope.insert(name.into(), symbol);
        Ok(())
    }
    fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.scopes.iter().rev().find_map(|scope| scope.get(name))
    }
    fn push(&mut self) {
        self.scopes.push(HashMap::new());
    }
    fn pop(&mut self) {
        self.scopes.pop();
    }
    fn unknown(&self, name: &str, span: Span) -> Diagnostic {
        self.error(
            "Name Error",
            format!("unknown name `{name}`"),
            span,
            "No variable, function, or built-in with this name is visible here.",
            "Define it before use or check the spelling.",
        )
    }
    fn const_error(&self, name: &str, span: Span) -> Diagnostic {
        self.error(
            "Const Error",
            format!("cannot change constant `{name}`"),
            span,
            "Constants cannot be reassigned or mutated.",
            "Create a new variable instead.",
        )
    }
    fn type_error(&self, expected: &str, actual: &TypeRef, span: Span) -> Diagnostic {
        self.error(
            "Type Error",
            format!("expected {expected}, received {}", type_name(actual)),
            span,
            "A fixed type or operation cannot receive this value.",
            "Use a matching value or remove the type annotation.",
        )
    }
    fn error(
        &self,
        category: impl Into<String>,
        message: impl Into<String>,
        span: Span,
        explanation: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic::at(
            category,
            message,
            self.file,
            self.source,
            span,
            explanation,
            suggestion,
        )
    }
}

fn compatible(expected: &TypeRef, actual: &TypeRef) -> bool {
    match (expected, actual) {
        (TypeRef::Number, TypeRef::Int | TypeRef::Float | TypeRef::Number) => true,
        (TypeRef::List(None), TypeRef::List(_)) => true,
        (TypeRef::List(Some(_)), TypeRef::List(None)) => true,
        (TypeRef::List(Some(a)), TypeRef::List(Some(b))) => compatible(a, b),
        (TypeRef::Dictionary(None), TypeRef::Dictionary(_)) => true,
        (TypeRef::Dictionary(Some(_)), TypeRef::Dictionary(None)) => true,
        (
            TypeRef::Dictionary(Some((expected_key, expected_value))),
            TypeRef::Dictionary(Some((actual_key, actual_value))),
        ) => compatible(expected_key, actual_key) && compatible(expected_value, actual_value),
        (a, b) => a == b,
    }
}
fn is_numeric(ty: &TypeRef) -> bool {
    matches!(ty, TypeRef::Int | TypeRef::Float | TypeRef::Number)
}
fn merge_numeric(a: Option<TypeRef>, b: Option<TypeRef>) -> Option<TypeRef> {
    match (a, b) {
        (Some(TypeRef::Int), Some(TypeRef::Int)) => Some(TypeRef::Int),
        (Some(a), Some(b)) if is_numeric(&a) && is_numeric(&b) => Some(TypeRef::Float),
        _ => None,
    }
}
fn merge_binary(a: Option<TypeRef>, b: Option<TypeRef>) -> Option<TypeRef> {
    if let (Some(a), Some(b)) = (&a, &b) {
        if compatible(a, b) {
            return Some(a.clone());
        }
    }
    merge_numeric(a, b)
}
fn merge_multiply(a: Option<TypeRef>, b: Option<TypeRef>) -> Option<TypeRef> {
    match (&a, &b) {
        (Some(value @ TypeRef::String), Some(TypeRef::Int))
        | (Some(value @ TypeRef::List(_)), Some(TypeRef::Int)) => Some(value.clone()),
        (Some(TypeRef::Int), Some(value @ TypeRef::String))
        | (Some(TypeRef::Int), Some(value @ TypeRef::List(_))) => Some(value.clone()),
        _ => merge_numeric(a, b),
    }
}
fn type_name(ty: &TypeRef) -> String {
    match ty {
        TypeRef::Int => "Int".into(),
        TypeRef::Float => "Float".into(),
        TypeRef::Number => "Number".into(),
        TypeRef::String => "String".into(),
        TypeRef::Bool => "Bool".into(),
        TypeRef::None => "None".into(),
        TypeRef::List(None) => "List".into(),
        TypeRef::List(Some(item)) => format!("List[{}]", type_name(item)),
        TypeRef::Dictionary(None) => "Dictionary".into(),
        TypeRef::Dictionary(Some((key, value))) => {
            format!("Dictionary[{}, {}]", type_name(key), type_name(value))
        }
    }
}

fn homogeneous_type(types: &[Option<TypeRef>]) -> Option<TypeRef> {
    let first = types.first()?.clone()?;
    types
        .iter()
        .all(|ty| ty.as_ref().is_some_and(|ty| compatible(&first, ty)))
        .then_some(first)
}

fn static_dictionary_key(expression: &Expr) -> Option<StaticDictionaryKey> {
    match expression {
        Expr::None(_) => Some(StaticDictionaryKey::None),
        Expr::Bool(value, _) => Some(StaticDictionaryKey::Bool(*value)),
        Expr::Int(value, _) => Some(StaticDictionaryKey::Int(*value)),
        Expr::Float(value, _) => Some(normalize_static_float(*value)),
        Expr::String(value, _) => Some(StaticDictionaryKey::String(value.clone())),
        Expr::Name(name, _) if name == "NAN" => Some(StaticDictionaryKey::Nan),
        Expr::Unary {
            op: UnaryOp::Positive,
            value,
            ..
        } => static_dictionary_key(value),
        Expr::Unary {
            op: UnaryOp::Negate,
            value,
            ..
        } => match value.as_ref() {
            Expr::Int(value, _) => value.checked_neg().map(StaticDictionaryKey::Int),
            Expr::Float(value, _) => Some(normalize_static_float(-value)),
            _ => None,
        },
        _ => None,
    }
}

fn normalize_static_float(value: f64) -> StaticDictionaryKey {
    if value.is_nan() {
        return StaticDictionaryKey::Nan;
    }
    const I64_UPPER_EXCLUSIVE: f64 = 9_223_372_036_854_775_808.0;
    if value >= i64::MIN as f64 && value < I64_UPPER_EXCLUSIVE && value.fract() == 0.0 {
        let integer = value as i64;
        if integer as f64 == value {
            return StaticDictionaryKey::Int(integer);
        }
    }
    StaticDictionaryKey::Float(value.to_bits())
}

fn is_mutating_method(name: &str) -> bool {
    matches!(
        name,
        "add" | "del" | "remove" | "clear" | "reverse" | "sort"
    )
}
fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "input"
            | "length"
            | "type"
            | "number"
            | "string"
            | "bool"
            | "readFile"
            | "writeFile"
            | "abs"
            | "round"
            | "floor"
            | "ceil"
            | "pow"
            | "min"
            | "max"
            | "sum"
            | "sqrt"
            | "sin"
            | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "log"
            | "log10"
            | "log2"
            | "exp"
            | "exp2"
            | "cbrt"
            | "trunc"
            | "fract"
            | "sinh"
            | "cosh"
            | "tanh"
            | "asinh"
            | "acosh"
            | "atanh"
            | "degrees"
            | "radians"
            | "hypot"
            | "atan2"
            | "clamp"
            | "sign"
            | "gcd"
            | "lcm"
            | "factorial"
            | "product"
            | "mean"
            | "median"
            | "mode"
            | "variance"
            | "std"
            | "isNan"
            | "isInfinite"
            | "isFinite"
            | "assert"
    )
}
fn builtin_return(name: &str, arguments: &[Option<TypeRef>]) -> Option<TypeRef> {
    match name {
        "print" | "writeFile" | "assert" => Some(TypeRef::None),
        "input" | "string" | "readFile" | "type" => Some(TypeRef::String),
        "length" => Some(TypeRef::Int),
        "bool" | "isNan" | "isInfinite" | "isFinite" => Some(TypeRef::Bool),
        "sign" | "gcd" | "lcm" | "factorial" => Some(TypeRef::Int),
        "sum" | "product" => arguments
            .first()
            .and_then(|argument| match argument {
                Some(TypeRef::List(Some(item)))
                    if matches!(item.as_ref(), TypeRef::Int | TypeRef::Float) =>
                {
                    Some(item.as_ref().clone())
                }
                _ => None,
            })
            .or(Some(TypeRef::Number)),
        "min" | "max" => {
            let first = arguments.first().cloned().flatten();
            if first.as_ref().is_some_and(|first| {
                matches!(first, TypeRef::Int | TypeRef::Float)
                    && arguments
                        .iter()
                        .all(|argument| argument.as_ref() == Some(first))
            }) {
                first
            } else {
                Some(TypeRef::Number)
            }
        }
        "mean" | "median" | "mode" | "variance" | "std" | "abs" | "floor" | "ceil" | "pow"
        | "sqrt" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "log" | "log10" | "log2"
        | "exp" | "exp2" | "cbrt" | "trunc" | "fract" | "sinh" | "cosh" | "tanh" | "asinh"
        | "acosh" | "atanh" | "degrees" | "radians" | "hypot" | "atan2" | "clamp" => {
            Some(TypeRef::Float)
        }
        "number" | "round" => Some(TypeRef::Number),
        _ => None,
    }
}
