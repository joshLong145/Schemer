# Schemer Library Extensions & Project Manifest Specification

**Status:** Draft
**Date:** 2026-07-04
**Author:** Generated from codebase analysis

---

## 1. Overview

This specification defines two complementary features for Schemer:

1. **Library Extensions** — A mechanism for users to extend the runtime with custom native (C ABI) functions that compiled Scheme code can call, without modifying the core runtime crate.
2. **Project Manifests** — A `schemer.toml` manifest file that defines a Schemer project, its dependencies on library extensions, source files, and build configuration, enabling `schemer build` to compile an entire project.

### 1.1 Design Goals

- **Minimal friction**: Writing a library extension should require only a Rust file exporting `extern "C"` functions and a small TOML descriptor.
- **Self-describing libraries**: Extension metadata is embedded directly into the compiled `.a` static library — no sidecar files needed at Schemer compile time.
- **Safe composition**: Multiple extensions can be linked into a single binary without symbol conflicts.
- **No runtime modification**: Extensions are static libraries linked alongside `libschemer_runtime.a` — the core runtime is never patched.
- **Familiar tooling**: The manifest format (`schemer.toml`) follows conventions from Cargo.toml and is processed at compile time, not interpretation time.

### 1.2 Non-Goals (v1)

- Dynamic/shared library loading (`.dylib`/`.so` at runtime).
- Scheme-level `import`/`export` module system (R7RS library syntax). That is a separate concern.
- Extension versioning or a package registry.

---

## 2. Current Architecture (Context)

Understanding the current pipeline is essential for seeing where extensions fit.

### 2.1 Compilation Pipeline

```
Source (.scm)
  │
  ├─ Parser ──> AST (Value)
  │
  ├─ ANF Transform ──> AnfProgram { functions, entry, strings, symbols }
  │
  ├─ Closure Conversion ──> AnfProgram (with has_env, captures filled in)
  │
  ├─ CodeGenerator ──> QbeModule { functions, data }
  │
  ├─ QBE (external tool) ──> Assembly (.s)
  │
  └─ Linker (clang) ──> Executable
        links: program.s + libschemer_runtime.a + system libs
```

### 2.2 How Primitives Work Today

The compiler knows about primitive operations through two mechanisms:

1. **`PrimOp` enum** (`core/src/compiler/anf.rs:60`) — ~30 operations recognized during ANF transformation. Each maps to either an inline QBE instruction sequence or a C ABI runtime call.

2. **`RUNTIME_FUNCTIONS`** (`core/src/compiler/primitives.rs:115`) — A static list of `RuntimeFn { name, arity, can_gc, can_raise }` that tells the codegen which `extern` symbols to emit `call` instructions for. These symbols are resolved at link time from `libschemer_runtime.a`.

3. **`get_primitive_impl()`** (`core/src/compiler/primitives.rs:49`) — Maps each `PrimOp` to either `PrimImpl::Inline(InlineOp)` or `PrimImpl::RuntimeCall("scm_function_name")`.

### 2.3 Linking Today

The `Linker` struct (`core/src/compiler/link.rs:82`) produces executables by:
1. Writing QBE IR to a `.ssa` file.
2. Invoking `qbe` to produce assembly (`.s`). QBE does not produce object files directly.
3. Invoking `clang` to assemble `program.s` and link against `libschemer_runtime.a` + system libs in a single pass.

The runtime is found via `SCHEMER_RUNTIME_PATH` env var or searched in standard locations.

### 2.4 Object File Format

The primary development target is **macOS arm64**, where object files in `.a` archives are **Mach-O**, not ELF. The secondary target is **Linux aarch64**, which uses ELF. These have different section naming conventions:

| | Mach-O (macOS) | ELF (Linux) |
|---|---|---|
| Custom section name | `__DATA,__schemer_meta` | `.schemer_meta` |
| Rust `#[link_section]` value | `"__DATA,__schemer_meta"` | `".schemer_meta"` |

The `object` crate provides a uniform API for reading both formats.

### 2.5 Prelude

`lib/prelude.scm` is prepended to all compiled sources. It provides pure-Scheme standard library functions (`map`, `filter`, `fold-left`, etc.).

---

## 3. Library Extension System

### 3.1 Extension Structure

A library extension is a directory containing:

```
my-extension/
├── extension.toml       # Extension descriptor — human-authored source of truth (recommended)
├── build.rs             # Reads extension.toml, embeds metadata into the .a (required)
├── Cargo.toml           # Standard Rust crate config (crate-type = ["staticlib"])
├── src/
│   └── lib.rs           # Rust source: extern "C" functions + metadata static
└── lib/                 # Optional: Scheme prelude additions
    └── prelude.scm      # Scheme code loaded before user code
```

`extension.toml` is the **recommended** way to declare metadata. An extension author may instead construct the `ExtensionDescriptor` programmatically in `build.rs`, but `extension.toml` is the conventional approach. At Schemer compile time, only the compiled `.a` file is read — the TOML file is not required to be present.

### 3.2 Extension Metadata: Two-Layer Model

Extension metadata flows through two layers:

