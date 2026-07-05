# QBE Compiler Backend Implementation Workflow

## Document Metadata

| Attribute | Value |
|-----------|-------|
| Project | Schemer QBE Compiler Backend |
| Created | 2025-01-18 |
| Status | Ready for Implementation |
| Phases | 9 |
| Estimated Tasks | 47 |

---

## Executive Summary

This workflow implements a native code compiler backend for the Schemer interpreter using QBE as the code generator. The implementation follows a bottom-up approach: foundational infrastructure first, then progressively more complex features.

### Key Deliverables
1. ANF intermediate representation and transformation
2. Closure conversion pass
3. QBE IR code generation
4. Runtime library with reference counting GC
5. Trampoline-based tail call optimization
6. Exception handling with backtraces
7. CLI integration (`--compile` flag)

### Target Platform
- ARM64 macOS (Apple Silicon)
- ARM64 Linux

---

## Dependency Graph

```
Phase 1: Foundation
    ├── 1.1 Project structure
    ├── 1.2 Shared tag definitions
    └── 1.3 Macro infrastructure

Phase 2: Runtime Core (can start after 1)
    ├── 2.1 Value representation
    ├── 2.2 Memory allocation
    ├── 2.3 Basic primitives
    └── 2.4 Build as static library

Phase 3: ANF Transformation (can start after 1)
    ├── 3.1 ANF IR types
    ├── 3.2 AST → ANF transform
    └── 3.3 ANF tests

Phase 4: QBE Codegen Basics (requires 2, 3)
    ├── 4.1 QBE IR types
    ├── 4.2 QBE writer
    ├── 4.3 Basic codegen (no closures)
    └── 4.4 Linker integration

Phase 5: Trampoline TCO (requires 4)
    ├── 5.1 Trampoline runtime
    ├── 5.2 Bounce struct codegen
    └── 5.3 Tail call detection

Phase 6: Closures (requires 5)
    ├── 6.1 Free variable analysis
    ├── 6.2 Closure conversion
    ├── 6.3 Closure runtime support
    └── 6.4 Box for mutables

Phase 7: Reference Counting (requires 6)
    ├── 7.1 Incref/decref codegen
    ├── 7.2 Scope-based decref
    ├── 7.3 Trial deletion
    └── 7.4 GC tests

Phase 8: Exceptions (requires 7)
    ├── 8.1 setjmp/longjmp setup
    ├── 8.2 Backtrace capture
    ├── 8.3 Error procedures
    └── 8.4 Exception-safe GC

Phase 9: Integration (requires 8)
    ├── 9.1 CLI --compile flag
    ├── 9.2 Example compilation
    ├── 9.3 Performance benchmarks
    └── 9.4 Documentation
```

---

## Phase 1: Foundation & Infrastructure

**Goal**: Set up project structure, shared definitions, and macro infrastructure.

**Duration Estimate**: 1-2 days

### Task 1.1: Project Structure Setup

**Priority**: Critical | **Blocking**: All other tasks

```
Create directories and Cargo configuration:

src/
├── compiler/
│   ├── mod.rs
│   ├── anf.rs
│   ├── closure.rs
│   ├── codegen.rs
│   ├── qbe.rs
│   ├── qbe_macros.rs
│   ├── primitives.rs
│   └── link.rs
├── tags.rs              # Shared tag definitions

runtime/                  # Separate crate
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── macros.rs
│   ├── value.rs
│   ├── alloc.rs
│   ├── gc.rs
│   ├── trampoline.rs
│   ├── exception.rs
│   ├── primitives.rs
│   └── backtrace.rs
└── build.rs
```

**Acceptance Criteria**:
- [ ] `cargo build` succeeds with empty modules
- [ ] Runtime crate builds as `cdylib` and `staticlib`
- [ ] Main crate has `compiler` feature flag

**Files to Create**:
1. `src/compiler/mod.rs` - Module exports
2. `src/tags.rs` - Shared tag constants
3. `runtime/Cargo.toml` - Runtime crate config
4. `runtime/src/lib.rs` - Runtime entry point
5. Update `Cargo.toml` - Add workspace and features

---

### Task 1.2: Shared Tag Definitions

**Priority**: Critical | **Depends on**: 1.1

```rust
// src/tags.rs - Shared between compiler and runtime

/// Tag bit width
pub const TAG_BITS: u32 = 3;
pub const TAG_MASK: u64 = 0b111;

/// Immediate type tags (low 3 bits)
pub const POINTER_TAG: u64 = 0b000;
pub const FIXNUM_TAG: u64 = 0b001;
pub const CHAR_TAG: u64 = 0b010;
pub const SPECIAL_TAG: u64 = 0b011;

/// Special value constants
pub const NIL_VALUE: u64 = 0b0_011;      // nil
pub const TRUE_VALUE: u64 = 0b1_011;     // #t  
pub const FALSE_VALUE: u64 = 0b10_011;   // #f
pub const VOID_VALUE: u64 = 0b11_011;    // void
pub const EOF_VALUE: u64 = 0b100_011;    // eof-object

/// Heap object type tags
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectType {
    Pair = 1,
    Vector = 2,
    String = 3,
    Symbol = 4,
    Closure = 5,
    Box = 6,
    Float = 7,
    Bounce = 8,
    ArgsArray = 9,
}
```

**Acceptance Criteria**:
- [ ] Tags compile in both main crate and runtime
- [ ] Constants are `pub` and accessible

---

### Task 1.3: Macro Infrastructure

**Priority**: High | **Depends on**: 1.1

Create core macros that will be used throughout:

```rust
// runtime/src/macros.rs

/// Export function for C FFI
#[macro_export]
macro_rules! define_rt_fn {
    ($name:ident($($arg:ident : $arg_ty:ty),* $(,)?) -> $ret:ty $body:block) => {
        #[no_mangle]
        #[inline]
        pub extern "C" fn $name($($arg: $arg_ty),*) -> $ret $body
    };
    
    ($name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $body:block) => {
        #[no_mangle]
        #[inline]
        pub extern "C" fn $name($($arg: $arg_ty),*) $body
    };
}

/// Type-check and extract heap object
#[macro_export]
macro_rules! with_typed {
    ($val:expr, $type_tag:expr, $rust_ty:ty, $name:ident => $body:expr) => {{
        let __val = $val;
        if !__val.is_pointer() {
            $crate::exception::rt_type_error_val(stringify!($rust_ty), __val);
        }
        unsafe {
            let __obj = __val.as_ptr();
            if (*__obj).type_tag != $type_tag {
                $crate::exception::rt_type_error_val(stringify!($rust_ty), __val);
            }
            let $name = __obj as *mut $rust_ty;
            $body
        }
    }};
}

// Convenience macros
#[macro_export]
macro_rules! with_pair {
    ($val:expr, $name:ident => $body:expr) => {
        $crate::with_typed!($val, $crate::ObjectType::Pair, $crate::Pair, $name => $body)
    };
}
```

**Acceptance Criteria**:
- [ ] Macros compile without errors
- [ ] Test macro with dummy function

---

## Phase 2: Runtime Core

**Goal**: Implement runtime value representation and basic primitives.

**Duration Estimate**: 3-4 days

### Task 2.1: Value Representation

**Priority**: Critical | **Depends on**: 1.2

