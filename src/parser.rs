use crate::{
    ast::*,
    diagnostics::Diagnostic,
    token::{Span, Token, TokenKind},
};

pub struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    source: &'a str,
    file: &'a str,
}

impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<Token>, source: &'a str, file: &'a str) -> Self {
        Self {
            tokens,
            current: 0,
            source,
            file,
        }
    }

    pub fn parse(mut self) -> Result<Program, Diagnostic> {
        let mut imports = vec![];
        let mut exports = vec![];
        let mut statements = vec![];
        self.separators();
        while !self.at(&TokenKind::Eof) {
            if self.take(&TokenKind::Import).is_some() {
                imports.push(self.import_module()?);
            } else if self.take(&TokenKind::From).is_some() {
                imports.push(self.import_symbol()?);
            } else {
                let exported = self.take(&TokenKind::Export).is_some();
                let statement = self.statement()?;
                if exported {
                    exports.push(self.exported_name(&statement)?);
                }
                statements.push(statement);
            }
            self.end_statement()?;
        }
        Ok(Program {
            imports,
            exports,
            statements,
        })
    }

    fn import_module(&mut self) -> Result<ImportDecl, Diagnostic> {
        let start = self.previous().span;
        let module = self.module_path()?;
        let alias = if self.take(&TokenKind::As).is_some() {
            Some(self.identifier("expected an alias after `as`")?.0)
        } else {
            None
        };
        Ok(ImportDecl {
            module,
            kind: ImportKind::Module { alias },
            span: start.merge(self.previous().span),
        })
    }

    fn import_symbol(&mut self) -> Result<ImportDecl, Diagnostic> {
        let start = self.previous().span;
        let module = self.module_path()?;
        self.expect(
            &TokenKind::Import,
            "expected `import` after the module path",
        )?;
        let (name, name_span) = self.identifier("expected an exported name to import")?;
        let alias = if self.take(&TokenKind::As).is_some() {
            Some(self.identifier("expected an alias after `as`")?.0)
        } else {
            None
        };
        Ok(ImportDecl {
            module,
            kind: ImportKind::Symbol { name, alias },
            span: start.merge(name_span),
        })
    }

    fn module_path(&mut self) -> Result<Vec<String>, Diagnostic> {
        let mut path = vec![self.identifier("expected a module name")?.0];
        while self.take(&TokenKind::Dot).is_some() {
            path.push(self.identifier("expected a module name after `.`")?.0);
        }
        Ok(path)
    }

    fn exported_name(&self, statement: &Stmt) -> Result<String, Diagnostic> {
        match statement {
            Stmt::Function { name, .. }
            | Stmt::Assign {
                target: AssignTarget::Name(name, _),
                ..
            } => Ok(name.clone()),
            _ => Err(self.error(
                statement.span(),
                "`export` requires a named declaration",
                "Only a top-level function, variable, or constant can be exported.",
                "Put `export` before `fn`, `const`, or a named assignment.",
            )),
        }
    }

    pub fn expression_only(mut self) -> Result<Expr, Diagnostic> {
        self.separators();
        let e = self.expression()?;
        self.separators();
        if !self.at(&TokenKind::Eof) {
            return Err(self.error(
                self.peek().span,
                "unexpected text after expression",
                "The interpolation must contain one complete expression.",
                "Remove the extra text.",
            ));
        }
        Ok(e)
    }

    fn statement(&mut self) -> Result<Stmt, Diagnostic> {
        if self.take(&TokenKind::Fn).is_some() {
            return self.function();
        }
        if self.take(&TokenKind::If).is_some() {
            return self.if_statement();
        }
        if self.take(&TokenKind::Loop).is_some() {
            return self.loop_statement();
        }
        if let Some(t) = self.take(&TokenKind::Return) {
            let value = if self.line_end() {
                None
            } else {
                Some(self.expression()?)
            };
            return Ok(Stmt::Return(value, t.span));
        }
        if let Some(t) = self.take(&TokenKind::Break) {
            return Ok(Stmt::Break(t.span));
        }
        if let Some(t) = self.take(&TokenKind::Continue) {
            return Ok(Stmt::Continue(t.span));
        }
        let constant = self.take(&TokenKind::Const).is_some();
        let start = self.peek().span;
        if constant {
            return self.assignment(true, start);
        }
        if self.looks_like_assignment() {
            return self.assignment(false, start);
        }
        Ok(Stmt::Expr(self.expression()?))
    }

    fn function(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.previous().span;
        let (name, _) = self.identifier("expected a function name")?;
        self.expect(
            &TokenKind::LeftParen,
            "expected `(` after the function name",
        )?;
        let mut params = vec![];
        if !self.at(&TokenKind::RightParen) {
            loop {
                let (param_name, span) = self.identifier("expected a parameter name")?;
                let ty = if self.take(&TokenKind::Colon).is_some() {
                    Some(self.type_ref()?)
                } else {
                    None
                };
                params.push(Param {
                    name: param_name,
                    ty,
                    span,
                });
                if self.take(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        self.expect(&TokenKind::RightParen, "expected `)` after parameters")?;
        let return_type = if self.take(&TokenKind::Colon).is_some() {
            Some(self.type_ref()?)
        } else {
            None
        };
        let body = self.block()?;
        Ok(Stmt::Function {
            name,
            params,
            return_type,
            span: start.merge(body.span),
            body,
        })
    }

    fn if_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.previous().span;
        let condition = self.expression()?;
        let then_block = self.block()?;
        let before_separators = self.current;
        self.separators();
        let else_branch = if self.take(&TokenKind::Else).is_some() {
            if self.take(&TokenKind::If).is_some() {
                Some(Box::new(self.if_statement()?))
            } else {
                let block = self.block()?;
                let span = block.span;
                Some(Box::new(Stmt::If {
                    condition: Expr::Bool(true, span),
                    then_block: block,
                    else_branch: None,
                    span,
                }))
            }
        } else {
            self.current = before_separators;
            None
        };
        Ok(Stmt::If {
            condition,
            span: start.merge(then_block.span),
            then_block,
            else_branch,
        })
    }

    fn loop_statement(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.previous().span;
        if self.at(&TokenKind::LeftBrace) {
            let body = self.block()?;
            return Ok(Stmt::Loop {
                kind: LoopKind::Forever,
                span: start.merge(body.span),
                body,
            });
        }
        let kind = if let TokenKind::Identifier(name) = self.peek().kind.clone() {
            if self.peek_n(1).kind.same_variant(&TokenKind::In) {
                self.advance();
                self.advance();
                let iterable = self.expression()?;
                let step = if self.take(&TokenKind::Step).is_some() {
                    Some(self.expression()?)
                } else {
                    None
                };
                LoopKind::For {
                    name,
                    iterable,
                    step,
                }
            } else {
                LoopKind::While(self.expression()?)
            }
        } else {
            LoopKind::While(self.expression()?)
        };
        let body = self.block()?;
        Ok(Stmt::Loop {
            kind,
            span: start.merge(body.span),
            body,
        })
    }

    fn block(&mut self) -> Result<Block, Diagnostic> {
        self.separators();
        let open = self
            .expect(&TokenKind::LeftBrace, "expected `{` to start a block")?
            .span;
        self.separators();
        let mut statements = vec![];
        while !self.at(&TokenKind::RightBrace) && !self.at(&TokenKind::Eof) {
            statements.push(self.statement()?);
            self.end_statement()?;
        }
        let close = self
            .expect(&TokenKind::RightBrace, "expected `}` to close the block")?
            .span;
        Ok(Block {
            statements,
            span: open.merge(close),
        })
    }

    fn looks_like_assignment(&self) -> bool {
        if matches!(self.peek().kind, TokenKind::Identifier(_)) {
            let mut i = 1;
            if self.peek_n(i).kind.same_variant(&TokenKind::Colon) {
                i += 1;
                while !matches!(
                    self.peek_n(i).kind,
                    TokenKind::Equal | TokenKind::Newline | TokenKind::Eof
                ) {
                    i += 1;
                    if i > 16 {
                        return false;
                    }
                }
            }
            return matches!(
                self.peek_n(i).kind,
                TokenKind::Equal
                    | TokenKind::PlusEqual
                    | TokenKind::MinusEqual
                    | TokenKind::StarEqual
                    | TokenKind::SlashEqual
            ) || self.peek_n(i).kind.same_variant(&TokenKind::LeftBracket);
        }
        if self.peek().kind.same_variant(&TokenKind::LeftBracket) {
            let mut i = 1;
            while i < 64 && !self.peek_n(i).kind.same_variant(&TokenKind::RightBracket) {
                i += 1;
            }
            return self.peek_n(i + 1).kind.same_variant(&TokenKind::Equal);
        }
        false
    }

    fn assignment(&mut self, constant: bool, start: Span) -> Result<Stmt, Diagnostic> {
        let target = if self.take(&TokenKind::LeftBracket).is_some() {
            let mut names = vec![];
            if !self.at(&TokenKind::RightBracket) {
                loop {
                    names.push(self.identifier("expected a name in destructuring")?.0);
                    if self.take(&TokenKind::Comma).is_none() {
                        break;
                    }
                }
            }
            let close = self.expect(&TokenKind::RightBracket, "expected `]`")?.span;
            AssignTarget::Destructure(names, start.merge(close))
        } else {
            let (name, name_span) = self.identifier("expected a variable name")?;
            if self.take(&TokenKind::LeftBracket).is_some() {
                let index = self.expression()?;
                let close = self
                    .expect(&TokenKind::RightBracket, "expected `]` after index")?
                    .span;
                AssignTarget::Index(
                    Box::new(Expr::Name(name, name_span)),
                    Box::new(index),
                    name_span.merge(close),
                )
            } else {
                AssignTarget::Name(name, name_span)
            }
        };
        let ty = if self.take(&TokenKind::Colon).is_some() {
            Some(self.type_ref()?)
        } else {
            None
        };
        let op = if self.take(&TokenKind::Equal).is_some() {
            AssignOp::Set
        } else if self.take(&TokenKind::PlusEqual).is_some() {
            AssignOp::Add
        } else if self.take(&TokenKind::MinusEqual).is_some() {
            AssignOp::Subtract
        } else if self.take(&TokenKind::StarEqual).is_some() {
            AssignOp::Multiply
        } else if self.take(&TokenKind::SlashEqual).is_some() {
            AssignOp::Divide
        } else {
            return Err(self.error(
                self.peek().span,
                "expected an assignment operator",
                "Assignments use `=`, `+=`, `-=`, `*=`, or `/=`.",
                "Add `=` and a value.",
            ));
        };
        if constant && !matches!(op, AssignOp::Set) {
            return Err(self.error(
                start,
                "a constant must use `=`",
                "A constant is initialized exactly once.",
                "Replace the compound operator with `=`.",
            ));
        }
        let value = self.expression()?;
        Ok(Stmt::Assign {
            target,
            ty,
            value,
            constant,
            op,
            span: start.merge(self.previous().span),
        })
    }

    fn type_ref(&mut self) -> Result<TypeRef, Diagnostic> {
        let (name, span) = self.identifier("expected a type name")?;
        match name.as_str() {
            "Int" => Ok(TypeRef::Int),
            "Float" => Ok(TypeRef::Float),
            "Number" => Ok(TypeRef::Number),
            "String" => Ok(TypeRef::String),
            "Bool" => Ok(TypeRef::Bool),
            "None" => Ok(TypeRef::None),
            "List" => {
                let item = if self.take(&TokenKind::LeftBracket).is_some() {
                    let ty = self.type_ref()?;
                    self.expect(
                        &TokenKind::RightBracket,
                        "expected `]` after list item type",
                    )?;
                    Some(Box::new(ty))
                } else {
                    None
                };
                Ok(TypeRef::List(item))
            }
            _ => Err(self.error(
                span,
                format!("unknown type `{name}`"),
                "Shine supports Int, Float, Number, String, Bool, List, and None.",
                "Use a built-in type name.",
            )),
        }
    }

    fn expression(&mut self) -> Result<Expr, Diagnostic> {
        self.range()
    }
    fn range(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.or()?;
        if self.take(&TokenKind::DotDot).is_some() {
            let r = self.or()?;
            let s = e.span().merge(r.span());
            e = Expr::Range {
                start: Box::new(e),
                end: Box::new(r),
                span: s,
            };
        }
        Ok(e)
    }
    fn or(&mut self) -> Result<Expr, Diagnostic> {
        self.binary(|p| p.and(), &[(TokenKind::Or, BinaryOp::Or)])
    }
    fn and(&mut self) -> Result<Expr, Diagnostic> {
        self.binary(|p| p.equality(), &[(TokenKind::And, BinaryOp::And)])
    }
    fn equality(&mut self) -> Result<Expr, Diagnostic> {
        self.binary(
            |p| p.comparison(),
            &[
                (TokenKind::EqualEqual, BinaryOp::Equal),
                (TokenKind::BangEqual, BinaryOp::NotEqual),
            ],
        )
    }
    fn comparison(&mut self) -> Result<Expr, Diagnostic> {
        self.binary(
            |p| p.term(),
            &[
                (TokenKind::Less, BinaryOp::Less),
                (TokenKind::LessEqual, BinaryOp::LessEqual),
                (TokenKind::Greater, BinaryOp::Greater),
                (TokenKind::GreaterEqual, BinaryOp::GreaterEqual),
                (TokenKind::In, BinaryOp::In),
            ],
        )
    }
    fn term(&mut self) -> Result<Expr, Diagnostic> {
        self.binary(
            |p| p.factor(),
            &[
                (TokenKind::Plus, BinaryOp::Add),
                (TokenKind::Minus, BinaryOp::Subtract),
            ],
        )
    }
    fn factor(&mut self) -> Result<Expr, Diagnostic> {
        self.binary(
            |p| p.power(),
            &[
                (TokenKind::Star, BinaryOp::Multiply),
                (TokenKind::Slash, BinaryOp::Divide),
                (TokenKind::SlashSlash, BinaryOp::IntegerDivide),
                (TokenKind::Percent, BinaryOp::Remainder),
            ],
        )
    }

    fn binary<F>(&mut self, next: F, ops: &[(TokenKind, BinaryOp)]) -> Result<Expr, Diagnostic>
    where
        F: Fn(&mut Self) -> Result<Expr, Diagnostic>,
    {
        let mut left = next(self)?;
        loop {
            let op = ops.iter().find(|(t, _)| self.at(t)).map(|(_, o)| *o);
            let Some(op) = op else { break };
            self.advance();
            let right = next(self)?;
            let span = left.span().merge(right.span());
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn power(&mut self) -> Result<Expr, Diagnostic> {
        let left = self.unary()?;
        if self.take(&TokenKind::StarStar).is_some() {
            let right = self.power()?;
            let span = left.span().merge(right.span());
            Ok(Expr::Binary {
                left: Box::new(left),
                op: BinaryOp::Power,
                right: Box::new(right),
                span,
            })
        } else {
            Ok(left)
        }
    }
    fn unary(&mut self) -> Result<Expr, Diagnostic> {
        let op = if self.take(&TokenKind::Minus).is_some() {
            Some(UnaryOp::Negate)
        } else if self.take(&TokenKind::Plus).is_some() {
            Some(UnaryOp::Positive)
        } else if self.take(&TokenKind::Not).is_some() {
            Some(UnaryOp::Not)
        } else {
            None
        };
        if let Some(op) = op {
            let start = self.previous().span;
            let value = self.unary()?;
            let span = start.merge(value.span());
            Ok(Expr::Unary {
                op,
                value: Box::new(value),
                span,
            })
        } else {
            self.postfix()
        }
    }
    fn postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut e = self.primary()?;
        loop {
            if self.take(&TokenKind::LeftParen).is_some() {
                let args = self.arguments()?;
                let close = self
                    .expect(&TokenKind::RightParen, "expected `)` after arguments")?
                    .span;
                let span = e.span().merge(close);
                e = Expr::Call {
                    callee: Box::new(e),
                    args,
                    span,
                };
            } else if self.take(&TokenKind::Dot).is_some() {
                let (name, _) = self.identifier("expected a method name after `.`")?;
                self.expect(&TokenKind::LeftParen, "expected `(` after method name")?;
                let args = self.arguments()?;
                let close = self
                    .expect(&TokenKind::RightParen, "expected `)` after arguments")?
                    .span;
                let span = e.span().merge(close);
                e = Expr::MemberCall {
                    object: Box::new(e),
                    name,
                    args,
                    span,
                };
            } else if self.take(&TokenKind::LeftBracket).is_some() {
                if self.take(&TokenKind::DotDot).is_some() {
                    let end = if self.at(&TokenKind::RightBracket) {
                        None
                    } else {
                        Some(Box::new(self.expression()?))
                    };
                    let close = self.expect(&TokenKind::RightBracket, "expected `]`")?.span;
                    let span = e.span().merge(close);
                    e = Expr::Slice {
                        object: Box::new(e),
                        start: None,
                        end,
                        span,
                    };
                } else {
                    let first = self.or()?;
                    if self.take(&TokenKind::DotDot).is_some() {
                        let end = if self.at(&TokenKind::RightBracket) {
                            None
                        } else {
                            Some(Box::new(self.expression()?))
                        };
                        let close = self.expect(&TokenKind::RightBracket, "expected `]`")?.span;
                        let span = e.span().merge(close);
                        e = Expr::Slice {
                            object: Box::new(e),
                            start: Some(Box::new(first)),
                            end,
                            span,
                        };
                    } else {
                        let close = self.expect(&TokenKind::RightBracket, "expected `]`")?.span;
                        let span = e.span().merge(close);
                        e = Expr::Index {
                            object: Box::new(e),
                            index: Box::new(first),
                            span,
                        };
                    }
                }
            } else {
                break;
            }
        }
        Ok(e)
    }
    fn arguments(&mut self) -> Result<Vec<Expr>, Diagnostic> {
        let mut a = vec![];
        if !self.at(&TokenKind::RightParen) {
            loop {
                a.push(self.expression()?);
                if self.take(&TokenKind::Comma).is_none() {
                    break;
                }
            }
        }
        Ok(a)
    }
    fn primary(&mut self) -> Result<Expr, Diagnostic> {
        let t = self.advance().clone();
        match t.kind {
            TokenKind::None => Ok(Expr::None(t.span)),
            TokenKind::True => Ok(Expr::Bool(true, t.span)),
            TokenKind::False => Ok(Expr::Bool(false, t.span)),
            TokenKind::Int(v) => Ok(Expr::Int(v, t.span)),
            TokenKind::Float(v) => Ok(Expr::Float(v, t.span)),
            TokenKind::String(v) => Ok(Expr::String(v, t.span)),
            TokenKind::Identifier(v) => Ok(Expr::Name(v, t.span)),
            TokenKind::LeftParen => {
                let e = self.expression()?;
                self.expect(&TokenKind::RightParen, "expected `)`")?;
                Ok(e)
            }
            TokenKind::LeftBracket => {
                let mut items = vec![];
                if !self.at(&TokenKind::RightBracket) {
                    loop {
                        items.push(self.expression()?);
                        if self.take(&TokenKind::Comma).is_none() {
                            break;
                        }
                    }
                }
                let close = self.expect(&TokenKind::RightBracket, "expected `]`")?.span;
                Ok(Expr::List(items, t.span.merge(close)))
            }
            _ => Err(self.error(
                t.span,
                "expected an expression",
                "A value, variable, list, or function call was expected here.",
                "Add a valid expression.",
            )),
        }
    }

    fn end_statement(&mut self) -> Result<(), Diagnostic> {
        if self.at(&TokenKind::RightBrace) || self.at(&TokenKind::Eof) {
            return Ok(());
        }
        if self.take(&TokenKind::Semicolon).is_some() || self.take(&TokenKind::Newline).is_some() {
            self.separators();
            Ok(())
        } else {
            Err(self.error(
                self.peek().span,
                "expected a new line or `;`",
                "Statements must be separated.",
                "Put the next statement on a new line.",
            ))
        }
    }
    fn separators(&mut self) {
        while self.take(&TokenKind::Newline).is_some() || self.take(&TokenKind::Semicolon).is_some()
        {
        }
    }
    fn line_end(&self) -> bool {
        matches!(
            self.peek().kind,
            TokenKind::Newline | TokenKind::Semicolon | TokenKind::RightBrace | TokenKind::Eof
        )
    }
    fn identifier(&mut self, message: &str) -> Result<(String, Span), Diagnostic> {
        let t = self.advance().clone();
        if let TokenKind::Identifier(s) = t.kind {
            Ok((s, t.span))
        } else {
            Err(self.error(
                t.span,
                message,
                "A name was required here.",
                "Use a name made from letters, digits, or underscores.",
            ))
        }
    }
    fn expect(&mut self, kind: &TokenKind, message: &str) -> Result<&Token, Diagnostic> {
        if self.at(kind) {
            Ok(self.advance())
        } else {
            Err(self.error(
                self.peek().span,
                message,
                "The syntax is incomplete.",
                "Check the surrounding punctuation.",
            ))
        }
    }
    fn take(&mut self, kind: &TokenKind) -> Option<Token> {
        if self.at(kind) {
            Some(self.advance().clone())
        } else {
            None
        }
    }
    fn at(&self, kind: &TokenKind) -> bool {
        self.peek().kind.same_variant(kind)
    }
    fn peek(&self) -> &Token {
        self.tokens
            .get(self.current)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }
    fn peek_n(&self, n: usize) -> &Token {
        self.tokens
            .get(self.current + n)
            .unwrap_or_else(|| self.tokens.last().unwrap())
    }
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
    fn advance(&mut self) -> &Token {
        let i = self.current;
        if !self.at(&TokenKind::Eof) {
            self.current += 1
        }
        &self.tokens[i]
    }
    fn error(
        &self,
        span: Span,
        message: impl Into<String>,
        explanation: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Diagnostic {
        Diagnostic::at(
            "Syntax Error",
            message,
            self.file,
            self.source,
            span,
            explanation,
            suggestion,
        )
    }
}
