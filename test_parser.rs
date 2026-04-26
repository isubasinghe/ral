use std::fs;
use ral::parser::parse;

fn main() {
    let input = fs::read_to_string("testdata/simple.ral").expect("Failed to read simple.ral");
    
    match parse(&input) {
        Ok(ral) => {
            println!("Successfully parsed RAL file!");
            println!("Config variables: {:?}", ral.config.x.variables.len());
            println!("Registers: {:?}", ral.registers.len());
            
            // Print config variables
            println!("\nConfig variables:");
            for var in &ral.config.x.variables {
                println!("  - {}", var.x);
            }
            
            // Print registers
            println!("\nRegisters:");
            for (name, entry) in &ral.registers {
                match entry {
                    ral::ast::RalEntry::RawRegister(reg) => {
                        println!("  - {}: {} fields", name, reg.x.fields.len());
                    }
                    _ => {}
                }
            }
        }
        Err(err) => {
            println!("Parse error: {}", err);
            if let Some(report) = err.report("testdata/simple.ral") {
                // For now just print the error, in a real application you'd use ariadne to print it nicely
                println!("Error details: {:?}", report);
            }
        }
    }
}