```rust
// runtime/src/value.rs

use std::sync::atomic::AtomicU32;

/// Tagged 64-bit Scheme value
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SchemeValue(pub u64);

impl SchemeValue {
    pub const NIL: Self = Self(NIL_VALUE);
    pub const TRUE: Self = Self(TRUE_VALUE);
    pub const FALSE: Self = Self(FALSE_VALUE);
    pub const VOID: Self = Self(VOID_VALUE);
    
    #[inline]
    pub fn is_pointer(self) -> bool {
        (self.0 & TAG_MASK) == POINTER_TAG && self.0 != 0
    }
    
    #[inline]
    pub fn is_fixnum(self) -> bool {
        (self.0 & TAG_MASK) == FIXNUM_TAG
    }
    
    #[inline]
    pub fn fixnum(n: i64) -> Self {
        Self(((n as u64) << TAG_BITS) | FIXNUM_TAG)
    }
    
    #[inline]
    pub fn as_fixnum(self) -> i64 {
        (self.0 as i64) >> TAG_BITS
    }
    
    #[inline]
    pub fn from_ptr(ptr: *mut HeapObject) -> Self {
        Self(ptr as u64)
    }
    
    #[inline]
    pub fn as_ptr(self) -> *mut HeapObject {
        (self.0 & !TAG_MASK) as *mut HeapObject
    }
    
    #[inline]
    pub fn is_truthy(self) -> bool {
        self != Self::FALSE
    }
}

/// Heap object header
#[repr(C)]
pub struct HeapObject {
    pub refcount: AtomicU32,
    pub type_tag: ObjectType,
    pub gc_flags: u8,
    pub _reserved: u16,
}

/// Pair (cons cell)
#[repr(C)]
pub struct Pair {
    pub header: HeapObject,
    pub car: SchemeValue,
    pub cdr: SchemeValue,
}

/// Closure
#[repr(C)]
pub struct Closure {
    pub header: HeapObject,
    pub func_ptr: *const (),
    pub num_captures: usize,
    // captures follow inline
}

/// Mutable box
#[repr(C)]
pub struct BoxCell {
    pub header: HeapObject,
    pub value: SchemeValue,
}
```

**Acceptance Criteria**:
- [ ] `SchemeValue::fixnum(42).as_fixnum() == 42`
- [ ] `SchemeValue::NIL.is_pointer() == false`
- [ ] Pointer round-trip works
- [ ] All structs have correct `#[repr(C)]` layout

---

### Task 2.2: Memory Allocation

**Priority**: Critical | **Depends on**: 2.1

```rust
// runtime/src/alloc.rs

use std::alloc::{alloc, dealloc, Layout};

/// Allocate a pair
pub fn alloc_pair() -> *mut Pair {
    unsafe {
        let layout = Layout::new::<Pair>();
        let ptr = alloc(layout) as *mut Pair;
        (*ptr).header.refcount = AtomicU32::new(1);
        (*ptr).header.type_tag = ObjectType::Pair;
        (*ptr).header.gc_flags = 0;
        ptr
    }
}

/// Allocate a closure with N captures
pub fn alloc_closure(num_captures: usize) -> *mut Closure {
    unsafe {
        let size = std::mem::size_of::<Closure>() 
            + num_captures * std::mem::size_of::<SchemeValue>();
        let layout = Layout::from_size_align_unchecked(size, 8);
        let ptr = alloc(layout) as *mut Closure;
        (*ptr).header.refcount = AtomicU32::new(1);
        (*ptr).header.type_tag = ObjectType::Closure;
        (*ptr).header.gc_flags = 0;
        (*ptr).num_captures = num_captures;
        ptr
    }
}

/// Allocate a mutable box
pub fn alloc_box() -> *mut BoxCell {
    unsafe {
        let layout = Layout::new::<BoxCell>();
        let ptr = alloc(layout) as *mut BoxCell;
        (*ptr).header.refcount = AtomicU32::new(1);
        (*ptr).header.type_tag = ObjectType::Box;
        (*ptr).header.gc_flags = 0;
        ptr
    }
}

/// Deallocate a heap object
pub unsafe fn dealloc_object(obj: *mut HeapObject, size: usize) {
    let layout = Layout::from_size_align_unchecked(size, 8);
    dealloc(obj as *mut u8, layout);
}
```

**Acceptance Criteria**:
- [ ] Allocate and free pair without crash
- [ ] Closure with captures allocates correct size
- [ ] No memory leaks under valgrind/ASAN

---

### Task 2.3: Basic Primitives

**Priority**: High | **Depends on**: 2.2, 1.3

```rust
// runtime/src/primitives.rs

use crate::*;

define_rt_fn!(rt_cons(car: SchemeValue, cdr: SchemeValue) -> SchemeValue {
    let pair = alloc_pair();
    unsafe {
        (*pair).car = car;
        (*pair).cdr = cdr;
    }
    // Note: incref handled by caller in Phase 7
    SchemeValue::from_ptr(pair as *mut HeapObject)
});

define_rt_fn!(rt_car(pair: SchemeValue) -> SchemeValue {
    with_pair!(pair, p => unsafe { (*p).car })
});

define_rt_fn!(rt_cdr(pair: SchemeValue) -> SchemeValue {
    with_pair!(pair, p => unsafe { (*p).cdr })
});

define_rt_fn!(rt_is_null(val: SchemeValue) -> SchemeValue {
    if val == SchemeValue::NIL {
        SchemeValue::TRUE
    } else {
        SchemeValue::FALSE
    }
});

define_rt_fn!(rt_is_pair(val: SchemeValue) -> SchemeValue {
    if val.is_pointer() {
        unsafe {
            if (*val.as_ptr()).type_tag == ObjectType::Pair {
                return SchemeValue::TRUE;
            }
        }
    }
    SchemeValue::FALSE
});

define_rt_fn!(rt_display(val: SchemeValue) {
    print!("{}", format_value(val));
});

define_rt_fn!(rt_newline() {
    println!();
});

// Internal formatting
fn format_value(val: SchemeValue) -> String {
    if val.is_fixnum() {
        format!("{}", val.as_fixnum())
    } else if val == SchemeValue::TRUE {
        "#t".to_string()
    } else if val == SchemeValue::FALSE {
        "#f".to_string()
    } else if val == SchemeValue::NIL {
        "()".to_string()
    } else if val.is_pointer() {
        format_heap_object(val)
    } else {
        "#<unknown>".to_string()
    }
}
```

**Acceptance Criteria**:
- [ ] `rt_cons` creates valid pair
- [ ] `rt_car`/`rt_cdr` extract values
- [ ] `rt_display` prints correctly
- [ ] Type predicates work

---

### Task 2.4: Runtime Static Library Build

**Priority**: Critical | **Depends on**: 2.3

```toml
# runtime/Cargo.toml
[package]
name = "schemer-runtime"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["staticlib", "cdylib", "rlib"]

[dependencies]
# Minimal dependencies for runtime

[build-dependencies]
# For generating header if needed
```

```rust
// runtime/build.rs
fn main() {
    // Optionally generate C header
    println!("cargo:rerun-if-changed=src/");
}
```

**Acceptance Criteria**:
- [ ] `cargo build --release` produces `libschemer_runtime.a`
- [ ] Library exports all `rt_*` symbols
- [ ] Can link with simple C test program

---

## Phase 3: ANF Transformation

**Goal**: Transform AST to A-Normal Form intermediate representation.

**Duration Estimate**: 3-4 days

### Task 3.1: ANF IR Types

**Priority**: Critical | **Depends on**: 1.1

