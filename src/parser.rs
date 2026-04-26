use crate::ast::*;
use ariadne::{Color, Label, Report, ReportKind};
use chumsky::error::SimpleReason;
use chumsky::prelude::*;
use chumsky::Stream;
use core::fmt;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
    pub primary_label: String,
    pub secondary: Vec<(Span, String)>,
    pub help: Option<String>,
}

#[derive(Debug)]
pub struct ParseError {
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseError {
    pub fn single(message: String) -> Self {
        ParseError {
            diagnostics: vec![Diagnostic {
                message,
                span: 0..0,
                primary_label: String::new(),
                secondary: vec![],
                help: None,
            }],
        }
    }

    /// Top-level message — joins every diagnostic so callers and tests can
    /// pattern-match on the full error stream.
    pub fn message(&self) -> String {
        self.diagnostics
            .iter()
            .map(|d| d.message.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }

    pub fn reports<'a>(
        &self,
        filename: &'a str,
    ) -> Vec<Report<(&'a str, std::ops::Range<usize>)>> {
        self.diagnostics
            .iter()
            .map(|d| {
                let mut report = Report::build(ReportKind::Error, filename, d.span.start)
                    .with_message(&d.message)
                    .with_label(
                        Label::new((filename, d.span.clone()))
                            .with_message(if d.primary_label.is_empty() {
                                d.message.as_str()
                            } else {
                                d.primary_label.as_str()
                            })
                            .with_color(Color::Red),
                    );
                for (sp, msg) in &d.secondary {
                    report = report.with_label(
                        Label::new((filename, sp.clone()))
                            .with_message(msg)
                            .with_color(Color::Yellow),
                    );
                }
                if let Some(help) = &d.help {
                    report = report.with_help(help);
                }
                report.finish()
            })
            .collect()
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message())
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }
    let mut prev: Vec<usize> = (0..=n).collect();
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            curr[j] = (curr[j - 1] + 1).min(prev[j] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }
    prev[n]
}

/// Closest reserved word within edit distance 2, if any.
fn keyword_suggestion(found: &str) -> Option<&'static str> {
    const KEYWORDS: &[&str] = &["config", "register", "alias", "restricted"];
    KEYWORDS
        .iter()
        .copied()
        .map(|kw| (levenshtein(found, kw), kw))
        .filter(|&(d, _)| d > 0 && d <= 2)
        .min_by_key(|&(d, _)| d)
        .map(|(_, kw)| kw)
}

