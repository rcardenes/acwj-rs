# ACWJ-RS Agent Guidance

## Essential Commands

- Build: `cargo build`
- Run tests: `cargo test`
- Run a single test: `cargo test test_name`
- Run the compiler: `cargo run -- <input_file> [-o <output_file>]`
  - Default output file: `out.s`
  - Example: `cargo run -- test.c -o test.s`

## Project Structure

- Entry point: `src/main.rs`
- Library code: `src/lib.rs` and module files (`scan.rs`, `expr.rs`, `stmt.rs`, etc.)
- The compiler processes C-like input files and generates assembly output

## Testing Approach

- Tests are colocated with implementation files (e.g., `scan.rs` contains scanning tests)
- Use `#[test]` functions for unit tests
- No special test setup required beyond `cargo test`

## Development Notes

- Uses Rust 2024 edition
- Dependencies: `anyhow` for error handling, `clap` for CLI argument parsing
- Dev dependency: `rstest` for parameterized tests
- Generated assembly output follows the format expected by the original ACWJ project