```rust
// src/compiler/anf.rs

use std::collections::HashSet;

/// Variable identifier
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarId(pub String);

impl VarId {
    pub fn new(name: &str) -> Self { VarId(name.to_string()) }
    pub fn temp(id: u64) -> Self { VarId(format!("_t{}", id)) }
}

/// Atomic expressions (immediate operands)
#[derive(Clone, Debug)]
pub enum Atom {
    Var(VarId),
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(usize),  // Index into string table
    Symbol(usize),  // Index into symbol table
    Nil,
    Void,
}

/// Primitive operations
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PrimOp {
    // Arithmetic
    Add, Sub, Mul, Div, Mod,
    // Comparison
    NumEq, Lt, Gt, Le, Ge,
    // Type predicates
    IsNull, IsPair, IsNumber, IsBool, IsSymbol, IsString, IsProc,
    // List operations
    Cons, Car, Cdr, SetCar, SetCdr,
    // Equality
    Eq, Eqv, Equal,
    // I/O
    Display, Newline,
}

/// Complex expressions (need binding)
#[derive(Clone, Debug)]
pub enum ComplexExpr {
    PrimApp { op: PrimOp, args: Vec<Atom> },
    App { func: Atom, args: Vec<Atom> },
    TailApp { func: Atom, args: Vec<Atom> },
    MakeClosure { label: String, captures: Vec<VarId> },
    ClosureRef { closure: VarId, index: usize },
    If { cond: Atom, then_expr: Box<AnfExpr>, else_expr: Box<AnfExpr> },
    MakeBox(Atom),
    ReadBox(VarId),
    WriteBox { box_var: VarId, value: Atom },
}

/// ANF expressions
#[derive(Clone, Debug)]
pub enum AnfExpr {
    Return(Atom),
    Let { var: VarId, value: ComplexExpr, body: Box<AnfExpr> },
    Seq { effect: ComplexExpr, body: Box<AnfExpr> },
    TailCall { func: Atom, args: Vec<Atom> },
    Halt(Atom),
}

/// Function definition
#[derive(Clone, Debug)]
pub struct FunctionDef {
    pub label: String,
    pub source_name: Option<String>,
    pub params: Vec<VarId>,
    pub has_env: bool,
    pub body: AnfExpr,
    pub free_vars: HashSet<VarId>,
}

/// Complete program
#[derive(Clone, Debug)]
pub struct AnfProgram {
    pub functions: Vec<FunctionDef>,
    pub entry: AnfExpr,
    pub strings: Vec<String>,
    pub symbols: Vec<String>,
}
```

**Acceptance Criteria**:
- [ ] All types compile
- [ ] Can construct simple ANF manually
- [ ] Debug printing works

---

### Task 3.2: AST to ANF Transformation

**Priority**: Critical | **Depends on**: 3.1

```rust
// src/compiler/anf.rs (continued)

pub struct AnfTransformer {
    temp_counter: u64,
    functions: Vec<FunctionDef>,
    strings: Vec<String>,
    symbols: Vec<String>,
    current_function: Option<String>,
}

impl AnfTransformer {
    pub fn new() -> Self {
        Self {
            temp_counter: 0,
            functions: Vec::new(),
            strings: Vec::new(),
            symbols: Vec::new(),
            current_function: None,
        }
    }
    
    pub fn transform_program(&mut self, exprs: Vec<Value>) -> Result<AnfProgram, String> {
        // Transform top-level defines and expressions
        let entry = self.transform_top_level(exprs)?;
        
        Ok(AnfProgram {
            functions: std::mem::take(&mut self.functions),
            entry,
            strings: std::mem::take(&mut self.strings),
            symbols: std::mem::take(&mut self.symbols),
        })
    }
    
    fn fresh_temp(&mut self) -> VarId {
        let id = self.temp_counter;
        self.temp_counter += 1;
        VarId::temp(id)
    }
    
    /// Transform expression, tracking tail position
    fn transform(&mut self, expr: &Value, tail_pos: bool) -> Result<AnfExpr, String> {
        match expr {
            Value::Number(n) => Ok(AnfExpr::Return(self.number_to_atom(n))),
            Value::Boolean(b) => Ok(AnfExpr::Return(Atom::Bool(*b))),
            Value::Nil => Ok(AnfExpr::Return(Atom::Nil)),
            Value::Symbol(s) => Ok(AnfExpr::Return(Atom::Var(VarId::new(s)))),
            Value::List(list) => self.transform_list(list, tail_pos),
            Value::Pair(p) => self.transform_pair(p, tail_pos),
            _ => Err(format!("Cannot transform: {:?}", expr)),
        }
    }
    
    fn transform_list(&mut self, list: &SchemeList, tail_pos: bool) -> Result<AnfExpr, String> {
        let elements: Vec<_> = list.iter().cloned().collect();
        if elements.is_empty() {
            return Ok(AnfExpr::Return(Atom::Nil));
        }
        
        match &elements[0] {
            Value::Symbol(s) if s == "if" => self.transform_if(&elements[1..], tail_pos),
            Value::Symbol(s) if s == "lambda" => self.transform_lambda(&elements[1..]),
            Value::Symbol(s) if s == "let" => self.transform_let(&elements[1..], tail_pos),
            Value::Symbol(s) if s == "begin" => self.transform_begin(&elements[1..], tail_pos),
            Value::Symbol(s) if s == "define" => self.transform_define(&elements[1..]),
            Value::Symbol(s) if s == "set!" => self.transform_set(&elements[1..]),
            Value::Symbol(s) if s == "quote" => self.transform_quote(&elements[1..]),
            _ => self.transform_application(&elements, tail_pos),
        }
    }
    
    // ... implement each special form transformer
}
```

**Acceptance Criteria**:
- [ ] `(+ 1 2)` transforms correctly
- [ ] `(if cond then else)` transforms with branches
- [ ] `(lambda (x) body)` creates FunctionDef
- [ ] Nested expressions normalize properly

---

### Task 3.3: ANF Transformation Tests

**Priority**: High | **Depends on**: 3.2

```rust
// tests/anf.rs

#[test]
fn test_anf_simple_arithmetic() {
    let ast = parse("(+ 1 2)");
    let anf = transform(ast);
    // Verify structure
}

#[test]
fn test_anf_nested_arithmetic() {
    let ast = parse("(+ (* 2 3) (- 5 1))");
    let anf = transform(ast);
    // Should have let bindings for intermediate values
}

#[test]
fn test_anf_if_expression() {
    let ast = parse("(if (= x 0) 1 (* x 2))");
    let anf = transform(ast);
    // Verify If complex expr
}

#[test]
fn test_anf_lambda() {
    let ast = parse("(lambda (x) (+ x 1))");
    let anf = transform(ast);
    // Verify FunctionDef created
}

#[test]
fn test_anf_tail_call() {
    let ast = parse("(define (f x) (f (+ x 1)))");
    let anf = transform(ast);
    // Body should be TailCall, not App
}
```

**Acceptance Criteria**:
- [ ] All tests pass
- [ ] Edge cases handled (empty list, nested quotes)
- [ ] Error messages are clear

---

## Phase 4: QBE Code Generation (Basic)

**Goal**: Generate QBE IR for simple programs without closures.

**Duration Estimate**: 4-5 days

### Task 4.1: QBE IR Types

**Priority**: Critical | **Depends on**: 3.1

