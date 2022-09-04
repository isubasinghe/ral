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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    Var(Arc<String>),
    Subtract,
    Add,
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
            Token::Var(s) => write!(f, "{}", s),
            Token::Subtract => write!(f, "-"),
            Token::Add => write!(f, "+"),
        }
    }
}

fn lexer() -> impl Parser<char, Vec<(Token, Span)>, Error = Simple<char>> {
    let lbrace = just('{').map(|_| Token::LBrace);
    let rbrace = just('}').map(|_| Token::RBrace);
    let add = just('+').map(|_| Token::Add);
    let sub = just('-').map(|_| Token::Subtract);
    let alias = just("alias").map(|_| Token::Alias);
    let empty = just('_').map(|_| Token::Empty);
    let colon = just(':').map(|_| Token::Colon);
    let register = just("register").map(|_| Token::Register);
    let restricted = just("restricted").map(|_| Token::Restriced);
    let config = just("config").map(|_| Token::Config);
    let num = text::int(10).map(|s| Token::Num(Arc::new(s)));
    let var = just('@').ignore_then(text::ident().map(|s| Token::Var(Arc::new(s))));
    let ident = text::ident().map(|s| Token::Ident(Arc::new(s)));

    let token = lbrace
        .or(rbrace)
        .or(alias)
        .or(add)
        .or(sub)
        .or(empty)
        .or(register)
        .or(colon)
        .or(restricted)
        .or(config)
        .or(num)
        .or(var)
        .or(ident);

    token
        .map_with_span(|tok, span| (tok, span))
        .padded()
        .repeated()
}

fn parser_expr(
    orig_source: Arc<String>,
) -> impl Parser<Token, Spanned<Expr>, Error = Simple<Token>> + Clone {
    recursive(move |expr| {
        let ident = select! { Token::Var(s) => s.clone() }.labelled("variable");

        let num = select! { Token::Num(s) => s.clone() }.labelled("number");
        let source = orig_source.clone();

        let num = num.map_with_span(move |v, span| Spanned {
            x: Arc::new(ExprX::Num(v.parse::<u32>().unwrap())),
            source: source.clone(),
            span,
        });

        let source = orig_source.clone();
        let ident = ident.map_with_span(move |v, span| Spanned {
            source: source.clone(),
            x: Arc::new(ExprX::Var(v)),
            span,
        });

        let arith_op = just(Token::Add).or(just(Token::Subtract));
        let arith_op = expr.clone().then(arith_op).then(expr.clone());

        let source = orig_source.clone();
        let arith_op = arith_op.map_with_span(move |((e1, t), e2), span| {
            let op = match t {
                Token::Add => BinaryOp::Add,
                Token::Subtract => BinaryOp::Subtract,
                _ => unreachable!(),
            };
            let x = Arc::new(ExprX::Binary(op, e1, e2));
            Spanned {
                source: source.clone(),
                x,
                span,
            }
        });

        ident.or(arith_op).or(num)
    })
}