```
extension.toml          (human-authored source of truth)
      │
      ▼
build.rs                (reads TOML, serializes to JSON, writes OUT_DIR/meta.bin)
      │
      ▼
lib.rs                  (embeds meta.bin as a #[link_section] static)
      │
      ▼ cargo build --release
lib{name}.a             (Mach-O/ELF .o members, one contains __schemer_meta section)
      │
      ▼ schemer build
Schemer compiler        (reads .a with object crate, deserializes JSON → ExtensionDescriptor)
```

The compiled `.a` file is **self-describing**: it carries its own metadata and can be inspected without any sidecar files.

#### `extension.toml` Format (unchanged from original design)

```toml
[extension]
name = "my-extension"
version = "0.1.0"
description = "Brief description of what this extension provides"

# Functions exported to Scheme (registered as additional primitives)
[[extension.functions]]
name = "my_ext_do_thing"       # C symbol name (must be unique, use prefix)
scheme-name = "do-thing"       # Name as it appears in Scheme code
arity = 2
can-gc = false                 # Can this function trigger garbage collection?
can-raise = true               # Can this function raise a Scheme exception?

[[extension.functions]]
name = "my_ext_make_widget"
scheme-name = "make-widget"
arity = 1
can-gc = true
can-raise = false

# Optional: Scheme source files to prepend to user code (like lib/prelude.scm)
[extension.prelude]
files = ["lib/prelude.scm"]
```

### 3.3 Extension Implementation Rules

Extensions are Rust crates compiled to static libraries (`staticlib`). They must:

1. **Use `#[no_mangle] pub extern "C"` for all exported functions.**
   All parameters and return values are `u64` (the `Value` type). Extensions work with tagged values using the same tagging scheme as the runtime.

2. **Follow the naming convention**: All C symbol names should be prefixed to avoid collisions. We recommend `{ext_name}_{fn_name}` (e.g., `my_ext_do_thing`).

3. **Never call `scm_init()` or `scm_shutdown()`** — the main program owns the lifecycle.

4. **May call runtime functions** by depending on `schemer_runtime` as an `rlib` dependency, or by declaring them as `extern "C"` and relying on link-time resolution.

5. **Must declare GC interaction honestly** in `extension.toml`:
   - `can-gc = true` if the function allocates heap objects (`scm_alloc_*`, `scm_cons`, etc.) — this tells the compiler to emit GC safepoints around the call.
   - `can-raise = true` if the function calls `scm_raise` — this tells the compiler the call may not return normally.

6. **Must include a `build.rs`** that reads `extension.toml` and writes a JSON-encoded `ExtensionDescriptor` to `OUT_DIR/meta.bin`. See Section 3.6 for the reference implementation.

7. **Must embed metadata in `lib.rs`** using `#[link_section]` and `#[used]`:
   ```rust
   #[cfg_attr(target_os = "macos", link_section = "__DATA,__schemer_meta")]
   #[cfg_attr(target_os = "linux", link_section = ".schemer_meta")]
   #[used]
   pub static SCHEMER_METADATA: &[u8] =
       include_bytes!(concat!(env!("OUT_DIR"), "/meta.bin"));
   ```
   `#[used]` prevents rustc from dead-stripping the symbol before it reaches the archive. The section survives inside the `.o` members of the `.a` (archives are never stripped by the linker).

8. **`Cargo.toml` requirements**:
   - `crate-type = ["staticlib"]`
   - `build = "build.rs"` under `[package]`
   - `serde_json` and `serde` (with `derive` feature) in `[build-dependencies]`
   - `toml` in `[build-dependencies]` (if using the conventional `extension.toml` approach)

#### 3.3.1 Example Extension: `schemer-net`

`extension.toml`:
```toml
[extension]
name = "schemer-net"
version = "0.1.0"
description = "Basic TCP networking for Schemer"

[[extension.functions]]
name = "scm_net_tcp_connect"
scheme-name = "tcp-connect"
arity = 2
can-gc = true
can-raise = true

[[extension.functions]]
name = "scm_net_tcp_read"
scheme-name = "tcp-read"
arity = 1
can-gc = true
can-raise = true

[[extension.functions]]
name = "scm_net_tcp_write"
scheme-name = "tcp-write"
arity = 2
can-gc = false
can-raise = true

[[extension.functions]]
name = "scm_net_tcp_close"
scheme-name = "tcp-close"
arity = 1
can-gc = false
can-raise = true
```

