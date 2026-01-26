use crate::ast::*;
use ariadne::{Color, Label, Report, ReportKind};
use chumsky::prelude::*;
use chumsky::Stream;
use core::fmt;
use std::collections::HashMap;
use std::io;
use std::sync::Arc;

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Option<Span>,
    pub source: Option<Arc<String>>,
}

impl ParseError {
    pub fn new(message: String) -> Self {
        ParseError {
            message,
            span: None,
            source: None,
        }
    }
    
    pub fn with_span(message: String, span: Span, source: Arc<String>) -> Self {
        ParseError {
            message,
            span: Some(span),
            source: Some(source),
        }
    }
    
    pub fn report<'a>(&self, filename: &'a str) -> Option<Report<(&'a str, std::ops::Range<usize>)>> {
        if let (Some(span), Some(_source)) = (&self.span, &self.source) {
            Some(
                Report::build(ReportKind::Error, filename, span.start)
                    .with_message(&self.message)
                    .with_label(
                        Label::new((filename, span.clone()))
                            .with_message(&self.message)
                            .with_color(Color::Red),
                    )
                    .finish()
            )
        } else {
            None
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Parse error: {}", self.message)
    }
}

impl From<io::Error> for ParseError {
    fn from(err: io::Error) -> Self {
        ParseError::new(format!("IO error: {}", err))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Token {
    Register,
    Alias,
    Restricted,
    Empty,
    Colon,
    Comma,
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
            Token::Comma => write!(f, ","),
            Token::Register => write!(f, "register"),
            Token::Restricted => write!(f, "restricted"),
            Token::Ident(s) => write!(f, "{}", s),
            Token::RBrace => write!(f, "}}"),
            Token::LBrace => write!(f, "{{"),
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
    let comma = just(',').map(|_| Token::Comma);
    let register = just("register").map(|_| Token::Register);
    let restricted = just("restricted").map(|_| Token::Restricted);
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
        .or(comma)
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

fn parse_expr(
    orig_source: Arc<String>,
) -> impl Parser<Token, Spanned<Expr>, Error = Simple<Token>> + Clone {
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

    // Parse binary expressions using chain operations
    let atom = ident.or(num);
    
    let source = orig_source.clone();
    atom.clone().then(
        just(Token::Add).or(just(Token::Subtract))
            .then(atom)
            .repeated()
    ).foldl(move |left, (op, right)| {
        let span = left.span.start..right.span.end;
        let binary_op = match op {
            Token::Add => BinaryOp::Add,
            Token::Subtract => BinaryOp::Subtract,
            _ => unreachable!(),
        };
        Spanned {
            source: source.clone(),
            x: Arc::new(ExprX::Binary(binary_op, left, right)),
            span,
        }
    })
}

fn parse_config(
    orig_source: Arc<String>,
) -> impl Parser<Token, Spanned<Config>, Error = Simple<Token>> + Clone {
    let source = orig_source.clone();
    
    just(Token::Config)
        .ignore_then(just(Token::LBrace))
        .ignore_then(
            select! { Token::Ident(s) => s.clone() }
                .map_with_span(move |name, span| Spanned {
                    source: source.clone(),
                    x: name,
                    span,
                })
                .separated_by(just(Token::Comma))
                .allow_trailing()
        )
        .then_ignore(just(Token::RBrace))
        .map_with_span(move |variables, span| {
            let source = orig_source.clone();
            Spanned {
                source,
                x: Config { variables },
                span,
            }
        })
}

fn parse_field(
    orig_source: Arc<String>,
) -> impl Parser<Token, Field, Error = Simple<Token>> + Clone {
    let expr_parser = parse_expr(orig_source.clone());
    let source = orig_source.clone();
    
    // Parse either named field (ident: expr) or unnamed field (_: expr)
    let named_field = select! { Token::Ident(s) => s.clone() }
        .map_with_span(move |name, span| Some(Spanned {
            source: source.clone(),
            x: name,
            span,
        }));
    
    let unnamed_field = just(Token::Empty).map(|_| None);
    
    named_field
        .or(unnamed_field)
        .then_ignore(just(Token::Colon))
        .then(expr_parser)
        .then_ignore(just(Token::Comma).or_not())
        .map(|(name, size)| Field { name, size })
}

fn parse_register(
    orig_source: Arc<String>,
) -> impl Parser<Token, Spanned<Register>, Error = Simple<Token>> + Clone {
    let expr_parser = parse_expr(orig_source.clone());
    let field_parser = parse_field(orig_source.clone());
    let source = orig_source.clone();
    
    just(Token::Register)
        .ignore_then(select! { Token::Ident(s) => s.clone() })
        .map_with_span(move |name, span| Spanned {
            source: source.clone(),
            x: name,
            span,
        })
        .then_ignore(just(Token::Colon))
        .then(expr_parser)
        .then_ignore(just(Token::LBrace))
        .then(field_parser.repeated())
        .then_ignore(just(Token::RBrace).labelled("closing brace '}'"))
        .map_with_span(move |((name, size), fields), span| {
            let source = orig_source.clone();
            Spanned {
                source,
                x: Register { name, size, fields },
                span,
            }
        })
}

fn parse_ral(
    orig_source: Arc<String>,
) -> impl Parser<Token, Ral, Error = Simple<Token>> + Clone {
    let config_parser = parse_config(orig_source.clone());
    let register_parser = parse_register(orig_source.clone());
    
    config_parser
        .then(register_parser.repeated())
        .then_ignore(end())
        .map(|(config, registers)| {
            let mut register_map = HashMap::new();
            for register in registers {
                let name = register.x.name.x.clone();
                register_map.insert(name, RalEntry::RawRegister(register));
            }
            Ral {
                config,
                registers: register_map,
            }
        })
}

pub fn parse(input: &str) -> Result<Ral, ParseError> {
    let source = Arc::new(input.to_string());
    
    // Tokenize
    let tokens = lexer().parse(input).map_err(|errs| {
        if let Some(err) = errs.first() {
            ParseError::with_span(
                format!("Lexer error: {}", err),
                err.span(),
                source.clone(),
            )
        } else {
            ParseError::new("Unknown lexer error".to_string())
        }
    })?;
    
    // Parse
    let len = input.chars().count();
    let stream = Stream::from_iter(len..len + 1, tokens.into_iter());
    
    let ral_parser = parse_ral(source.clone());
    ral_parser.parse(stream).map_err(|errs| {
        if let Some(err) = errs.first() {
            ParseError::with_span(
                format!("Parser error: {}", err),
                err.span(),
                source.clone(),
            )
        } else {
            ParseError::new("Unknown parser error".to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_ral() {
        let input = std::fs::read_to_string("testdata/simple.ral").expect("Failed to read simple.ral");
        
        match parse(&input) {
            Ok(ral) => {
                println!("Successfully parsed RAL file!");
                println!("Config variables: {:?}", ral.config.x.variables.len());
                println!("Registers: {:?}", ral.registers.len());
                
                // Check that we have the expected config variable
                assert_eq!(ral.config.x.variables.len(), 1);
                assert_eq!(*ral.config.x.variables[0].x, "xlen".to_string());
                
                // Check that we have the mstatus register
                assert_eq!(ral.registers.len(), 1);
                assert!(ral.registers.contains_key(&Arc::new("mstatus".to_string())));
                
                if let Some(RalEntry::RawRegister(reg)) = ral.registers.get(&Arc::new("mstatus".to_string())) {
                    println!("mstatus register has {} fields", reg.x.fields.len());
                    assert!(reg.x.fields.len() > 0);
                }
            }
            Err(err) => {
                panic!("Parse error: {}", err);
            }
        }
    }

    #[test]
    fn test_parse_fail_ral_missing_brace() {
        let input = std::fs::read_to_string("testdata/fail.ral").expect("Failed to read fail.ral");
        
        match parse(&input) {
            Ok(_) => {
                panic!("Expected parse error for missing closing brace, but parsing succeeded");
            }
            Err(err) => {
                println!("Got expected parse error: {}", err);
                // Check that the error message mentions the missing brace or unexpected end
                let error_msg = err.message.to_lowercase();
                assert!(
                    error_msg.contains("expected") ||
                    error_msg.contains("missing") ||
                    error_msg.contains("unexpected") ||
                    error_msg.contains("end"),
                    "Error message should indicate missing brace or unexpected end, got: {}", err.message
                );
            }
        }
    }
}
