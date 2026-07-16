use std::fmt;

use crate::token::Span;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub category: String,
    pub message: String,
    pub file: Option<String>,
    pub span: Option<Span>,
    pub source: Option<String>,
    pub explanation: String,
    pub suggestion: String,
}

impl Diagnostic {
    pub fn at(
        category: impl Into<String>,
        message: impl Into<String>,
        file: impl Into<String>,
        source: impl Into<String>,
        span: Span,
        explanation: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            message: message.into(),
            file: Some(file.into()),
            span: Some(span),
            source: Some(source.into()),
            explanation: explanation.into(),
            suggestion: suggestion.into(),
        }
    }

    pub fn plain(
        category: impl Into<String>,
        message: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self {
            category: category.into(),
            message: message.into(),
            file: None,
            span: None,
            source: None,
            explanation: String::new(),
            suggestion: suggestion.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}: {}", self.category, self.message)?;
        if let (Some(file), Some(span), Some(source)) = (&self.file, self.span, &self.source) {
            let line = source
                .lines()
                .nth(span.line.saturating_sub(1))
                .unwrap_or("");
            writeln!(f, "\n{file}:{}:{}", span.line, span.column)?;
            writeln!(f, "{} | {line}", span.line)?;
            let gutter = span.line.to_string().len();
            let width = span
                .length
                .max(1)
                .min(line.len().saturating_sub(span.column - 1).max(1));
            writeln!(
                f,
                "{} | {}{}",
                " ".repeat(gutter),
                " ".repeat(span.column - 1),
                "^".repeat(width)
            )?;
        }
        if !self.explanation.is_empty() {
            writeln!(f, "\n{}", self.explanation)?;
        }
        if !self.suggestion.is_empty() {
            writeln!(f, "Suggestion: {}", self.suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for Diagnostic {}