`src/lib.rs`:
```rust
use std::net::TcpStream;
use std::io::{Read, Write};
use std::collections::HashMap;
use std::sync::Mutex;

// Value = u64 tagged pointer
type Value = u64;

// Import tag constants and helpers (these are resolved at link time from the runtime)
extern "C" {
    fn scm_alloc_string(data: *const u8, len: u64) -> Value;
    fn scm_raise(value: Value) -> !;
    fn scm_make_error(msg: *const u8, len: u64) -> Value;
}

// Metadata embedded into the compiled .a — Schemer reads this section at compile time
#[cfg_attr(target_os = "macos", link_section = "__DATA,__schemer_meta")]
#[cfg_attr(target_os = "linux", link_section = ".schemer_meta")]
#[used]
pub static SCHEMER_METADATA: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/meta.bin"));

// Simple handle table for open connections
static CONNECTIONS: Mutex<Option<HashMap<u64, TcpStream>>> = Mutex::new(None);

#[no_mangle]
pub extern "C" fn scm_net_tcp_connect(host: Value, port: Value) -> Value {
    // Implementation: extract string from host, fixnum from port,
    // open TcpStream, store in handle table, return handle as fixnum
    todo!()
}

#[no_mangle]
pub extern "C" fn scm_net_tcp_read(handle: Value) -> Value {
    // Implementation: read bytes, return as Scheme string
    todo!()
}

#[no_mangle]
pub extern "C" fn scm_net_tcp_write(handle: Value, data: Value) -> Value {
    // Implementation: write string data to connection
    todo!()
}

#[no_mangle]
pub extern "C" fn scm_net_tcp_close(handle: Value) -> Value {
    // Implementation: close and remove from handle table
    todo!()
}
```

### 3.4 How Extensions Integrate with the Compiler

When an extension is listed in a project's `schemer.toml` (see Section 4), the compiler does the following:

#### 3.4.1 Registration Phase (before compilation)

1. **Read the `.a` file** for each declared extension using the `object` crate.
2. **Find the `__schemer_meta` / `.schemer_meta` section** in the archive members.
3. **Deserialize the JSON bytes** into an `ExtensionDescriptor`.
4. **Merge extension functions into the runtime function table.** Each `functions` entry becomes a `RuntimeFn` appended to the list the codegen uses. The `scheme-name` is mapped as a new `PrimOp::ExtCall(symbol_name)` variant so the ANF transformer recognizes it.
5. **Prepend extension preludes.** Any Scheme files listed in `prelude_files` are prepended to the source (after the core prelude, before user code). Prelude paths are relative to the extension's declared `path` in `schemer.toml`.

#### 3.4.2 Compilation Phase (no changes needed)

The existing pipeline handles extension calls naturally:
- ANF: `(do-thing x y)` transforms to `ComplexExpr::PrimApp { op: PrimOp::ExtCall("my_ext_do_thing"), args: [x, y] }`.
- Codegen: Emits a QBE `call` to the extern symbol `$my_ext_do_thing`.
- QBE + assembler: Produces assembly with an unresolved external reference.

#### 3.4.3 Link Phase (extended)

The linker is extended to accept additional static libraries:

```
clang -o output program.s \
  libschemer_runtime.a \
  libmy_extension.a \        # <-- new: extension libraries
  libschemer_net.a \          # <-- new: another extension
  -lSystem
```

The `Linker` struct gains a new field:

```rust
pub struct Linker {
    target: Target,
    qbe_path: PathBuf,
    runtime_path: Option<PathBuf>,
    extension_libs: Vec<PathBuf>,   // NEW
    linker_flags: Vec<String>,
    keep_intermediates: bool,
}
```

### 3.5 Building Extensions

Extensions are built as standard Rust static libraries:

```bash
cd my-extension/
cargo build --release
# Produces: target/release/libmy_extension.a
# The .a contains a __schemer_meta / .schemer_meta section with JSON metadata.
```

The `Cargo.toml` must specify:

```toml
[package]
name = "my-extension"
version = "0.1.0"
edition = "2021"
build = "build.rs"

[lib]
crate-type = ["staticlib"]

[build-dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
```

---

## 3.6 Extension Build Requirements

Every extension crate must include a `build.rs` that embeds its metadata into the compiled `.a`.

### Metadata JSON Schema

The embedded JSON must deserialize into the `ExtensionDescriptor` struct (Section 6.1). The canonical schema:

```json
{
  "name": "my-extension",
  "version": "0.1.0",
  "description": "...",
  "functions": [
    {
      "symbol": "my_ext_do_thing",
      "scheme_name": "do-thing",
      "arity": 2,
      "can_gc": false,
      "can_raise": true
    }
  ],
  "prelude_files": ["lib/prelude.scm"]
}
```

Field names use `snake_case` (Rust convention) in the JSON, matching the `#[serde(rename_all = "snake_case")]` default.

### Reference `build.rs` Implementation