```rust
// src/compiler/qbe.rs

#[derive(Clone, Debug)]
pub enum QbeType {
    Word,   // w - 32-bit
    Long,   // l - 64-bit
    Single, // s - 32-bit float
    Double, // d - 64-bit float
}

#[derive(Clone, Debug)]
pub enum QbeValue {
    Temp(String),
    Global(String),
    Const(i64),
}

#[derive(Clone, Debug)]
pub enum QbeOp {
    Add(QbeValue, QbeValue),
    Sub(QbeValue, QbeValue),
    Mul(QbeValue, QbeValue),
    Div(QbeValue, QbeValue),
    And(QbeValue, QbeValue),
    Or(QbeValue, QbeValue),
    Sar(QbeValue, QbeValue),
    Shl(QbeValue, QbeValue),
    Ceql(QbeValue, QbeValue),
    Csltl(QbeValue, QbeValue),
    Copy(QbeValue),
}

#[derive(Clone, Debug)]
pub enum QbeInst {
    Assign { dest: String, ty: QbeType, op: QbeOp },
    Call { dest: Option<String>, func: QbeValue, args: Vec<(QbeType, QbeValue)> },
    Store { ty: QbeType, value: QbeValue, addr: QbeValue },
    Load { dest: String, ty: QbeType, addr: QbeValue },
    Jmp(String),
    Jnz { cond: QbeValue, if_true: String, if_false: String },
    Ret(Option<QbeValue>),
    Hlt,
}

#[derive(Clone, Debug)]
pub struct QbeBlock {
    pub label: String,
    pub instructions: Vec<QbeInst>,
}

#[derive(Clone, Debug)]
pub struct QbeFunction {
    pub export: bool,
    pub name: String,
    pub params: Vec<(QbeType, String)>,
    pub return_type: Option<QbeType>,
    pub blocks: Vec<QbeBlock>,
}

#[derive(Clone, Debug)]
pub struct QbeData {
    pub export: bool,
    pub name: String,
    pub items: Vec<QbeDataItem>,
}

#[derive(Clone, Debug)]
pub struct QbeModule {
    pub functions: Vec<QbeFunction>,
    pub data: Vec<QbeData>,
}
```

**Acceptance Criteria**:
- [ ] All QBE constructs representable
- [ ] Types have correct size/alignment info

---

### Task 4.2: QBE IR Writer

**Priority**: Critical | **Depends on**: 4.1

```rust
// src/compiler/qbe.rs (continued)

impl QbeModule {
    pub fn write(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        // Write data section
        for data in &self.data {
            data.write(w)?;
        }
        
        writeln!(w)?;
        
        // Write functions
        for func in &self.functions {
            func.write(w)?;
            writeln!(w)?;
        }
        
        Ok(())
    }
}

impl QbeFunction {
    pub fn write(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        if self.export {
            write!(w, "export ")?;
        }
        
        write!(w, "function ")?;
        
        if let Some(ref ty) = self.return_type {
            write!(w, "{} ", ty.to_qbe())?;
        }
        
        write!(w, "${}(", self.name)?;
        
        for (i, (ty, name)) in self.params.iter().enumerate() {
            if i > 0 { write!(w, ", ")?; }
            write!(w, "{} %{}", ty.to_qbe(), name)?;
        }
        
        writeln!(w, ") {{")?;
        
        for block in &self.blocks {
            writeln!(w, "@{}", block.label)?;
            for inst in &block.instructions {
                write!(w, "    ")?;
                inst.write(w)?;
                writeln!(w)?;
            }
        }
        
        writeln!(w, "}}")?;
        Ok(())
    }
}

impl QbeInst {
    pub fn write(&self, w: &mut impl std::io::Write) -> std::io::Result<()> {
        match self {
            QbeInst::Assign { dest, ty, op } => {
                write!(w, "%{} ={} ", dest, ty.to_qbe())?;
                op.write(w)
            }
            QbeInst::Call { dest, func, args } => {
                if let Some(d) = dest {
                    write!(w, "%{} =l ", d)?;
                }
                write!(w, "call ")?;
                func.write(w)?;
                write!(w, "(")?;
                for (i, (ty, val)) in args.iter().enumerate() {
                    if i > 0 { write!(w, ", ")?; }
                    write!(w, "{} ", ty.to_qbe())?;
                    val.write(w)?;
                }
                write!(w, ")")
            }
            QbeInst::Ret(val) => {
                write!(w, "ret")?;
                if let Some(v) = val {
                    write!(w, " ")?;
                    v.write(w)?;
                }
                Ok(())
            }
            QbeInst::Jmp(label) => write!(w, "jmp @{}", label),
            QbeInst::Jnz { cond, if_true, if_false } => {
                write!(w, "jnz ")?;
                cond.write(w)?;
                write!(w, ", @{}, @{}", if_true, if_false)
            }
            // ... other instructions
        }
    }
}
```

**Acceptance Criteria**:
- [ ] Generated QBE IR is syntactically valid
- [ ] `qbe` tool accepts output
- [ ] All instruction types write correctly

---

### Task 4.3: Basic Code Generation

**Priority**: Critical | **Depends on**: 4.2, 3.2

```rust
// src/compiler/codegen.rs

pub struct CodeGenerator {
    blocks: Vec<QbeBlock>,
    current_block: QbeBlock,
    temp_counter: u64,
    label_counter: u64,
}

impl CodeGenerator {
    pub fn generate(&mut self, program: &AnfProgram) -> QbeModule {
        let mut functions = Vec::new();
        let mut data = Vec::new();
        
        // Generate string constants
        for (i, s) in program.strings.iter().enumerate() {
            data.push(self.generate_string_data(i, s));
        }
        
        // Generate each function
        for func in &program.functions {
            functions.push(self.generate_function(func));
        }
        
        // Generate main
        functions.push(self.generate_main(&program.entry));
        
        QbeModule { functions, data }
    }
    
    fn generate_function(&mut self, func: &FunctionDef) -> QbeFunction {
        self.reset();
        
        let params: Vec<_> = func.params.iter()
            .map(|p| (QbeType::Long, p.0.clone()))
            .collect();
        
        self.generate_expr(&func.body);
        
        self.finalize_blocks();
        
        QbeFunction {
            export: false,
            name: func.label.clone(),
            params,
            return_type: Some(QbeType::Long),
            blocks: std::mem::take(&mut self.blocks),
        }
    }
    
    fn generate_expr(&mut self, expr: &AnfExpr) -> QbeValue {
        match expr {
            AnfExpr::Return(atom) => {
                let val = self.atom_to_qbe(atom);
                self.emit(QbeInst::Ret(Some(val.clone())));
                val
            }
            
            AnfExpr::Let { var, value, body } => {
                let result = self.generate_complex(value);
                self.emit_assign(&var.0, result);
                self.generate_expr(body)
            }
            
            AnfExpr::TailCall { func, args } => {
                self.generate_tail_call(func, args)
            }
            
            // ... other cases
        }
    }
    
    fn generate_complex(&mut self, expr: &ComplexExpr) -> QbeValue {
        match expr {
            ComplexExpr::PrimApp { op, args } => {
                self.generate_prim_op(op, args)
            }
            
            ComplexExpr::App { func, args } => {
                self.generate_call(func, args)
            }
            
            ComplexExpr::If { cond, then_expr, else_expr } => {
                self.generate_if(cond, then_expr, else_expr)
            }
            
            // ... other cases
        }
    }
    
    fn generate_prim_op(&mut self, op: &PrimOp, args: &[Atom]) -> QbeValue {
        match op {
            PrimOp::Add => {
                let a = self.unbox_fixnum(&args[0]);
                let b = self.unbox_fixnum(&args[1]);
                let sum = self.emit_binop(QbeOp::Add, a, b);
                self.box_fixnum(sum)
            }
            
            PrimOp::Cons => {
                let car = self.atom_to_qbe(&args[0]);
                let cdr = self.atom_to_qbe(&args[1]);
                self.emit_call("rt_cons", vec![car, cdr])
            }
            
            // ... other primitives
        }
    }
}
```

