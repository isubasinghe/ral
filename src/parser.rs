use crate::ast::*;
use ariadne::{Color, Fmt, Label, Report, ReportKind, Source};
use chumsky::prelude::*;
use chumsky::Stream;
use core::fmt;
use std::fs::*;
use std::io;
use std::sync::Arc;
use std::collections::HashMap;
use std::cell::RefCell;

#[derive(Debug)]
pub struct ParseError {}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}

impl From<io::Error> for ParseError {
    fn from(_: io::Error) -> Self {
        ParseError {  }
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
    Ident(Arc<String>)
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            _ => todo!()
        }
    }
}