fn diagnostic_from_simple<I, F>(err: &Simple<I, Span>, format_item: F, source_len: usize) -> Diagnostic
where
    I: fmt::Display + std::hash::Hash + Eq + Clone,
    F: Fn(&I) -> String,
{
    // Clamp past-EOF spans so ariadne has a visible character to render.
    let raw = err.span();
    let span = if raw.start >= source_len {
        if source_len == 0 {
            0..0
        } else {
            (source_len - 1)..source_len
        }
    } else {
        raw
    };

    match err.reason() {
        SimpleReason::Unclosed { span: open_span, delimiter } => {
            let d = format_item(delimiter);
            let close = match d.as_str() {
                "{" => "}",
                "(" => ")",
                "[" => "]",
                _ => "",
            };
            Diagnostic {
                message: format!("unclosed `{}` — missing matching `{}`", d, close),
                span,
                primary_label: format!("expected `{}` to close this block", close),
                secondary: vec![(open_span.clone(), format!("`{}` opened here", d))],
                help: Some(format!("add a `{}` to close the block", close)),
            }
        }
        SimpleReason::Custom(msg) => Diagnostic {
            message: msg.clone(),
            span,
            primary_label: String::new(),
            secondary: vec![],
            help: None,
        },
        SimpleReason::Unexpected => {
            let found_str = match err.found() {
                Some(t) => format!("`{}`", format_item(t)),
                None => "end of input".to_string(),
            };
            let expected: Vec<&Option<I>> = err.expected().collect();

            // EOF reached while a closing `}` was on the table — almost always
            // a forgotten brace. chumsky 0.8 `delimited_by` doesn't surface
            // Unclosed for this, so we recognize the pattern here.
            let has_close_brace = expected.iter().any(|e| match e {
                Some(t) => format_item(t) == "}",
                None => false,
            });
            if err.found().is_none() && has_close_brace {
                return Diagnostic {
                    message: "missing closing `}`".to_string(),
                    span,
                    primary_label: "expected `}` here to close the open block".to_string(),
                    secondary: vec![],
                    help: Some(
                        "you may have forgotten to close a `register` or `config` block"
                            .to_string(),
                    ),
                };
            }

            let exp_phrase = if expected.is_empty() {
                String::new()
            } else {
                let parts: Vec<String> = expected
                    .iter()
                    .map(|e| match e {
                        Some(t) => format!("`{}`", format_item(t)),
                        None => "end of input".to_string(),
                    })
                    .collect();
                match parts.as_slice() {
                    [a] => a.clone(),
                    [a, b] => format!("{} or {}", a, b),
                    rest => {
                        let (last, init) = rest.split_last().unwrap();
                        format!("{}, or {}", init.join(", "), last)
                    }
                }
            };

            let label = err.label();
            let (message, primary_label, help) = if err.found().is_none() {
                let msg = "unexpected end of input".to_string();
                let lbl = match (label, exp_phrase.is_empty()) {
                    (Some(l), _) => format!("expected {} here", l),
                    (None, false) => format!("expected {}", exp_phrase),
                    (None, true) => "more input expected".to_string(),
                };
                (msg, lbl, None)
            } else {
                let msg = format!("unexpected {}", found_str);
                let lbl = match (label, exp_phrase.is_empty()) {
                    (Some(l), _) => format!("expected {} here", l),
                    (None, false) => format!("expected {}", exp_phrase),
                    (None, true) => "unexpected token".to_string(),
                };
                (msg, lbl, None)
            };

            Diagnostic {
                message,
                span,
                primary_label,
                secondary: vec![],
                help,
            }
        }
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
    let colon = just(':').map(|_| Token::Colon);
    let comma = just(',').map(|_| Token::Comma);
    let num = text::int(10).map(|s| Token::Num(Arc::new(s)));
    let var = just('@').ignore_then(text::ident().map(|s| Token::Var(Arc::new(s))));

    // Lex a full identifier-shaped word, then disambiguate to keyword/empty/ident.
    // Matching keywords as raw prefixes (e.g. `just("config")`) would split
    // identifiers like `configuration` into `config` + `uration`.
    let ident_or_keyword = text::ident().map(|s: String| match s.as_str() {
        "_" => Token::Empty,
        "alias" => Token::Alias,
        "register" => Token::Register,
        "restricted" => Token::Restricted,
        "config" => Token::Config,
        _ => Token::Ident(Arc::new(s)),
    });

    let token = lbrace
        .or(rbrace)
        .or(add)
        .or(sub)
        .or(colon)
        .or(comma)
        .or(num)
        .or(var)
        .or(ident_or_keyword);

    token
        .map_with_span(|tok, span| (tok, span))
        .padded()
        .repeated()
}

fn parse_expr(
    orig_source: Arc<String>,
) -> impl Parser<Token, Spanned<Expr>, Error = Simple<Token>> + Clone {
    // No `.labelled(...)` on the atom parsers — chumsky 0.8 keeps the deepest
    // label, so labelling here would shadow callers' more useful labels like
    // "register size" or "field size".
    let ident = select! { Token::Var(s) => s.clone() };
    let num = select! { Token::Num(s) => s.clone() };
    
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
        .ignore_then(
            select! { Token::Ident(s) => s.clone() }
                .labelled("config variable name")
                .map_with_span(move |name, span| Spanned {
                    source: source.clone(),
                    x: name,
                    span,
                })
                .separated_by(just(Token::Comma))
                .allow_trailing()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
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
        .then_ignore(just(Token::Colon).labelled("`:` after field name"))
        .then(expr_parser.labelled("field size"))
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
        .then_ignore(just(Token::Colon).labelled("`:` after register name"))
        .then(expr_parser.labelled("register size"))
        .then(
            field_parser
                .repeated()
                .delimited_by(just(Token::LBrace), just(Token::RBrace)),
        )
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
            let mut register_map = BTreeMap::new();
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
    let source_len = input.len();

    // Lex.
    let tokens = lexer().parse(input).map_err(|errs| {
        let diagnostics = errs
            .iter()
            .map(|e| diagnostic_from_simple(e, |c: &char| c.to_string(), source_len))
            .collect();
        ParseError { diagnostics }
    })?;

    // Anchor the EOI span at the end of the last real token so ariadne renders
    // a visible caret instead of pointing past the file.
    let eoi_pos = tokens.last().map(|(_, sp)| sp.end).unwrap_or(source_len);
    let eoi_span = eoi_pos..eoi_pos;
    let stream = Stream::from_iter(eoi_span, tokens.into_iter());

    let ral_parser = parse_ral(Arc::new(input.to_string()));
    ral_parser.parse(stream).map_err(|errs| {
        let diagnostics = errs
            .iter()
            .map(|e| {
                let mut diag =
                    diagnostic_from_simple(e, |t: &Token| format!("{}", t), source_len);
                // If the user wrote a near-miss of a reserved word as an
                // identifier, suggest the keyword they probably meant.
                if let Some(Token::Ident(name)) = e.found() {
                    if let Some(kw) = keyword_suggestion(name) {
                        let hint = format!("did you mean `{}`?", kw);
                        diag.help = Some(match diag.help {
                            Some(prev) => format!("{}; {}", prev, hint),
                            None => hint,
                        });
                    }
                }
                diag
            })
            .collect();
        ParseError { diagnostics }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_ral() {
        let input = std::fs::read_to_string("testdata/simple.ral").expect("Failed to read simple.ral");
        let ral = parse(&input).expect("simple.ral should parse");

        // Single config variable: xlen
        assert_eq!(ral.config.x.variables.len(), 1);
        assert_eq!(*ral.config.x.variables[0].x, "xlen".to_string());

        // Single register: mtvec, with @xlen-parameterized size and arithmetic in `base`.
        assert_eq!(ral.registers.len(), 1);
        let reg = match ral.registers.get(&Arc::new("mtvec".to_string())) {
            Some(RalEntry::RawRegister(r)) => r,
            _ => panic!("expected `mtvec` register"),
        };
        assert_eq!(reg.x.fields.len(), 2);
    }

    #[test]
    fn test_keyword_prefix_identifier() {
        // An identifier that starts with a keyword (e.g. `configuration`) must
        // not be split into `config` + `uration` by the lexer.
        let input = "config { } register configuration: 32 { f: 32 }";
        let ral = parse(input).expect("should parse register named `configuration`");
        assert!(ral.registers.contains_key(&Arc::new("configuration".to_string())));
    }

    #[test]
    fn test_underscore_prefix_field() {
        // A field name starting with `_` (e.g. `_foo`) must be a single Ident,
        // not the unnamed-field marker `_` followed by `foo`.
        let input = "config { } register r: 32 { _foo: 16, bar: 16 }";
        let ral = parse(input).expect("should parse field named `_foo`");
        let reg = match ral.registers.get(&Arc::new("r".to_string())) {
            Some(RalEntry::RawRegister(reg)) => reg,
            _ => panic!("missing register `r`"),
        };
        assert_eq!(reg.x.fields.len(), 2);
        let first_name = reg.x.fields[0].name.as_ref().expect("first field should be named");
        assert_eq!(*first_name.x, "_foo".to_string());
    }

    #[test]
    fn test_unnamed_field_still_works() {
        // Bare `_` as a field name still means an unnamed (reserved) field.
        let input = "config { } register r: 32 { _: 16, bar: 16 }";
        let ral = parse(input).expect("should parse with unnamed field `_`");
        let reg = match ral.registers.get(&Arc::new("r".to_string())) {
            Some(RalEntry::RawRegister(reg)) => reg,
            _ => panic!("missing register `r`"),
        };
        assert!(reg.x.fields[0].name.is_none(), "first field should be unnamed");
    }

    #[test]
    fn test_parse_fail_ral_missing_brace() {
        let input = std::fs::read_to_string("testdata/fail.ral").expect("Failed to read fail.ral");
        let err = parse(&input).expect_err("expected parse failure for missing closing brace");

        let msg = err.message().to_lowercase();
        assert!(
            msg.contains("unclosed") || msg.contains("missing") || msg.contains("expected"),
            "expected message to indicate missing/unclosed brace, got: {}",
            err.message()
        );
        // Primary span must be inside the source so ariadne renders a caret.
        let primary = &err.diagnostics[0];
        assert!(
            primary.span.start < input.len(),
            "primary span ({:?}) should be within source (len {})",
            primary.span,
            input.len()
        );
    }

    fn err_for(path: &str) -> ParseError {
        let input = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {}", path, e));
        match parse(&input) {
            Ok(_) => panic!("expected {} to fail to parse, but it succeeded", path),
            Err(e) => e,
        }
    }

    fn any_help_contains(err: &ParseError, needle: &str) -> bool {
        err.diagnostics
            .iter()
            .any(|d| d.help.as_deref().map_or(false, |h| h.contains(needle)))
    }

    fn any_label_contains(err: &ParseError, needle: &str) -> bool {
        err.diagnostics
            .iter()
            .any(|d| d.primary_label.contains(needle))
    }

    #[test]
    fn err_typo_register_suggests_register() {
        let err = err_for("testdata/errors/typo_register.ral");
        assert!(
            any_help_contains(&err, "`register`"),
            "expected `did you mean register?` hint; diagnostics: {:?}",
            err.diagnostics
        );
    }

    #[test]
    fn err_typo_config_suggests_config() {
        let err = err_for("testdata/errors/typo_config.ral");
        assert!(any_help_contains(&err, "`config`"));
    }

    #[test]
    fn err_typo_restricted_suggests_restricted() {
        let err = err_for("testdata/errors/typo_restricted.ral");
        assert!(any_help_contains(&err, "`restricted`"));
    }

    #[test]
    fn err_missing_register_size_uses_register_size_label() {
        let err = err_for("testdata/errors/missing_register_size.ral");
        assert!(
            any_label_contains(&err, "register size"),
            "expected primary label to mention `register size`; diagnostics: {:?}",
            err.diagnostics
        );
    }

    #[test]
    fn err_missing_field_size_uses_field_size_label() {
        let err = err_for("testdata/errors/missing_field_size.ral");
        assert!(any_label_contains(&err, "field size"));
    }

    #[test]
    fn err_missing_register_colon() {
        let err = err_for("testdata/errors/missing_register_colon.ral");
        assert!(
            any_label_contains(&err, "`:` after register name"),
            "expected primary label to mention `:` after register name; diagnostics: {:?}",
            err.diagnostics
        );
    }

    #[test]
    fn err_missing_field_colon() {
        let err = err_for("testdata/errors/missing_field_colon.ral");
        assert!(
            any_label_contains(&err, "`:`"),
            "expected primary label to mention `:`; diagnostics: {:?}",
            err.diagnostics
        );
    }
}