**Acceptance Criteria**:
- [ ] Simple arithmetic compiles
- [ ] Function calls work
- [ ] If/else generates correct branches
- [ ] Output assembles with QBE

---

### Task 4.4: Linker Integration

**Priority**: High | **Depends on**: 4.3, 2.4

```rust
// src/compiler/link.rs

use std::process::Command;
use std::path::Path;
use tempfile::TempDir;

pub struct Linker {
    qbe_path: String,
    runtime_path: String,
}

impl Linker {
    pub fn new() -> Self {
        Self {
            qbe_path: std::env::var("QBE_PATH").unwrap_or_else(|_| "qbe".to_string()),
            runtime_path: Self::find_runtime(),
        }
    }
    
    fn find_runtime() -> String {
        // Look for libschemer_runtime.a
        let candidates = [
            "target/release/libschemer_runtime.a",
            "target/debug/libschemer_runtime.a",
        ];
        
        for path in candidates {
            if Path::new(path).exists() {
                return path.to_string();
            }
        }
        
        panic!("Runtime library not found. Build with: cd runtime && cargo build --release");
    }
    
    pub fn link(&self, qbe_ir: &str, output: &Path) -> Result<(), String> {
        let temp = TempDir::new().map_err(|e| e.to_string())?;
        
        let ssa_path = temp.path().join("program.ssa");
        let asm_path = temp.path().join("program.s");
        
        // Write QBE IR
        std::fs::write(&ssa_path, qbe_ir).map_err(|e| e.to_string())?;
        
        // Run QBE: qbe -t arm64 program.ssa -o program.s
        let qbe_status = Command::new(&self.qbe_path)
            .args(["-t", "arm64"])
            .arg(&ssa_path)
            .arg("-o")
            .arg(&asm_path)
            .status()
            .map_err(|e| format!("Failed to run qbe: {}", e))?;
        
        if !qbe_status.success() {
            return Err("QBE compilation failed".to_string());
        }
        
        // Link: cc program.s -L. -lschemer_runtime -o output
        let cc_status = Command::new("cc")
            .arg(&asm_path)
            .arg(&self.runtime_path)
            .arg("-o")
            .arg(output)
            .status()
            .map_err(|e| format!("Failed to link: {}", e))?;
        
        if !cc_status.success() {
            return Err("Linking failed".to_string());
        }
        
        Ok(())
    }
}
```

**Acceptance Criteria**:
- [ ] QBE invoked correctly
- [ ] Assembly links with runtime
- [ ] Output binary runs
- [ ] Error messages are helpful

---

## Phase 5: Trampoline TCO

**Goal**: Implement tail call optimization using trampoline pattern.

**Duration Estimate**: 2-3 days

### Task 5.1: Trampoline Runtime

**Priority**: Critical | **Depends on**: 4.4

```rust
// runtime/src/trampoline.rs

/// Result of a function call - either done or bounce
#[repr(C)]
pub union TrampolineResult {
    pub done: SchemeValue,
    pub bounce: BounceData,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BounceData {
    pub tag: u64,  // 1 = bounce
    pub func: *const (),
    pub args: *mut SchemeValue,
    pub argc: usize,
}

/// Main trampoline loop
define_rt_fn!(rt_trampoline(
    func: *const (), 
    args: *mut SchemeValue, 
    argc: usize
) -> SchemeValue {
    let mut current_func = func;
    let mut current_args = args;
    let mut current_argc = argc;
    
    loop {
        // Call function
        let f: extern "C" fn(*mut SchemeValue, usize) -> TrampolineResult = 
            unsafe { std::mem::transmute(current_func) };
        
        let result = f(current_args, current_argc);
        
        // Free previous args if heap-allocated
        if !current_args.is_null() {
            unsafe { rt_free_args(current_args, current_argc) };
        }
        
        unsafe {
            if result.bounce.tag == 0 {
                // Done - return value
                return result.done;
            } else {
                // Bounce - continue with new function
                current_func = result.bounce.func;
                current_args = result.bounce.args;
                current_argc = result.bounce.argc;
            }
        }
    }
});

define_rt_fn!(rt_alloc_args(argc: usize) -> *mut SchemeValue {
    if argc == 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        let layout = std::alloc::Layout::array::<SchemeValue>(argc).unwrap();
        std::alloc::alloc(layout) as *mut SchemeValue
    }
});

define_rt_fn!(rt_free_args(args: *mut SchemeValue, argc: usize) {
    if args.is_null() || argc == 0 { return; }
    unsafe {
        let layout = std::alloc::Layout::array::<SchemeValue>(argc).unwrap();
        std::alloc::dealloc(args as *mut u8, layout);
    }
});
```

**Acceptance Criteria**:
- [ ] Trampoline loop works
- [ ] Bounce detection correct
- [ ] Args properly allocated/freed
- [ ] Infinite tail recursion doesn't overflow stack

---

### Task 5.2: Bounce Codegen

**Priority**: Critical | **Depends on**: 5.1

```rust
// src/compiler/codegen.rs (additions)

impl CodeGenerator {
    fn generate_tail_call(&mut self, func: &Atom, args: &[Atom]) -> QbeValue {
        let func_val = self.atom_to_qbe(func);
        let argc = args.len();
        
        // Allocate args array
        let args_ptr = self.emit_call("rt_alloc_args", vec![QbeValue::Const(argc as i64)]);
        
        // Store each argument
        for (i, arg) in args.iter().enumerate() {
            let arg_val = self.atom_to_qbe(arg);
            let offset = QbeValue::Const((i * 8) as i64);
            let addr = self.emit_binop(QbeOp::Add, args_ptr.clone(), offset);
            self.emit(QbeInst::Store {
                ty: QbeType::Long,
                value: arg_val,
                addr,
            });
        }
        
        // Create bounce result
        // tag=1, func, args, argc
        let bounce = self.fresh_temp();
        
        // For simplicity, return a tagged pointer to bounce struct
        // Alternatively, use multiple return values
        let bounce_ptr = self.emit_call(
            "rt_make_bounce",
            vec![func_val, args_ptr, QbeValue::Const(argc as i64)],
        );
        
        self.emit(QbeInst::Ret(Some(bounce_ptr)));
        
        QbeValue::Const(0) // Unreachable
    }
    
    fn generate_non_tail_call(&mut self, func: &Atom, args: &[Atom]) -> QbeValue {
        // For non-tail calls, go through trampoline
        let func_val = self.atom_to_qbe(func);
        let argc = args.len();
        
        let args_ptr = self.emit_call("rt_alloc_args", vec![QbeValue::Const(argc as i64)]);
        
        for (i, arg) in args.iter().enumerate() {
            let arg_val = self.atom_to_qbe(arg);
            let offset = QbeValue::Const((i * 8) as i64);
            let addr = self.emit_binop(QbeOp::Add, args_ptr.clone(), offset);
            self.emit(QbeInst::Store {
                ty: QbeType::Long,
                value: arg_val,
                addr,
            });
        }
        
        self.emit_call(
            "rt_trampoline",
            vec![func_val, args_ptr, QbeValue::Const(argc as i64)],
        )
    }
}
```

**Acceptance Criteria**:
- [ ] Tail calls create bounce
- [ ] Non-tail calls use trampoline
- [ ] Stack doesn't grow on tail recursion

---

### Task 5.3: Tail Call Detection

**Priority**: High | **Depends on**: 5.2

Already handled in ANF transformation (Task 3.2) - the `tail_pos` parameter tracks whether we're in tail position. `TailApp` vs `App` distinguishes the two.