```rust
// build.rs — reference implementation for Schemer extensions

use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
struct ExtMeta {
    name: String,
    version: String,
    description: String,
    functions: Vec<ExtFnMeta>,
    prelude_files: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ExtFnMeta {
    symbol: String,
    scheme_name: String,
    arity: usize,
    can_gc: bool,
    can_raise: bool,
}

fn main() {
    // Re-run if extension.toml changes
    println!("cargo:rerun-if-changed=extension.toml");

    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("extension.toml");
    let toml_str = fs::read_to_string(&manifest_path)
        .expect("extension.toml not found — required for Schemer extension metadata");

    let raw: toml::Value = toml::from_str(&toml_str)
        .expect("extension.toml: invalid TOML");

    let ext = &raw["extension"];
    let functions_raw = ext.get("functions")
        .and_then(|v| v.as_array())
        .unwrap_or(&vec![]);

    let functions: Vec<ExtFnMeta> = functions_raw.iter().map(|f| ExtFnMeta {
        symbol:      f["name"].as_str().unwrap().to_string(),
        scheme_name: f["scheme-name"].as_str().unwrap().to_string(),
        arity:       f["arity"].as_integer().unwrap() as usize,
        can_gc:      f["can-gc"].as_bool().unwrap_or(false),
        can_raise:   f["can-raise"].as_bool().unwrap_or(false),
    }).collect();

    let prelude_files = ext
        .get("prelude")
        .and_then(|p| p.get("files"))
        .and_then(|f| f.as_array())
        .map(|arr| arr.iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect())
        .unwrap_or_default();

    let meta = ExtMeta {
        name:         ext["name"].as_str().unwrap().to_string(),
        version:      ext["version"].as_str().unwrap().to_string(),
        description:  ext.get("description")
                         .and_then(|v| v.as_str())
                         .unwrap_or("")
                         .to_string(),
        functions,
        prelude_files,
    };

    let json = serde_json::to_vec(&meta).expect("failed to serialize extension metadata");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("meta.bin"), &json)
        .expect("failed to write meta.bin");
}
```

### Section Name Convention

| Platform | `#[link_section]` value | Query name (object crate) |
|---|---|---|
| macOS (Mach-O) | `"__DATA,__schemer_meta"` | `"__schemer_meta"` |
| Linux (ELF) | `".schemer_meta"` | `".schemer_meta"` |

The `object` crate's `section_by_name` matches on the section name only (not the Mach-O segment prefix), so querying `"__schemer_meta"` finds `__DATA,__schemer_meta` on macOS.

---

## 4. Project Manifest (`schemer.toml`)

### 4.1 Purpose

A `schemer.toml` file in a directory marks it as a Schemer project. It declares:
- The project's source files and entry point.
- Dependencies on library extensions (as paths to compiled `.a` files or extension directories).
- Build configuration (output name, optimization level, etc.).

Running `schemer build` in a directory with a `schemer.toml` compiles the project into an executable, building any extension dependencies first.

### 4.2 Manifest Format

```toml
[project]
name = "my-game"
version = "0.1.0"
description = "A game of life implementation"

# Entry point — the Scheme file containing the main program
entry = "src/main.scm"

# Additional Scheme source files to include (prepended before entry, after preludes)
# Loaded in order. This is for project-local library code.
sources = [
    "src/utils.scm",
    "src/grid.scm",
]

[build]
# Output executable name (default: project name)
output = "game-of-life"

# Optimization level: 0 (none) or 1 (basic)
opt-level = 1

# Keep intermediate files for debugging (.ssa, .s)
keep-intermediates = false

# Additional linker flags
linker-flags = []

# Extension dependencies
[dependencies]
# Path-based dependency (points to extension source directory; Schemer builds it if needed)
schemer-net = { path = "../extensions/schemer-net" }

# Another extension
schemer-sdl = { path = "../extensions/schemer-sdl" }
```

### 4.3 Minimal Manifest

The smallest valid `schemer.toml`:

```toml
[project]
name = "hello"
entry = "main.scm"
```

This compiles `main.scm` (with the core prelude) into an executable named `hello`.

### 4.4 Source Loading Order

When compiling a project, sources are concatenated in this order:

```
1. lib/prelude.scm              (core Schemer prelude)
2. extension preludes            (in dependency declaration order, paths from ExtensionDescriptor.prelude_files)
3. project sources[]             (in declared order)
4. project entry                 (the main program)
```

This means:
- Extensions can define helper functions (in their `lib/prelude.scm`) that user code calls.
- Project `sources` can define shared utilities used by `entry`.
- The entry point runs last and constitutes the program's behavior.

---

## 5. Build Command (`schemer build`)

### 5.1 Behavior

When invoked in a directory containing `schemer.toml`:

```bash
schemer build
```

The build process:

1. **Parse `schemer.toml`** — validate the manifest.
2. **Resolve extensions** — for each `[dependencies]` entry:
   a. Locate the extension directory (currently: path-based only).
   b. If the extension's static library doesn't exist (or is stale), **build it** by running `cargo build --release` in the extension directory.
   c. **Read the `.a` file** using the `object` crate to extract the `__schemer_meta` / `.schemer_meta` section.
   d. **Deserialize the JSON** into an `ExtensionDescriptor`.
   e. Collect the `.a` library path, function descriptors, and prelude file paths.
3. **Assemble sources** — concatenate prelude + extension preludes + project sources + entry.
4. **Compile** — run the standard pipeline (parse -> ANF -> closure conversion -> codegen -> QBE -> link), with:
   - Extension functions registered in the primitive table.
   - Extension static libraries added to the linker command.
5. **Output** — produce the executable at the configured output path.

### 5.2 CLI Interface

The existing CLI is extended:

```
schemer [OPTIONS] [path]

SUBCOMMANDS:
    build       Build a project from schemer.toml
    init        Initialize a new schemer.toml in the current directory
    inspect     Print embedded metadata from a compiled extension .a file

OPTIONS:
    -c, --compile           Compile a single file (existing behavior)
    -o, --output <path>     Output file
    --emit-qbe              Output QBE IR only
    --emit-asm              Output assembly only
    --keep-intermediates    Keep .ssa and .s files
```

#### `schemer build`
```
schemer build [OPTIONS]

OPTIONS:
    --manifest <path>    Path to schemer.toml (default: ./schemer.toml)
    --release            Build with optimizations (default)
    --debug              Build without optimizations, keep intermediates
    -v, --verbose        Print build steps
```

#### `schemer init`
```
schemer init [OPTIONS]

OPTIONS:
    --name <name>        Project name (default: directory name)
    --extension          Initialize as an extension instead of a project
```

#### `schemer inspect`
```
schemer inspect <lib.a>

Reads and displays the embedded ExtensionDescriptor from a compiled .a file.
Useful for verifying that metadata was embedded correctly.
```

### 5.3 `schemer init` Output

For a project:
```
my-project/
├── schemer.toml
├── src/
│   └── main.scm        # (display "Hello, world!") (newline)
```

For an extension:
```
my-extension/
├── extension.toml
├── build.rs             # Reference build.rs (see Section 3.6)
├── Cargo.toml           # [lib] crate-type = ["staticlib"], build = "build.rs"
├── src/
│   └── lib.rs           # Skeleton with SCHEMER_METADATA static + one #[no_mangle] extern "C" fn
└── lib/
    └── prelude.scm      # Empty
```

---

## 6. Compiler Changes Required

### 6.1 New Types

```rust
// core/src/compiler/manifest.rs (new file)

/// Descriptor extracted from the __schemer_meta section of a compiled .a file
#[derive(Debug, serde::Deserialize)]
pub struct ExtensionDescriptor {
    pub name: String,
    pub version: String,
    pub description: String,
    pub functions: Vec<ExtensionFn>,
    pub prelude_files: Vec<PathBuf>,
}

/// A function exported by an extension
#[derive(Debug, serde::Deserialize)]
pub struct ExtensionFn {
    /// C symbol name (e.g. "scm_net_tcp_connect")
    pub symbol: String,
    /// Scheme-visible name (e.g. "tcp-connect")
    pub scheme_name: String,
    /// Number of parameters
    pub arity: usize,
    /// Can trigger GC
    pub can_gc: bool,
    /// Can raise an exception
    pub can_raise: bool,
}

/// Parsed schemer.toml
pub struct ProjectManifest {
    pub name: String,
    pub version: String,
    pub entry: PathBuf,
    pub sources: Vec<PathBuf>,
    pub build: BuildConfig,
    pub dependencies: Vec<DependencySpec>,
}

pub struct BuildConfig {
    pub output: Option<String>,
    pub opt_level: u8,
    pub keep_intermediates: bool,
    pub linker_flags: Vec<String>,
}

pub struct DependencySpec {
    pub name: String,
    pub path: PathBuf,
}

/// Resolved extension ready for compilation
pub struct ResolvedExtension {
    pub descriptor: ExtensionDescriptor,
    pub lib_path: PathBuf,         // Path to the compiled .a file
    pub prelude_source: String,    // Concatenated Scheme prelude content
}
```

### 6.2 Changes to `PrimOp`

```rust
// core/src/compiler/anf.rs — extend the PrimOp enum

pub enum PrimOp {
    // ... existing variants ...

    /// Call to an extension-provided function.
    /// The String is the C symbol name.
    ExtCall(String),
}
```

### 6.3 Changes to `get_primitive_impl()`

```rust
// core/src/compiler/primitives.rs

pub fn get_primitive_impl(op: &PrimOp) -> PrimImpl {
    match op {
        // ... existing arms ...
        PrimOp::ExtCall(symbol) => PrimImpl::RuntimeCall(symbol),
    }
}
```

### 6.4 Changes to ANF Transformer

The ANF transformer's symbol resolution (in `transform_application`) must be extended to check against registered extension functions. Currently, if a name isn't a known special form or primitive, it's treated as a user-defined function call. Extension functions would be added to the primitive lookup table before transformation begins.

```rust
// Before transformation, build a HashMap<String, PrimOp> that includes:
// - Built-in primitives ("+", "car", "display", etc.)
// - Extension primitives ("tcp-connect" -> PrimOp::ExtCall("scm_net_tcp_connect"))
```

### 6.5 Changes to `Linker`

```rust
impl Linker {
    /// Add an extension library to link
    pub fn extension_lib(mut self, path: impl Into<PathBuf>) -> Self {
        self.extension_libs.push(path.into());
        self
    }
}
```

In `link()`, after adding the runtime library, also add each extension library:

```rust
// Link with extension libraries
for ext_lib in &self.extension_libs {
    linker_cmd.arg(ext_lib);
}
```

### 6.6 Changes to `compile()` / `compile_file()`

A new top-level function `compile_project()` orchestrates manifest-based compilation:

```rust
/// Compile a project from a schemer.toml manifest
pub fn compile_project(manifest_path: &Path) -> Result<(), CompileError> {
    // 1. Parse schemer.toml
    // 2. Resolve extensions:
    //    a. Run cargo build --release if .a is stale
    //    b. Call read_extension_meta() to extract descriptor from .a
    //    c. Load prelude files
    // 3. Register extension functions into primitive table
    // 4. Assemble source (prelude + ext preludes + sources + entry)
    // 5. Compile with extended primitive table
    // 6. Link with extension libraries
}
```

### 6.7 New Module: `core/src/compiler/manifest.rs`

Responsibilities:
- Parse `schemer.toml` (using `toml` crate, for the project manifest only).
- **Read extension metadata from `.a` files** using the `object` crate.
- Resolve extension paths and build them via `cargo build --release`.
- Produce `ResolvedExtension` values for the compiler.

Key function:

```rust
use object::{read::archive::ArchiveFile, File as ObjFile, Object, ObjectSection};

/// Extract ExtensionDescriptor from a compiled .a static library.
/// Reads the __schemer_meta (macOS) or .schemer_meta (Linux) section
/// embedded by the extension's build.rs.
pub fn read_extension_meta(lib_path: &Path) -> Result<ExtensionDescriptor, ManifestError> {
    let bytes = std::fs::read(lib_path)
        .map_err(|e| ManifestError::IoError(lib_path.to_owned(), e))?;

    let archive = ArchiveFile::parse(&*bytes)
        .map_err(|_| ManifestError::NotAnArchive(lib_path.to_owned()))?;

    for member in archive.members() {
        let member = member.map_err(|_| ManifestError::ArchiveMemberError)?;
        let data = member.data(&*bytes).map_err(|_| ManifestError::ArchiveMemberError)?;

        if let Ok(obj) = ObjFile::parse(data) {
            // The object crate matches on section name only (not Mach-O segment prefix),
            // so "__schemer_meta" finds __DATA,__schemer_meta on macOS.
            let section_name = if cfg!(target_os = "macos") {
                "__schemer_meta"
            } else {
                ".schemer_meta"
            };

            if let Some(section) = obj.section_by_name(section_name) {
                let meta_bytes = section.data()
                    .map_err(|_| ManifestError::SectionReadError)?;
                return serde_json::from_slice(meta_bytes)
                    .map_err(|e| ManifestError::MetaDeserializeError(e));
            }
        }
    }

    Err(ManifestError::SectionNotFound(lib_path.to_owned()))
}
```

### 6.8 Dependencies

`core/Cargo.toml`:
```toml
[dependencies]
toml = "0.8"         # for parsing schemer.toml (project manifest only)
serde = { version = "1", features = ["derive"] }
serde_json = "1"     # for deserializing embedded extension metadata
object = "0.36"      # for reading .a archives and Mach-O/ELF sections
```

Note: `toml` is still needed for `schemer.toml` project manifest parsing, but is **no longer** used for reading extension descriptors (those come from the embedded JSON section). The `toml` dependency moves to extension `build.rs` for authoring, and remains in `core` only for project manifest parsing.

---

## 7. File Changes Summary

| File | Change |
|------|--------|
| `core/src/compiler/mod.rs` | Add `manifest` module, `compile_project()` function |
| `core/src/compiler/manifest.rs` | **New**: `read_extension_meta()` (object crate), `schemer.toml` parsing, extension resolution |
| `core/src/compiler/anf.rs` | Add `PrimOp::ExtCall(String)` variant |
| `core/src/compiler/primitives.rs` | Handle `ExtCall` in `get_primitive_impl()`, support dynamic fn registration |
| `core/src/compiler/link.rs` | Add `extension_libs: Vec<PathBuf>` to `Linker`, link them in `link()` |
| `core/src/compiler/codegen.rs` | No changes needed (already handles `RuntimeCall` generically) |
| `core/src/bin/cli/main.rs` | Add `build`, `init`, and `inspect` subcommands |
| `core/Cargo.toml` | Add `serde_json`, `object = "0.36"`; keep `toml`, `serde` |
| Each extension crate | Add `build.rs` (Section 3.6), update `lib.rs` with metadata static, update `Cargo.toml` |

---

## 8. Extension Discovery at Compile Time

Extensions are resolved **entirely at compile time**. There is no runtime extension loading. The discovery flow no longer reads sidecar TOML files — it reads directly from the compiled `.a`:

```
schemer.toml
    │
    └── [dependencies]
          │
          └── path to extension directory
                │
                ├── cargo build --release (if .a stale)
                │     └── build.rs reads extension.toml
                │           └── embeds JSON into .a section
                │
                └── read_extension_meta(lib.a)
                      │
                      ├── object crate: parse .a archive
                      ├── find __schemer_meta / .schemer_meta section in .o member
                      └── serde_json: deserialize -> ExtensionDescriptor
                              │
                              ├── functions[] -> PrimOp::ExtCall entries
                              ├── prelude_files -> prepended Scheme source
                              └── lib_path -> added to clang link command
```

The `.a` file is the **single source of truth** at Schemer compile time. If an extension's `extension.toml` is edited, the `.a` is considered stale and `cargo build` re-embeds the updated metadata. Schemer never reads `extension.toml` directly.

