use std::{
    backtrace::Backtrace,
    path::{Path, PathBuf},
};

use common::Span;
use slynx_lexer::LexerError;
use slynx_parser::ParseError;

#[derive(Debug)]
pub enum SourceErrorKind {
    InexistantSource(std::io::Error, Span, PathBuf),
    Lexing(LexerError),
    Parsing(ParseError),
}

#[derive(Debug)]
pub struct SourceError {
    kind: Box<SourceErrorKind>,
    entry: PathBuf,
    backtrace: Box<Backtrace>,
}

impl SourceError {
    ///Creates a new `inexistant_module` error with the given `e` io error, `entry` entry point that tried to be read, `generator` being the path of the file that generated that error and `span` to emit where it happened
    pub fn inexistant_module(
        e: std::io::Error,
        entry: PathBuf,
        generator: PathBuf,
        span: Span,
    ) -> Self {
        Self {
            kind: Box::new(SourceErrorKind::InexistantSource(e, span, generator)),
            entry: entry.clone(),
            backtrace: Box::new(Backtrace::capture()),
        }
    }
    pub fn lexing(e: LexerError, entry: PathBuf) -> Self {
        Self {
            kind: Box::new(SourceErrorKind::Lexing(e)),
            entry: entry.clone(),
            backtrace: Box::new(Backtrace::capture()),
        }
    }
    pub fn parsing(e: ParseError, entry: PathBuf) -> Self {
        Self {
            kind: Box::new(SourceErrorKind::Parsing(e)),
            entry: entry.clone(),
            backtrace: Box::new(Backtrace::capture()),
        }
    }

    pub fn backtrace(self) -> Backtrace {
        if let SourceErrorKind::Parsing(e) = *self.kind {
            e.backtrace
        } else {
            *self.backtrace
        }
    }

    pub fn kind(&self) -> &SourceErrorKind {
        &self.kind
    }
    pub fn entry(&self) -> &Path {
        &self.entry
    }
}