**Acceptance Criteria**:
- [ ] Direct tail recursion detected
- [ ] Mutual tail recursion works
- [ ] Non-tail calls correctly identified

---

## Phase 6: Closures

**Goal**: Implement first-class closures with captured variables.

**Duration Estimate**: 3-4 days

### Task 6.1: Free Variable Analysis

**Priority**: Critical | **Depends on**: 3.2

```rust
// src/compiler/closure.rs

use std::collections::HashSet;

pub fn analyze_free_vars(expr: &AnfExpr, bound: &HashSet<VarId>) -> HashSet<VarId> {
    match expr {
        AnfExpr::Return(atom) => free_vars_atom(atom, bound),
        
        AnfExpr::Let { var, value, body } => {
            let mut fv = free_vars_complex(value, bound);
            let mut new_bound = bound.clone();
            new_bound.insert(var.clone());
            fv.extend(analyze_free_vars(body, &new_bound));
            fv
        }
        
        AnfExpr::Seq { effect, body } => {
            let mut fv = free_vars_complex(effect, bound);
            fv.extend(analyze_free_vars(body, bound));
            fv
        }
        
        AnfExpr::TailCall { func, args } => {
            let mut fv = free_vars_atom(func, bound);
            for arg in args {
                fv.extend(free_vars_atom(arg, bound));
            }
            fv
        }
        
        AnfExpr::Halt(atom) => free_vars_atom(atom, bound),
    }
}

fn free_vars_atom(atom: &Atom, bound: &HashSet<VarId>) -> HashSet<VarId> {
    match atom {
        Atom::Var(v) if !bound.contains(v) => {
            let mut set = HashSet::new();
            set.insert(v.clone());
            set
        }
        _ => HashSet::new(),
    }
}
```

**Acceptance Criteria**:
- [ ] Free variables correctly identified
- [ ] Bound variables excluded
- [ ] Nested lambdas handled

---

### Task 6.2: Closure Conversion

**Priority**: Critical | **Depends on**: 6.1

```rust
// src/compiler/closure.rs (continued)

pub struct ClosureConverter {
    lambda_counter: u64,
    converted_functions: Vec<FunctionDef>,
}

impl ClosureConverter {
    pub fn convert(&mut self, program: AnfProgram) -> AnfProgram {
        let mut new_functions = Vec::new();
        
        for func in program.functions {
            new_functions.push(self.convert_function(func));
        }
        
        new_functions.extend(self.converted_functions.drain(..));
        
        let entry = self.convert_expr(&program.entry, &HashSet::new());
        
        AnfProgram {
            functions: new_functions,
            entry,
            strings: program.strings,
            symbols: program.symbols,
        }
    }
    
    fn convert_function(&mut self, mut func: FunctionDef) -> FunctionDef {
        let bound: HashSet<_> = func.params.iter().cloned().collect();
        func.body = self.convert_expr(&func.body, &bound);
        func
    }
    
    fn convert_expr(&mut self, expr: &AnfExpr, bound: &HashSet<VarId>) -> AnfExpr {
        match expr {
            AnfExpr::Let { var, value, body } => {
                let new_value = self.convert_complex(value, bound);
                let mut new_bound = bound.clone();
                new_bound.insert(var.clone());
                
                AnfExpr::Let {
                    var: var.clone(),
                    value: new_value,
                    body: Box::new(self.convert_expr(body, &new_bound)),
                }
            }
            
            // Handle lambda -> MakeClosure conversion here
            // ...
            
            _ => expr.clone(),
        }
    }
    
    fn lift_lambda(
        &mut self,
        params: &[VarId],
        body: &AnfExpr,
        bound: &HashSet<VarId>,
    ) -> (String, Vec<VarId>) {
        // Compute free variables
        let param_set: HashSet<_> = params.iter().cloned().collect();
        let free = analyze_free_vars(body, &param_set);
        
        // Filter to only truly free (not bound in enclosing scope)
        let captures: Vec<_> = free.into_iter()
            .filter(|v| !bound.contains(v))
            .collect();
        
        // Create lifted function
        let label = format!("lambda_{}", self.lambda_counter);
        self.lambda_counter += 1;
        
        let mut lifted_params = vec![VarId::new("__env")];
        lifted_params.extend(params.iter().cloned());
        
        // Convert body with captures as ClosureRef
        let converted_body = self.convert_with_captures(body, &captures);
        
        let func = FunctionDef {
            label: label.clone(),
            source_name: None,
            params: lifted_params,
            has_env: true,
            body: converted_body,
            free_vars: HashSet::new(),
        };
        
        self.converted_functions.push(func);
        
        (label, captures)
    }
}
```

**Acceptance Criteria**:
- [ ] Lambdas lifted to top level
- [ ] Free variables captured
- [ ] Closure access generates ClosureRef
- [ ] Nested closures work

---

### Task 6.3: Closure Runtime Support

**Priority**: High | **Depends on**: 6.2

```rust
// runtime/src/primitives.rs (additions)

define_rt_fn!(rt_make_closure(
    func_ptr: *const (),
    captures: *const SchemeValue,
    num_captures: usize
) -> SchemeValue {
    let closure = alloc_closure(num_captures);
    
    unsafe {
        (*closure).func_ptr = func_ptr;
        (*closure).num_captures = num_captures;
        
        for i in 0..num_captures {
            let val = *captures.add(i);
            (*closure).captures_mut()[i] = val;
            rt_incref(val);
        }
    }
    
    SchemeValue::from_ptr(closure as *mut HeapObject)
});

define_rt_fn!(rt_closure_ref(closure: SchemeValue, index: usize) -> SchemeValue {
    with_closure!(closure, c => unsafe {
        let val = (*c).captures()[index];
        rt_incref(val);
        val
    })
});

define_rt_fn!(rt_closure_func(closure: SchemeValue) -> *const () {
    with_closure!(closure, c => unsafe { (*c).func_ptr })
});
```

**Acceptance Criteria**:
- [ ] Closures allocate correctly
- [ ] Captures stored and retrieved
- [ ] Function pointer extractable

---

### Task 6.4: Box for Mutable Variables

**Priority**: High | **Depends on**: 6.3

Variables targeted by `set!` need boxing:

```rust
// src/compiler/closure.rs (additions)

pub fn find_mutated_vars(expr: &AnfExpr) -> HashSet<VarId> {
    let mut mutated = HashSet::new();
    find_mutated_vars_impl(expr, &mut mutated);
    mutated
}

fn find_mutated_vars_impl(expr: &AnfExpr, mutated: &mut HashSet<VarId>) {
    match expr {
        AnfExpr::Let { value, body, .. } => {
            if let ComplexExpr::WriteBox { box_var, .. } = value {
                mutated.insert(box_var.clone());
            }
            find_mutated_vars_impl(body, mutated);
        }
        // ... other cases
    }
}

// Transform set! targets to use boxes
pub fn insert_boxes(func: &mut FunctionDef) {
    let mutated = find_mutated_vars(&func.body);
    
    if mutated.is_empty() {
        return;
    }
    
    // Wrap mutated params in boxes at function entry
    // Transform reads to ReadBox, writes to WriteBox
    func.body = transform_for_boxes(&func.body, &mutated);
}
```

**Acceptance Criteria**:
- [ ] `set!` targets identified
- [ ] Boxes inserted for mutated vars
- [ ] Read/write through boxes works

---

## Phase 7: Reference Counting

**Goal**: Implement automatic memory management.

**Duration Estimate**: 3-4 days

### Task 7.1: Incref/Decref in Codegen

**Priority**: Critical | **Depends on**: 6.4

