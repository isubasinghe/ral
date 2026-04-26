use clap::Parser;
use ral::cli::{Cli, Output};
use ral::codegen;
use std::collections::HashMap;
use std::fs;
use std::process;

fn main() {
    let cli = Cli::parse();
    
    // Parse defines
    let mut defines = HashMap::new();
    for define in cli.defines {
        if let Some((key, value)) = define.split_once('=') {
            if let Ok(num) = value.parse::<u64>() {
                defines.insert(key.to_string(), num);
            } else {
                eprintln!("Error: Invalid number in define '{}'", define);
                process::exit(1);
            }
        } else {
            eprintln!("Error: Invalid define format '{}', expected KEY=VALUE", define);
            process::exit(1);
        }
    }
    
    // Read the input file
    let content = match fs::read_to_string(&cli.file) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file '{}': {}", cli.file.display(), err);
            process::exit(1);
        }
    };
    
    // Parse the content
    match ral::parser::parse(&content) {
        Ok(ast) => {
            match cli.output {
                Output::C => {
                    let code = codegen::c::convert_to_c(ast, &defines);
                    println!("{}", code);
                }
                Output::Rust => {
                    // This will currently panic as it is not implemented
                    let code = codegen::rust::convert_to_rust(ast, &defines);
                    println!("{}", code);
                }
            }
        }
        Err(parse_err) => {
            let filename = cli.file.to_string_lossy();
            for report in parse_err.reports(&filename) {
                report
                    .eprint((filename.as_ref(), ariadne::Source::from(&content)))
                    .unwrap_or_else(|err| {
                        eprintln!("failed to render error: {}", err);
                    });
            }
            process::exit(1);
        }
    }
}
