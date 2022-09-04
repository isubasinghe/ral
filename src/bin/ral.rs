use clap::{Parser, ValueEnum};
use std::error::Error;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, ValueEnum, Clone)]
pub enum Output {
    C,
    Rust,
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about=None)]
struct Cli {
    file: PathBuf,
    #[clap(value_enum)]
    output: Output,
}
fn main() {
    let cli = Cli::parse();
    println!("Hello World");
}
