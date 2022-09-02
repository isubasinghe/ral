use crate::ast::*;
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use chumsky::Stream;
use core::fmt;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::*;
use std::io;
use std::sync::Arc;

#[derive(Debug)]
pub struct ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}

impl From<io::Error> for ParseError {
    fn from(_: io::Error) -> Self {
        ParseError {}
    }
}

#[derive(Debug)]
enum Token {
    Register,
    Alias,
    Restriced,
    Empty,
    Colon,
    Num(u32),
    Ident(Arc<String>),
    RBrace,
    LBrace,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Alias => write!(f, "alias"),
            Token::Empty => write!(f, "_"),
            Token::Num(n) => write!(f, "{}", n),
            Token::Colon => write!(f, ":"),
            Token::Register => write!(f, "register"),
            Token::Restriced => write!(f, "restricted"),
            Token::Ident(s) => write!(f, "{}", s),
            Token::RBrace => write!(f, "{{"),
            Token::LBrace => write!(f, "}}"),
        }
    }
}

fn lexer() -> impl Parser<char, Vec<(Token, Span)>, Error = Simple<char>> {
    let lbrace = just('{').map(|_| Token::LBrace);
    let rbrace = just('}').map(|_| Token::RBrace);
    let alias = just("alias").map(|_| Token::Alias);
    let empty = just('_').map(|_| Token::Empty);
    let colon = just(':').map(|_| Token::Colon);
    let register = just("register").map(|_| Token::Register);
    let restricted = just("restricted").map(|_| Token::Restriced);

    let token = lbrace
        .or(rbrace)
        .or(alias)
        .or(empty)
        .or(register)
        .or(colon)
        .or(restricted);

    token
        .map_with_span(|tok, span| (tok, span))
        .padded()
        .repeated()
}
