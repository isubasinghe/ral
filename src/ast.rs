use std::collections::HashMap;
use std::sync::Arc;

pub type Span = std::ops::Range<usize>;


#[derive(Debug, Copy, Clone)]
pub enum BinaryOp {
    Add,
    Subtract
}


pub type Expr = Arc<ExprX>;

#[derive(Debug, Clone)]
pub enum ExprX {
    Num(u32),
    Var(Arc<String>),
    Binary(BinaryOp, Spanned<Expr>, Spanned<Expr>)
}

#[derive(Debug, Clone)]
pub struct Config {
    pub variables: Vec<Spanned<Arc<String>>>
}

#[derive(Debug, Clone)]
pub struct Register {
    pub name: Spanned<Arc<String>>,
    pub size: Spanned<Expr>,
    pub fields: Vec<Field>
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: Option<Spanned<Arc<String>>>, // None for unnamed fields (_)
    pub size: Spanned<Expr>
}

#[derive(Debug, Clone)]
pub enum RalEntry {
    RawRegister(Spanned<Register>),
    Alias(Spanned<Arc<String>>),
    RestrictedAlias(Spanned<Arc<String>>, Spanned<Arc<String>>)
}

#[derive(Debug, Clone)]
pub struct Ral {
    pub config: Spanned<Config>,
    pub registers: HashMap<Arc<String>, RalEntry>
}

#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub source: Arc<String>,
    pub x: T,
    pub span: Span
}
