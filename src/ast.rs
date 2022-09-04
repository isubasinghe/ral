use std::collections::HashMap;
use std::sync::Arc;
use std::cell::RefCell;

pub type Span = std::ops::Range<usize>;


#[derive(Debug, Copy, Clone)]
pub enum BinaryOp {
    Add,
    Subtract
}


pub type Expr = Arc<ExprX>;

pub enum ExprX {
    Num(u32),
    Var(Arc<String>),
    Binary(BinaryOp, Spanned<Expr>, Spanned<Expr>)
}

pub struct Config {
    entries: HashMap<Spanned<Arc<String>>, Spanned<RefCell<Arc<String>>>>
}

pub struct Register {
    name: Spanned<Arc<String>>, 
    mappings: Vec<(Spanned<Arc<String>>, Spanned<u32>)>
}

pub enum RalEntry {
    RawRegister(Spanned<Register>), 
    Alias(Spanned<Arc<String>>), 
    RestrictedAlias(Spanned<Arc<String>>, Spanned<Arc<String>>)
}

pub struct Ral {
    config: Spanned<Config>,
    registers: HashMap<Arc<String>, RalEntry>
}

pub struct Spanned<T> {
    pub source: Arc<String>, 
    pub x: T, 
    pub span: Span
}
