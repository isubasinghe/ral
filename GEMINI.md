# RAL (Register Access Language)

**Context:** Software Engineering / Rust / DSL / Embedded Systems

## Project Overview

RAL is a Domain Specific Language (DSL) designed to simplify the definition and usage of register bitfields in systems programming. It parses `.ral` files and intends to generate safe, idiomatic code for C and Rust, preventing common errors associated with manual bit manipulation in assembly or low-level code.

This project is currently in an **early development/prototyping phase**. The parser is functional, but code generation logic is currently stubbed (TODO).

## Key Features

*   **DSL Parsing:** Uses `chumsky` to parse custom `.ral` configuration files.
*   **Error Reporting:** Uses `ariadne` for pretty-printed, source-referenced error messages.
*   **CLI:** Uses `clap` for argument parsing.

## Architecture

*   **Entry Point:** `src/bin/ral.rs` - The CLI binary. Currently parses the input file and debug-prints the AST.
*   **Parser:** `src/parser.rs` - Contains the grammar and parsing logic.
*   **AST:** `src/ast.rs` - Defines the Abstract Syntax Tree structure.
*   **Code Generation:** `src/codegen/` - Structure exists for C and Rust generation (`c.rs`, `rust.rs`), but implementations are currently `todo!()`.
*   **Test Data:** `testdata/` - Contains example RAL files (e.g., `simple.ral` defining RISC-V `mstatus` register).

## Development

### Prerequisites
*   Rust (Nightly toolchain used in `build.sh`, but standard stable likely works for dev).

### Build & Run
Standard Cargo commands are used.

```bash
# Build
cargo build

# Run with test data
cargo run -- testdata/simple.ral rust
```

*Note: The `rust` argument for output format is required by the CLI definition but currently ignored in the execution logic.*

### Testing
There is a `test_parser.rs` file in the root, and likely unit tests within modules.

```bash
cargo test
```

## Project Structure

*   `src/lib.rs`: Library root, module declarations.
*   `src/cli.rs`: Command-line argument structure (`clap`).
*   `src/codegen/`: Code generation modules (C/Rust).
*   `testdata/`: Example `.ral` files.
*   `build.sh`: Script for specific release builds (x86_64-apple-darwin targeting).
