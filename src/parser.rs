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

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Register,
    Alias,
    Restriced,
    Empty,
    Colon,
    Num(Arc<String>),
    Ident(Arc<String>),
    RBrace,
    LBrace,
    Config,
    Variable(Arc<String>), 
    Subtract, 
    Add
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
            Token::Config => write!(f, "config"), 
            Token::Variable(s) => write!(f, "{}", s), 
            Token::Subtract => write!(f, "-"), 
            Token::Add => write!(f, "+")
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
    let config = just("config").map(|_| Token::Config);
    let num = text::int(10).map(|s| Token::Num(Arc::new(s)));
    let ident = text::ident().map(|s| Token::Ident(Arc::new(s)));

    let token = lbrace
        .or(rbrace)
        .or(alias)
        .or(empty)
        .or(register)
        .or(colon)
        .or(restricted)
        .or(config)
        .or(num)
        .or(ident);

    token
        .map_with_span(|tok, span| (tok, span))
        .padded()
        .repeated()
}

pub type KV = (Option<Arc<String>>, Arc<String>);

fn parser_ident_int(source: Arc<String>) -> impl Parser<Token, Spanned<KV>, Error=Simple<Token>> + Clone {
    let ident = select! { Token::Ident(s) => s.clone() }.labelled("identifier");
    let empty = just(Token::Empty);

    let value = recursive(move |expr| {
       let num = select! { Token::Num(v) => v.clone() }.labelled("number"); 
       let source = source.clone();
       num.map_with_span(move |v, span: Span| Spanned { source: source.clone(), x: num, span });
       
    });
    todo!()
}
