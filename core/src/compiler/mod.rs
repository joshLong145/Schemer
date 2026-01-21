//! QBE Compiler Backend for Schemer
//!
//! This module provides native code compilation via QBE (Quick Backend).
//! The compilation pipeline is:
//!
//! ```text
//! Source (.scm) → Parser → Value AST → ANF IR → Closure Conversion → QBE IR → Assembly → Executable
//! ```

pub mod anf;
pub mod closure;
pub mod codegen;
pub mod link;
pub mod primitives;
pub mod qbe;

use std::path::{Path, PathBuf};

use crate::parser::read_all;

/// Compilation options
#[derive(Debug, Clone)]
pub struct CompileOptions {
    /// Emit QBE IR instead of executable
    pub emit_qbe: bool,
    /// Emit assembly instead of executable
    pub emit_asm: bool,
    /// Source file name (for error messages and debug info)
    pub source_name: String,
    /// Optimization level (0 = none, 1 = basic)
    pub opt_level: u8,
    /// Keep intermediate files (.ssa, .s)
    pub keep_intermediates: bool,
}

impl Default for CompileOptions {
    fn default() -> Self {
        Self {
            emit_qbe: false,
            emit_asm: false,
            source_name: "<unknown>".to_string(),
            opt_level: 0,
            keep_intermediates: true,
        }
    }
}

impl CompileOptions {
    pub fn keep_intermediates(mut self, keep: bool) -> Self {
        self.keep_intermediates = keep;
        self
    }
}

/// Compile Scheme source to native executable
pub fn compile(source: &str, output: &Path, options: CompileOptions) -> Result<(), CompileError> {
    // Step 1: Parse source to AST
    let ast = read_all(source).map_err(|e| CompileError::ParseError(e.msg))?;

    // Step 2: Transform AST to ANF
    let mut transformer = anf::AnfTransformer::new();
    let anf_program = transformer
        .transform_program(ast)
        .map_err(CompileError::AnfError)?;

    // Step 3: Closure conversion
    let mut converter = closure::ClosureConverter::new();
    let converted = converter.convert(anf_program);

    // Step 4: Generate QBE IR
    let mut generator = codegen::CodeGenerator::new();
    let qbe_module = generator.generate(&converted);

    // Step 5: Write QBE IR
    let qbe_ir = qbe_module.to_string();

    if options.emit_qbe {
        std::fs::write(output, &qbe_ir).map_err(|e| CompileError::IoError(e.to_string()))?;
        return Ok(());
    }

    // Step 6: Invoke QBE and linker
    let linker = link::Linker::new(link::Target::Aarch64AppleDarwin)
        .keep_intermediates(options.keep_intermediates);
    linker
        .runtime_path(
            std::env::var("SCHEMER_RUNTIME_PATH")
                .map_err(|e| CompileError::IoError(e.to_string()))?,
        )
        .link(&qbe_ir, output, options.emit_asm)
        .map_err(CompileError::LinkError)?;

    Ok(())
}

/// Compile a Scheme source file to native executable
pub fn compile_file(input_path: &str, output_path: &str) -> Result<(), CompileError> {
    // Load prelude from lib/prelude.scm relative to cwd
    let prelude = std::fs::read_to_string("lib/prelude.scm").unwrap_or_default(); // Silently skip if not found

    let user_source = std::fs::read_to_string(input_path)
        .map_err(|e| CompileError::IoError(format!("Failed to read {}: {}", input_path, e)))?;

    // Combine prelude + user code
    let source = format!("{}\n{}", prelude, user_source);

    let output = Path::new(output_path);
    compile(
        &source,
        output,
        CompileOptions::default().keep_intermediates(true),
    )
}

/// Compilation error types
#[derive(Debug)]
pub enum CompileError {
    ParseError(String),
    AnfError(String),
    ClosureError(String),
    CodegenError(String),
    LinkError(link::LinkError),
    IoError(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            CompileError::AnfError(msg) => write!(f, "ANF transformation error: {}", msg),
            CompileError::ClosureError(msg) => write!(f, "Closure conversion error: {}", msg),
            CompileError::CodegenError(msg) => write!(f, "Code generation error: {}", msg),
            CompileError::LinkError(e) => write!(f, "Link error: {:?}", e),
            CompileError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for CompileError {}
