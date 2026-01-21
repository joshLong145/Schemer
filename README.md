# Schemer

A Scheme implementation in Rust with both an interpreter and an ahead-of-time native code compiler. Targets R7RS-small compliance.

## Architecture

Schemer supports two execution modes: interpretation via a tree-walking evaluator, and native compilation via the QBE backend.

### Compilation Pipeline

```
┌──────────────┐     ┌─────────────┐     ┌─────────────────┐     ┌─────────────┐
│ Scheme Source│────▶│   Parser    │────▶│ ANF Transformer │────▶│   Closure   │
│    (.scm)    │     │             │     │                 │     │  Converter  │
└──────────────┘     └─────────────┘     └─────────────────┘     └──────┬──────┘
                                                                        │
                     ┌─────────────┐     ┌─────────────────┐            │
                     │   Native    │◀────│   QBE Backend   │◀───────────┘
                     │ Executable  │     │                 │
                     └─────────────┘     └─────────────────┘
```

**Parser** — Tokenizes source and builds an S-expression AST as `Value` nodes.

**ANF Transformer** — Converts the AST to A-Normal Form, an intermediate representation where all intermediate computations are explicitly named. This simplifies code generation by ensuring arguments are always atomic values.

**Closure Converter** — Performs lambda lifting and closure conversion. Free variables are identified, closures are converted to flat function definitions that receive their environment explicitly, and nested lambdas are hoisted to top-level functions.

**QBE Backend** — Generates QBE IR from the closure-converted ANF. QBE then produces native assembly for the target architecture.

### Runtime System

Compiled programs link against `schemer_runtime`, a static library providing:

- **Tagged pointers** — 3-bit tags distinguish fixnums, pairs, closures, strings, symbols, and other heap objects
- **Memory management** — Reference counting with planned cycle detection
- **Primitives** — Type predicates, pair operations, I/O
- **Tail call optimization** — Trampoline-based TCO for proper tail recursion

## Requirements

**Interpreter only:**
- Rust 1.70+

**Native compilation:**
- [QBE](https://c9x.me/compile/) — macOS: `brew install qbe` | Linux: build from source or use your package manager
- Clang (linker)
- ARM64 macOS or Linux

**Task runner:**
- [cargo-make](https://github.com/sagiegurari/cargo-make#installation)

## Build

```sh
# Build everything
cargo make build

# Build runtime (required for native compilation)
cd packages/runtime && cargo build --release
```

## Usage

```sh
# REPL
cargo run -p schemer

# Run a file
cargo run -p schemer -- file.scm

# Compile to native executable
cargo run -p schemer -- -c file.scm -o output
```

## License

MIT