### Inspecting Embedded Metadata

```bash
schemer inspect libschemer_net.a
# Output:
# Extension: schemer-net v0.1.0
# Description: Basic TCP networking for Schemer
# Functions:
#   tcp-connect  (scm_net_tcp_connect, arity=2, can-gc, can-raise)
#   tcp-read     (scm_net_tcp_read,    arity=1, can-gc, can-raise)
#   tcp-write    (scm_net_tcp_write,   arity=2, can-raise)
#   tcp-close    (scm_net_tcp_close,   arity=1, can-raise)
# Prelude files: (none)
```

---

## 9. Calling Convention for Extension Functions

All extension functions follow the same calling convention as runtime functions:

```
extern "C" fn(arg1: u64, arg2: u64, ..., argN: u64) -> u64
```

Where:
- Each `u64` argument is a tagged `Value` (see Section 2.2 of runtime_types memory).
- The return value is a tagged `Value`.
- Functions receiving 0 arguments take no parameters.
- **Unlike closures**, extension functions do NOT receive a closure/environment pointer as a first argument. They are plain C functions, not Scheme closures.

This matches how existing runtime functions like `scm_cons(car, cdr)` and `scm_display(value)` work.

---

## 10. Error Handling

### 10.1 Build Errors

| Error | When |
|-------|------|
| `ManifestNotFound` | No `schemer.toml` in current directory |
| `ManifestParseError` | Invalid TOML syntax or missing required fields |
| `ExtensionNotFound` | Dependency path doesn't exist |
| `ExtensionBuildError` | `cargo build` fails for an extension |
| `SectionNotFound` | Compiled `.a` has no `__schemer_meta` / `.schemer_meta` section — extension was not built with a compliant `build.rs` |
| `MetaDeserializeError` | Embedded section bytes are not valid JSON or don't match `ExtensionDescriptor` schema |
| `DuplicateSymbol` | Two extensions export the same C symbol name |
| `DuplicateSchemeName` | Two extensions export the same Scheme name |
| `SymbolConflict` | Extension Scheme name conflicts with a built-in primitive |

### 10.2 Extension Compile Errors

Extension compilation failures surface as:
1. Scheme-name not found → standard "unbound variable" error (the ANF transformer already handles this).
2. Symbol not resolved at link time → linker error ("undefined symbol: scm_net_tcp_connect") — indicates the extension library wasn't built or wasn't linked.
3. Missing metadata section → `SectionNotFound` error with a hint: "Ensure your extension crate includes a compliant build.rs and the SCHEMER_METADATA static."

---

## 11. Future Extensions (v2+)

These are explicitly out of scope for v1 but recorded for future consideration:

1. **Versioned dependencies** — extensions declare semver ranges, dependency resolution.
2. **Registry** — `schemer add schemer-net` fetches from a package registry.
3. **R7RS `define-library`** — Scheme-level module system that can wrap extension functions.
4. **Shared library extensions** — `.dylib`/`.so` loaded at runtime for the interpreter.
5. **C extension API** — allow extensions written in plain C (not just Rust). C compilers can embed custom sections via `__attribute__((section(...)))`.
6. **Extension templates** — `schemer init --extension --template net` generates boilerplate.
7. **Cross-compilation** — build extensions for a different target triple than the host. Note: the `object` crate reads both Mach-O and ELF regardless of host, so cross-compiled `.a` files are inspectable on any host.
8. **Proc-macro metadata** — replace `build.rs` + TOML with a derive macro that generates the `SCHEMER_METADATA` static directly from annotations on exported functions.

---

## 12. Example: Full Project

```
my-game/
├── schemer.toml
├── src/
│   ├── main.scm
│   └── grid.scm
└── extensions/
    └── schemer-sdl/
        ├── extension.toml
        ├── build.rs
        ├── Cargo.toml
        ├── src/
        │   └── lib.rs
        └── lib/
            └── prelude.scm
```

`schemer.toml`:
```toml
[project]
name = "my-game"
version = "0.1.0"
entry = "src/main.scm"
sources = ["src/grid.scm"]

[build]
output = "game"

[dependencies]
schemer-sdl = { path = "extensions/schemer-sdl" }
```

`extensions/schemer-sdl/extension.toml`:
```toml
[extension]
name = "schemer-sdl"
version = "0.1.0"
description = "SDL2 bindings for Schemer"

[[extension.functions]]
name = "scm_sdl_init"
scheme-name = "sdl-init"
arity = 0
can-gc = false
can-raise = true

[[extension.functions]]
name = "scm_sdl_create_window"
scheme-name = "sdl-create-window"
arity = 4
can-gc = true
can-raise = true

[[extension.functions]]
name = "scm_sdl_draw_pixel"
scheme-name = "sdl-draw-pixel"
arity = 3
can-gc = false
can-raise = true

[[extension.functions]]
name = "scm_sdl_present"
scheme-name = "sdl-present"
arity = 0
can-gc = false
can-raise = false

[[extension.functions]]
name = "scm_sdl_quit"
scheme-name = "sdl-quit"
arity = 0
can-gc = false
can-raise = false

[extension.prelude]
files = ["lib/prelude.scm"]
```

