# reorder

`reorder` is a small CLI tool that rewrites Rust source files so their top-level items appear in a consistent order. The ordering groups imports, type aliases, constants, modules, implementations, functions, and test modules into predictable sections, preserving existing shebangs and crate-level attributes.

## Usage

Build and run the binary from the workspace root:

```bash
cargo run -- <paths>
```

The tool accepts any mix of file and directory paths:

- Passing one or more `.rs` files edits them in place.
- Passing a directory scans it recursively for `.rs` files (case-insensitive) and processes each one once, even if reached multiple times.

If no Rust files are found after expanding all inputs, the command exits with an error.

## Scenarios

- **Tidy a single file**: `cargo run -- src/lib.rs`
- **Normalize an entire crate**: `cargo run -- src`
- **Combine inputs**: `cargo run -- src tests/integration.rs examples`

Run `cargo check` (or your project-specific tests) after using the tool to confirm the reordered sources still compile as expected.