```rust
// src/compiler/codegen.rs (additions)

impl CodeGenerator {
    fn emit_incref(&mut self, val: QbeValue) {
        self.emit_call_void("rt_incref", vec![val]);
    }
    
    fn emit_decref(&mut self, val: QbeValue) {
        self.emit_call_void("rt_decref", vec![val]);
    }
    
    // When assigning to a variable
    fn emit_assign_with_rc(&mut self, var: &str, new_val: QbeValue) {
        // Incref new value
        self.emit_incref(new_val.clone());
        
        // If overwriting, decref old
        // (tracked separately or handled by scope)
        
        self.emit_assign(var, new_val);
    }
}
```

**Acceptance Criteria**:
- [ ] Incref on parameter passing
- [ ] Decref on scope exit
- [ ] Assignment properly managed

---

### Task 7.2: Scope-Based Decref

**Priority**: Critical | **Depends on**: 7.1

```rust
// Track live variables and emit decrefs at scope exit

impl CodeGenerator {
    fn generate_with_scope(&mut self, expr: &AnfExpr, live: &mut Vec<VarId>) -> QbeValue {
        match expr {
            AnfExpr::Let { var, value, body } => {
                let result = self.generate_complex(value);
                self.emit_incref(result.clone());
                self.emit_assign(&var.0, result);
                
                live.push(var.clone());
                
                let body_result = self.generate_with_scope(body, live);
                
                // Scope ends - decref if not returned
                // ...
                
                body_result
            }
            
            AnfExpr::Return(atom) => {
                let val = self.atom_to_qbe(atom);
                
                // Decref all live variables except returned one
                for v in live.iter() {
                    if !matches!(atom, Atom::Var(ref av) if av == v) {
                        self.emit_decref(QbeValue::Temp(v.0.clone()));
                    }
                }
                
                self.emit(QbeInst::Ret(Some(val.clone())));
                val
            }
            
            // ... other cases
        }
    }
}
```

**Acceptance Criteria**:
- [ ] Variables decrefd when out of scope
- [ ] Return value not decrefd
- [ ] No double-free

---

### Task 7.3: Trial Deletion

**Priority**: High | **Depends on**: 7.2

```rust
// runtime/src/gc.rs

use std::collections::VecDeque;

static mut CANDIDATES: VecDeque<*mut HeapObject> = VecDeque::new();
const THRESHOLD: usize = 1000;

pub fn possible_root(obj: *mut HeapObject) {
    unsafe {
        if !is_buffered(obj) {
            set_buffered(obj, true);
            CANDIDATES.push_back(obj);
            
            if CANDIDATES.len() >= THRESHOLD {
                collect_cycles();
            }
        }
    }
}

pub fn collect_cycles() {
    unsafe {
        let candidates: Vec<_> = CANDIDATES.drain(..).collect();
        
        // Mark gray
        for &obj in &candidates {
            mark_gray(obj);
        }
        
        // Scan
        for &obj in &candidates {
            scan(obj);
        }
        
        // Collect white
        for &obj in &candidates {
            collect_white(obj);
        }
    }
}

unsafe fn mark_gray(obj: *mut HeapObject) {
    if is_marked(obj) { return; }
    set_marked(obj, true);
    
    for_each_child(obj, |child| {
        decrement_rc(child);
        mark_gray(child);
    });
}

unsafe fn scan(obj: *mut HeapObject) {
    if !is_marked(obj) { return; }
    
    if get_refcount(obj) > 0 {
        scan_black(obj);
    } else {
        set_marked(obj, false);
        for_each_child(obj, scan);
    }
}

unsafe fn scan_black(obj: *mut HeapObject) {
    set_marked(obj, false);
    set_buffered(obj, false);
    
    for_each_child(obj, |child| {
        increment_rc(child);
        if is_marked(child) {
            scan_black(child);
        }
    });
}

unsafe fn collect_white(obj: *mut HeapObject) {
    if is_marked(obj) || is_buffered(obj) { return; }
    
    if get_refcount(obj) == 0 {
        for_each_child(obj, collect_white);
        deallocate(obj);
    }
}
```

**Acceptance Criteria**:
- [ ] Cycles detected
- [ ] Cyclic garbage collected
- [ ] Non-cyclic GC still works
- [ ] No memory leaks

---

### Task 7.4: GC Tests

**Priority**: High | **Depends on**: 7.3

```rust
// tests/gc.rs

#[test]
fn test_simple_allocation_free() {
    let pair = rt_cons(SchemeValue::fixnum(1), SchemeValue::NIL);
    rt_decref(pair);
    // Should not crash, memory freed
}

#[test]
fn test_cyclic_structure() {
    // Create (a . b) where b points back to a
    let a = rt_cons(SchemeValue::fixnum(1), SchemeValue::NIL);
    unsafe {
        let pair = a.as_ptr() as *mut Pair;
        (*pair).cdr = a;  // Cycle!
        rt_incref(a);
    }
    
    rt_decref(a);
    
    // Force cycle collection
    collect_cycles();
    
    // Should be freed, no leak
}

#[test]
fn test_deep_nesting() {
    // Create deeply nested list
    let mut list = SchemeValue::NIL;
    for i in 0..1000 {
        list = rt_cons(SchemeValue::fixnum(i), list);
    }
    
    rt_decref(list);
    // Should free all 1000 pairs
}
```

**Acceptance Criteria**:
- [ ] All GC tests pass
- [ ] No memory leaks under valgrind
- [ ] Reasonable performance

---

## Phase 8: Exception Handling

**Goal**: Implement setjmp/longjmp based exceptions with backtraces.

**Duration Estimate**: 2-3 days

### Task 8.1: setjmp/longjmp Setup

**Priority**: Critical | **Depends on**: 7.4

```rust
// runtime/src/exception.rs

use std::ffi::c_void;

#[cfg(target_arch = "aarch64")]
type JmpBuf = [u64; 32];

extern "C" {
    fn setjmp(buf: *mut c_void) -> i32;
    fn longjmp(buf: *mut c_void, val: i32) -> !;
}

#[repr(C)]
pub struct ExceptionContext {
    jmp_buf: JmpBuf,
    exception: SchemeValue,
    prev: *mut ExceptionContext,
}

thread_local! {
    static CURRENT_HANDLER: std::cell::Cell<*mut ExceptionContext> = 
        std::cell::Cell::new(std::ptr::null_mut());
}

define_rt_fn!(rt_push_handler(ctx: *mut ExceptionContext) {
    CURRENT_HANDLER.with(|h| {
        unsafe { (*ctx).prev = h.get(); }
        h.set(ctx);
    });
});

define_rt_fn!(rt_pop_handler() -> *mut ExceptionContext {
    CURRENT_HANDLER.with(|h| {
        let current = h.get();
        if !current.is_null() {
            unsafe { h.set((*current).prev); }
        }
        current
    })
});

define_rt_fn!(rt_throw(exception: SchemeValue) -> ! {
    CURRENT_HANDLER.with(|h| {
        let handler = h.get();
        
        if handler.is_null() {
            eprintln!("Unhandled exception: {}", format_value(exception));
            print_backtrace();
            std::process::exit(1);
        }
        
        unsafe {
            (*handler).exception = exception;
            longjmp((*handler).jmp_buf.as_ptr() as *mut c_void, 1);
        }
    })
});
```

**Acceptance Criteria**:
- [ ] Handler stack works
- [ ] Exception caught by handler
- [ ] Unhandled exception prints and exits

---

### Task 8.2: Backtrace Capture

**Priority**: High | **Depends on**: 8.1