`extensions/schemer-sdl/Cargo.toml`:
```toml
[package]
name = "schemer-sdl"
version = "0.1.0"
edition = "2021"
build = "build.rs"

[lib]
crate-type = ["staticlib"]

[build-dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"
```

`extensions/schemer-sdl/build.rs`:

See the reference implementation in Section 3.6. For `schemer-sdl`, the same pattern applies — the build script reads `extension.toml` and writes `OUT_DIR/meta.bin`.

`extensions/schemer-sdl/src/lib.rs`:
```rust
// Metadata embedded into the compiled .a for Schemer compiler discovery
#[cfg_attr(target_os = "macos", link_section = "__DATA,__schemer_meta")]
#[cfg_attr(target_os = "linux", link_section = ".schemer_meta")]
#[used]
pub static SCHEMER_METADATA: &[u8] =
    include_bytes!(concat!(env!("OUT_DIR"), "/meta.bin"));

type Value = u64;

extern "C" {
    fn scm_alloc_string(data: *const u8, len: u64) -> Value;
    fn scm_raise(value: Value) -> !;
    fn scm_make_error(msg: *const u8, len: u64) -> Value;
}

#[no_mangle]
pub extern "C" fn scm_sdl_init() -> Value { todo!() }

#[no_mangle]
pub extern "C" fn scm_sdl_create_window(title: Value, w: Value, h: Value, flags: Value) -> Value { todo!() }

#[no_mangle]
pub extern "C" fn scm_sdl_draw_pixel(win: Value, x: Value, y: Value) -> Value { todo!() }

#[no_mangle]
pub extern "C" fn scm_sdl_present(win: Value) -> Value { todo!() }

#[no_mangle]
pub extern "C" fn scm_sdl_quit() -> Value { todo!() }
```

`extensions/schemer-sdl/lib/prelude.scm`:
```scheme
(define (with-window title w h body)
  (sdl-init)
  (let ((win (sdl-create-window title w h 0)))
    (body win)
    (sdl-quit)))
```

`src/grid.scm`:
```scheme
(define (make-grid w h)
  (define cells (make-vector (* w h) 0))
  (list w h cells))

(define (grid-ref grid x y)
  (vector-ref (caddr grid) (+ (* y (car grid)) x)))

(define (grid-set! grid x y v)
  (vector-set! (caddr grid) (+ (* y (car grid)) x) v))
```

`src/main.scm`:
```scheme
(begin
  (define grid (make-grid 64 64))
  ;; ... seed pattern ...
  (with-window "Game of Life" 640 640
    (lambda (win)
      ;; render loop using sdl-draw-pixel, sdl-present
      (display "running!")
      (newline))))
```

Build:
```bash
cd my-game
schemer build
# 1. Detects extensions/schemer-sdl is stale
# 2. Runs: cd extensions/schemer-sdl && cargo build --release
#    build.rs reads extension.toml -> writes meta.bin
#    rustc embeds meta.bin into __DATA,__schemer_meta section of lib.o
#    ar packs -> libschemer_sdl.a (self-describing)
# 3. Runs: schemer inspect (internally) -> reads __schemer_meta -> ExtensionDescriptor
# 4. Registers sdl-init, sdl-create-window, etc. as PrimOp::ExtCall
# 5. Compiles src/grid.scm + src/main.scm (with sdl prelude + core prelude)
# 6. Links: program.s + libschemer_runtime.a + libschemer_sdl.a + -lSDL2 -lSystem
# 7. Produces: ./game
```

---

## 13. Implementation Phases

### Phase 1: Manifest Parsing & Project Build
- Implement `schemer.toml` parsing (TOML, project manifest only).
- Implement `schemer build` for simple projects (no extensions).
- Multi-source file concatenation.
- `schemer init` scaffolding.

### Phase 2: Extension Metadata & Linking
- Implement `read_extension_meta()` using the `object` crate (`object = "0.36"`).
  - Parse `.a` archive members.
  - Find `__schemer_meta` (macOS) / `.schemer_meta` (Linux) section.
  - Deserialize JSON → `ExtensionDescriptor`.
- Extend `Linker` to accept additional static libraries.
- Build extensions via `cargo build --release` when stale.
- Link extension `.a` files.
- Implement `schemer inspect <lib.a>` subcommand.

### Phase 3: Extension Function Registration
- Add `PrimOp::ExtCall(String)` to ANF.
- Extend the ANF transformer's primitive lookup to include extension functions.
- Handle `ExtCall` in codegen (trivial — already handles `RuntimeCall`).
- Validation: duplicate symbols, conflicts with builtins.

### Phase 4: Extension Preludes & Polish
- Extension prelude loading and ordering (paths from `ExtensionDescriptor.prelude_files`, resolved relative to extension directory).
- `schemer init --extension` scaffolding (generates `build.rs`, `extension.toml`, and the `SCHEMER_METADATA` static in `lib.rs`).
- Error messages and diagnostics, including `SectionNotFound` hint.
- Documentation and examples.
