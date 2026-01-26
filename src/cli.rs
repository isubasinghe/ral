use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, ValueEnum, Clone)]
pub enum Output {
    C,
    Rust,
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about=None)]
pub struct Cli {
    pub file: PathBuf,
    #[clap(value_enum)]
    pub output: Output,
    
    /// Define config variables (e.g. -D xlen=64)
    #[clap(short = 'D', value_parser)]
    pub defines: Vec<String>,
}