```rust
// runtime/src/backtrace.rs

use std::collections::HashMap;
use std::sync::RwLock;

lazy_static::lazy_static! {
    static ref FUNCTION_NAMES: RwLock<HashMap<usize, String>> = 
        RwLock::new(HashMap::new());
}

define_rt_fn!(rt_register_function(addr: usize, name: *const i8) {
    let name_str = unsafe {
        std::ffi::CStr::from_ptr(name).to_string_lossy().into_owned()
    };
    FUNCTION_NAMES.write().unwrap().insert(addr, name_str);
});

pub fn print_backtrace() {
    eprintln!("\nBacktrace:");
    
    let names = FUNCTION_NAMES.read().unwrap();
    
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut fp: usize;
        std::arch::asm!("mov {}, x29", out(reg) fp);
        
        let mut depth = 0;
        while fp != 0 && depth < 50 {
            let lr = *(fp as *const usize).add(1);
            let prev_fp = *(fp as *const usize);
            
            let name = names.get(&lr)
                .map(|s| s.as_str())
                .unwrap_or("<unknown>");
            
            eprintln!("  {}: {} (0x{:x})", depth, name, lr);
            
            fp = prev_fp;
            depth += 1;
        }
    }
}
```

**Acceptance Criteria**:
- [ ] Function names registered at startup
- [ ] Backtrace captures call stack
- [ ] Names displayed in errors

---

### Task 8.3: Error Procedures

**Priority**: High | **Depends on**: 8.2

```rust
// runtime/src/exception.rs (additions)

define_rt_fn!(rt_type_error_val(expected: &'static str, got: SchemeValue) -> ! {
    eprintln!("Type error: expected {}, got {}", expected, format_value(got));
    print_backtrace();
    std::process::exit(1)
});

define_rt_fn!(rt_error(msg: SchemeValue) -> ! {
    eprintln!("Error: {}", format_value(msg));
    print_backtrace();
    std::process::exit(1)
});

define_rt_fn!(rt_assert(cond: SchemeValue, msg: SchemeValue) {
    if !cond.is_truthy() {
        eprintln!("Assertion failed: {}", format_value(msg));
        print_backtrace();
        std::process::exit(1);
    }
});
```

**Acceptance Criteria**:
- [ ] Type errors show helpful messages
- [ ] Backtraces included
- [ ] User `error` procedure works

---

### Task 8.4: Exception-Safe GC

**Priority**: High | **Depends on**: 8.3

Ensure decrefs happen even when exception is thrown:

```rust
// When unwinding, decref live variables
// This may require tracking in exception context

impl ExceptionContext {
    live_vars: Vec<SchemeValue>,
}

// On throw, decref all live vars in current scope
unsafe fn cleanup_on_throw(ctx: *mut ExceptionContext) {
    for &val in &(*ctx).live_vars {
        rt_decref(val);
    }
}
```

**Acceptance Criteria**:
- [ ] No leaks on exception
- [ ] Cleanup happens before longjmp
- [ ] Nested handlers work

---

## Phase 9: Integration & Polish

**Goal**: Integrate compiler into CLI and validate with examples.

**Duration Estimate**: 2-3 days

### Task 9.1: CLI --compile Flag

**Priority**: Critical | **Depends on**: 8.4

```rust
// src/bin/cli/main.rs (modifications)

use clap::{Arg, Command};

fn main() {
    let matches = Command::new("schemer")
        .version("0.1.0")
        .about("A Scheme implementation")
        .arg(Arg::new("path").long("path").help("Interpret file"))
        .arg(Arg::new("compile").long("compile").help("Compile file"))
        .arg(Arg::new("output").short('o').long("output").help("Output path"))
        .arg(Arg::new("emit-qbe").long("emit-qbe").action(clap::ArgAction::SetTrue))
        .get_matches();
    
    if let Some(path) = matches.get_one::<String>("compile") {
        compile_file(path, &matches);
    } else if let Some(path) = matches.get_one::<String>("path") {
        interpret_file(path);
    } else {
        repl();
    }
}

fn compile_file(path: &str, matches: &clap::ArgMatches) {
    let source = std::fs::read_to_string(path).expect("Failed to read file");
    
    let output = matches.get_one::<String>("output")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let mut p = PathBuf::from(path);
            p.set_extension("");
            p
        });
    
    match schemer::compiler::compile(&source, &output) {
        Ok(()) => println!("Compiled to {}", output.display()),
        Err(e) => {
            eprintln!("Compilation error: {}", e);
            std::process::exit(1);
        }
    }
}
```

**Acceptance Criteria**:
- [ ] `schemer --compile foo.scm -o foo` works
- [ ] `schemer --path foo.scm` still interprets
- [ ] `schemer` with no args starts REPL
- [ ] `--emit-qbe` outputs IR

---

### Task 9.2: Example Compilation

**Priority**: Critical | **Depends on**: 9.1

Test all examples compile and produce correct output:

```bash
#!/bin/bash
# test_examples.sh

for f in examples/*.scm; do
    echo "Testing $f..."
    
    # Interpret
    expected=$(./target/release/schemer --path "$f")
    
    # Compile and run
    ./target/release/schemer --compile "$f" -o /tmp/test_prog
    actual=$(/tmp/test_prog)
    
    if [ "$expected" = "$actual" ]; then
        echo "  PASS"
    else
        echo "  FAIL"
        echo "  Expected: $expected"
        echo "  Got: $actual"
    fi
done
```

**Acceptance Criteria**:
- [ ] All examples compile
- [ ] Output matches interpreter
- [ ] No crashes or memory errors

---

### Task 9.3: Performance Benchmarks

**Priority**: Medium | **Depends on**: 9.2

```scheme
; benchmarks/factorial.scm
(define (factorial n)
  (if (= n 0)
      1
      (* n (factorial (- n 1)))))

(factorial 20)
```

```scheme
; benchmarks/fibonacci.scm
(define (fib n)
  (if (< n 2)
      n
      (+ (fib (- n 1)) (fib (- n 2)))))

(fib 30)
```

**Acceptance Criteria**:
- [ ] Compiled code at least 10x faster than interpreter
- [ ] Tail recursion doesn't overflow stack
- [ ] Reasonable compilation time

---

### Task 9.4: Documentation

**Priority**: Medium | **Depends on**: 9.3

Update README with:
- Compilation usage
- Supported features
- Platform requirements (QBE, ARM64)
- Known limitations

**Acceptance Criteria**:
- [ ] README updated
- [ ] Examples documented
- [ ] Build instructions clear

---

## Validation Checkpoints

| Checkpoint | After Phase | Validation |
|------------|-------------|------------|
| CP1 | 2 | Runtime builds, primitives work in isolation |
| CP2 | 4 | Simple programs compile and run |
| CP3 | 5 | Recursive programs don't overflow stack |
| CP4 | 6 | Closures and higher-order functions work |
| CP5 | 7 | No memory leaks, cycles collected |
| CP6 | 8 | Errors produce helpful backtraces |
| CP7 | 9 | All examples pass, performance targets met |

---

## Risk Mitigation

| Risk | Mitigation |
|------|------------|
| QBE ARM64 bugs | Test on both macOS and Linux ARM64 early |
| Trial deletion perf | Tune threshold, consider incremental collection |
| setjmp + GC interaction | Design cleanup paths carefully |
| Closure overhead | Consider lambda lifting for non-escaping |

---

## Next Steps

After workflow approval:

1. **Run `/sc:implement`** to begin Phase 1
2. Build and validate incrementally
3. Each phase has clear acceptance criteria
4. Test at each checkpoint before proceeding

Ready to begin implementation?
