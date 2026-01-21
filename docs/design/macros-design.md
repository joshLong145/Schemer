# Compile-Time Macros Design Addendum

## Overview

This document extends the QBE backend architecture with Rust compile-time macros to eliminate redundancy and improve maintainability.

---

## 1. Runtime FFI Export Macro

### Problem
Every runtime function needs repetitive boilerplate:
```rust
#[no_mangle]
pub extern "C" fn rt_cons(car: SchemeValue, cdr: SchemeValue) -> SchemeValue {
    // ...
}
```

### Solution: `define_rt_fn!`

```rust
// runtime/src/macros.rs

/// Define a runtime function with automatic FFI export
/// 
/// Usage:
///   define_rt_fn!(cons(car: Value, cdr: Value) -> Value { ... });
///   define_rt_fn!(newline() { ... });  // void return
#[macro_export]
macro_rules! define_rt_fn {
    // With return type
    ($name:ident($($arg:ident : $arg_ty:ty),* $(,)?) -> $ret:ty $body:block) => {
        #[no_mangle]
        #[inline]
        pub extern "C" fn $name($($arg: $arg_ty),*) -> $ret $body
    };
    
    // Void return (no -> type)
    ($name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $body:block) => {
        #[no_mangle]
        #[inline]
        pub extern "C" fn $name($($arg: $arg_ty),*) $body
    };
}

/// Batch-define multiple runtime functions
#[macro_export]
macro_rules! define_rt_fns {
    ($(fn $name:ident($($arg:ident : $arg_ty:ty),* $(,)?) $(-> $ret:ty)? $body:block)+) => {
        $(
            define_rt_fn!($name($($arg: $arg_ty),*) $(-> $ret)? $body);
        )+
    };
}
```

### Usage

```rust
// runtime/src/primitives.rs

use crate::{define_rt_fn, define_rt_fns};

define_rt_fns! {
    fn rt_cons(car: SchemeValue, cdr: SchemeValue) -> SchemeValue {
        let pair = alloc_pair();
        unsafe {
            (*pair).car = car;
            (*pair).cdr = cdr;
        }
        rt_incref(car);
        rt_incref(cdr);
        SchemeValue::from_ptr(pair as *mut HeapObject)
    }
    
    fn rt_car(pair: SchemeValue) -> SchemeValue {
        with_pair!(pair, p => {
            let car = (*p).car;
            rt_incref(car);
            car
        })
    }
    
    fn rt_cdr(pair: SchemeValue) -> SchemeValue {
        with_pair!(pair, p => {
            let cdr = (*p).cdr;
            rt_incref(cdr);
            cdr
        })
    }
    
    fn rt_display(val: SchemeValue) {
        print!("{}", format_value(val));
    }
    
    fn rt_newline() {
        println!();
    }
}
```

---

## 2. Type Check Macros

### Problem
Repetitive type checking with unsafe pointer casts:
```rust
if !pair.is_pointer() {
    rt_type_error(b"pair\0".as_ptr() as *const i8, pair);
}
unsafe {
    let obj = pair.as_ptr();
    if (*obj).type_tag != ObjectType::Pair {
        rt_type_error(b"pair\0".as_ptr() as *const i8, pair);
    }
    let p = obj as *const Pair;
    // use p...
}
```

### Solution: `with_typed!` and variants

```rust
// runtime/src/macros.rs

/// Type-check and cast a SchemeValue, calling body with the typed pointer
/// Automatically handles error on type mismatch
#[macro_export]
macro_rules! with_typed {
    ($val:expr, $type_tag:expr, $rust_ty:ty, $name:ident => $body:expr) => {{
        let __val = $val;
        if !__val.is_pointer() {
            $crate::exception::rt_type_error_static(
                stringify!($rust_ty),
                __val
            );
        }
        unsafe {
            let __obj = __val.as_ptr();
            if (*__obj).type_tag != $type_tag {
                $crate::exception::rt_type_error_static(
                    stringify!($rust_ty),
                    __val
                );
            }
            let $name = __obj as *mut $rust_ty;
            $body
        }
    }};
}

/// Convenience macros for common types
#[macro_export]
macro_rules! with_pair {
    ($val:expr, $name:ident => $body:expr) => {
        $crate::with_typed!($val, ObjectType::Pair, Pair, $name => $body)
    };
}

#[macro_export]
macro_rules! with_closure {
    ($val:expr, $name:ident => $body:expr) => {
        $crate::with_typed!($val, ObjectType::Closure, Closure, $name => $body)
    };
}

#[macro_export]
macro_rules! with_box {
    ($val:expr, $name:ident => $body:expr) => {
        $crate::with_typed!($val, ObjectType::Box, BoxCell, $name => $body)
    };
}

#[macro_export]
macro_rules! with_string {
    ($val:expr, $name:ident => $body:expr) => {
        $crate::with_typed!($val, ObjectType::String, SchemeString, $name => $body)
    };
}

/// Check if value matches type, return boolean SchemeValue
#[macro_export]
macro_rules! is_type {
    ($val:expr, $type_tag:expr) => {{
        let __val = $val;
        if __val.is_pointer() {
            unsafe {
                let __obj = __val.as_ptr();
                if (*__obj).type_tag == $type_tag {
                    SchemeValue::TRUE
                } else {
                    SchemeValue::FALSE
                }
            }
        } else {
            SchemeValue::FALSE
        }
    }};
}
```

### Usage

```rust
define_rt_fn!(rt_car(pair: SchemeValue) -> SchemeValue {
    with_pair!(pair, p => {
        let car = unsafe { (*p).car };
        rt_incref(car);
        car
    })
});

define_rt_fn!(rt_is_pair(val: SchemeValue) -> SchemeValue {
    is_type!(val, ObjectType::Pair)
});
```

---

## 3. Primitive Operations Macro

### Problem
Codegen has repetitive patterns for each primitive:
```rust
PrimOp::Add => { /* unbox, add, rebox */ }
PrimOp::Sub => { /* unbox, sub, rebox */ }
PrimOp::Mul => { /* unbox, mul, rebox */ }
// ... many more
```

### Solution: `define_primitives!`

```rust
// src/compiler/primitives.rs

/// Define primitive operations with their codegen patterns
#[macro_export]
macro_rules! define_primitives {
    (
        $(
            $variant:ident {
                arity: $arity:expr,
                $(inline: $inline:expr,)?
                codegen: |$gen:ident, $args:ident| $codegen:expr
            }
        ),* $(,)?
    ) => {
        /// Primitive operations enum
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub enum PrimOp {
            $($variant),*
        }
        
        impl PrimOp {
            /// Get arity of primitive
            pub fn arity(&self) -> usize {
                match self {
                    $(PrimOp::$variant => $arity),*
                }
            }
            
            /// Whether this primitive should be inlined
            pub fn should_inline(&self) -> bool {
                match self {
                    $(PrimOp::$variant => define_primitives!(@inline $($inline)?)),*
                }
            }
            
            /// Runtime function name (if not inlined)
            pub fn runtime_name(&self) -> &'static str {
                match self {
                    $(PrimOp::$variant => concat!("rt_", stringify!($variant)).to_lowercase()),*
                }
            }
        }
        
        impl CodeGenerator {
            /// Generate code for a primitive operation
            pub fn generate_prim_op(&mut self, op: &PrimOp, args: &[Atom]) -> QbeValue {
                match op {
                    $(
                        PrimOp::$variant => {
                            let $gen = self;
                            let $args = args;
                            $codegen
                        }
                    ),*
                }
            }
        }
    };
    
    // Helper for optional inline flag
    (@inline) => { false };
    (@inline $inline:expr) => { $inline };
}
```

### Usage

```rust
// src/compiler/primitives.rs

define_primitives! {
    // Arithmetic - inline for performance
    Add {
        arity: 2,
        inline: true,
        codegen: |gen, args| {
            let a = gen.unbox_fixnum(&args[0]);
            let b = gen.unbox_fixnum(&args[1]);
            let result = gen.emit_binop(QbeOp::Add, a, b);
            gen.box_fixnum(&result)
        }
    },
    
    Sub {
        arity: 2,
        inline: true,
        codegen: |gen, args| {
            let a = gen.unbox_fixnum(&args[0]);
            let b = gen.unbox_fixnum(&args[1]);
            let result = gen.emit_binop(QbeOp::Sub, a, b);
            gen.box_fixnum(&result)
        }
    },
    
    Mul {
        arity: 2,
        inline: true,
        codegen: |gen, args| {
            let a = gen.unbox_fixnum(&args[0]);
            let b = gen.unbox_fixnum(&args[1]);
            let result = gen.emit_binop(QbeOp::Mul, a, b);
            gen.box_fixnum(&result)
        }
    },
    
    // Comparison
    Eq {
        arity: 2,
        inline: false,  // Use runtime for full equality
        codegen: |gen, args| {
            gen.emit_runtime_call("rt_eq", args)
        }
    },
    
    Lt {
        arity: 2,
        inline: true,
        codegen: |gen, args| {
            let a = gen.unbox_fixnum(&args[0]);
            let b = gen.unbox_fixnum(&args[1]);
            gen.emit_comparison(QbeOp::Csltl, a, b)
        }
    },
    
    // List operations - always runtime calls
    Cons {
        arity: 2,
        codegen: |gen, args| gen.emit_runtime_call("rt_cons", args)
    },
    
    Car {
        arity: 1,
        codegen: |gen, args| gen.emit_runtime_call("rt_car", args)
    },
    
    Cdr {
        arity: 1,
        codegen: |gen, args| gen.emit_runtime_call("rt_cdr", args)
    },
    
    // Type predicates - can inline the check
    IsNull {
        arity: 1,
        inline: true,
        codegen: |gen, args| {
            let val = gen.atom_to_value(&args[0]);
            gen.emit_eq_check(val, QbeValue::Const(NIL_TAG))
        }
    },
    
    IsPair {
        arity: 1,
        inline: true,
        codegen: |gen, args| {
            gen.emit_type_check(&args[0], ObjectType::Pair)
        }
    },
    
    // I/O - always runtime
    Display {
        arity: 1,
        codegen: |gen, args| {
            gen.emit_runtime_call("rt_display", args);
            QbeValue::Const(VOID_TAG)
        }
    },
    
    Newline {
        arity: 0,
        codegen: |gen, _args| {
            gen.emit_runtime_call_no_args("rt_newline");
            QbeValue::Const(VOID_TAG)
        }
    },
}
```

---

## 4. QBE IR Builder Macros

### Problem
QBE instruction emission is verbose:
```rust
self.emit(QbeInst::Assign {
    dest: dest.clone(),
    ty: QbeType::Long,
    op: QbeOp::Add(a, b),
});
```

### Solution: `qbe!` DSL macro

```rust
// src/compiler/qbe_macros.rs

/// QBE instruction builder DSL
#[macro_export]
macro_rules! qbe {
    // Assignment: %dest =l add %a, %b
    ($gen:expr; $dest:ident = $op:ident($($arg:expr),* $(,)?)) => {{
        let dest_name = $gen.fresh_temp();
        $gen.emit(QbeInst::Assign {
            dest: dest_name.clone(),
            ty: QbeType::Long,
            op: qbe!(@op $op($($arg),*)),
        });
        let $dest = QbeValue::Temp(dest_name);
    }};
    
    // Call with result: %dest =l call $func(args...)
    ($gen:expr; $dest:ident = call $func:expr, $($arg:expr),* $(,)?) => {{
        let dest_name = $gen.fresh_temp();
        $gen.emit(QbeInst::Call {
            dest: Some(dest_name.clone()),
            func: $func,
            args: vec![$(($crate::compiler::qbe::QbeType::Long, $arg)),*],
        });
        let $dest = QbeValue::Temp(dest_name);
    }};
    
    // Call without result: call $func(args...)
    ($gen:expr; call $func:expr, $($arg:expr),* $(,)?) => {{
        $gen.emit(QbeInst::Call {
            dest: None,
            func: $func,
            args: vec![$(($crate::compiler::qbe::QbeType::Long, $arg)),*],
        });
    }};
    
    // Store: storel %val, %addr
    ($gen:expr; store $val:expr, $addr:expr) => {{
        $gen.emit(QbeInst::Store {
            ty: QbeType::Long,
            value: $val,
            addr: $addr,
        });
    }};
    
    // Load: %dest =l loadl %addr
    ($gen:expr; $dest:ident = load $addr:expr) => {{
        let dest_name = $gen.fresh_temp();
        $gen.emit(QbeInst::Load {
            dest: dest_name.clone(),
            ty: QbeType::Long,
            addr: $addr,
        });
        let $dest = QbeValue::Temp(dest_name);
    }};
    
    // Jump: jmp @label
    ($gen:expr; jmp $label:expr) => {{
        $gen.emit(QbeInst::Jmp($label.to_string()));
    }};
    
    // Conditional jump: jnz %cond, @true, @false
    ($gen:expr; jnz $cond:expr, $if_true:expr, $if_false:expr) => {{
        $gen.emit(QbeInst::Jnz {
            cond: $cond,
            if_true: $if_true.to_string(),
            if_false: $if_false.to_string(),
        });
    }};
    
    // Return: ret %val
    ($gen:expr; ret $val:expr) => {{
        $gen.emit(QbeInst::Ret(Some($val)));
    }};
    
    // Operation helpers
    (@op add($a:expr, $b:expr)) => { QbeOp::Add($a, $b) };
    (@op sub($a:expr, $b:expr)) => { QbeOp::Sub($a, $b) };
    (@op mul($a:expr, $b:expr)) => { QbeOp::Mul($a, $b) };
    (@op div($a:expr, $b:expr)) => { QbeOp::Div($a, $b) };
    (@op and($a:expr, $b:expr)) => { QbeOp::And($a, $b) };
    (@op or($a:expr, $b:expr)) => { QbeOp::Or($a, $b) };
    (@op xor($a:expr, $b:expr)) => { QbeOp::Xor($a, $b) };
    (@op sar($a:expr, $b:expr)) => { QbeOp::Sar($a, $b) };
    (@op shl($a:expr, $b:expr)) => { QbeOp::Shl($a, $b) };
    (@op ceql($a:expr, $b:expr)) => { QbeOp::Ceql($a, $b) };
    (@op csltl($a:expr, $b:expr)) => { QbeOp::Csltl($a, $b) };
    (@op copy($a:expr)) => { QbeOp::Copy($a) };
}

/// Multiple QBE instructions in sequence
#[macro_export]
macro_rules! qbe_block {
    ($gen:expr; $($inst:tt);+ $(;)?) => {{
        $(qbe!($gen; $inst);)+
    }};
}
```

### Usage

```rust
// Before (verbose)
fn generate_add(&mut self, args: &[Atom]) -> QbeValue {
    let a = self.atom_to_value(&args[0]);
    let b = self.atom_to_value(&args[1]);
    
    let unboxed_a = self.fresh_temp();
    self.emit(QbeInst::Assign {
        dest: unboxed_a.clone(),
        ty: QbeType::Long,
        op: QbeOp::Sar(a, QbeValue::Const(3)),
    });
    
    let unboxed_b = self.fresh_temp();
    self.emit(QbeInst::Assign {
        dest: unboxed_b.clone(),
        ty: QbeType::Long,
        op: QbeOp::Sar(b, QbeValue::Const(3)),
    });
    
    let sum = self.fresh_temp();
    self.emit(QbeInst::Assign {
        dest: sum.clone(),
        ty: QbeType::Long,
        op: QbeOp::Add(QbeValue::Temp(unboxed_a), QbeValue::Temp(unboxed_b)),
    });
    
    // ... box result
}

// After (concise)
fn generate_add(&mut self, args: &[Atom]) -> QbeValue {
    let a = self.atom_to_value(&args[0]);
    let b = self.atom_to_value(&args[1]);
    
    qbe!(self; unboxed_a = sar(a, QbeValue::Const(3)));
    qbe!(self; unboxed_b = sar(b, QbeValue::Const(3)));
    qbe!(self; sum = add(unboxed_a, unboxed_b));
    qbe!(self; shifted = shl(sum, QbeValue::Const(3)));
    qbe!(self; tagged = or(shifted, QbeValue::Const(1)));
    
    tagged
}
```

---

## 5. ANF Construction Macros

### Problem
Building ANF IR is verbose with nested Let expressions.

### Solution: `anf!` macro

```rust
// src/compiler/anf_macros.rs

/// Build ANF expressions declaratively
#[macro_export]
macro_rules! anf {
    // Return atom
    (return $atom:expr) => {
        AnfExpr::Return($atom)
    };
    
    // Let binding
    (let $var:ident = $complex:expr; $($rest:tt)+) => {
        AnfExpr::Let {
            var: VarId::new(stringify!($var)),
            value: $complex,
            body: Box::new(anf!($($rest)+)),
        }
    };
    
    // Sequence (side effect)
    (seq $effect:expr; $($rest:tt)+) => {
        AnfExpr::Seq {
            effect: $effect,
            body: Box::new(anf!($($rest)+)),
        }
    };
    
    // Tail call
    (tailcall $func:expr, [$($arg:expr),* $(,)?]) => {
        AnfExpr::TailCall {
            func: $func,
            args: vec![$($arg),*],
        }
    };
    
    // If expression
    (if $cond:expr => $then:tt else $else:tt) => {
        ComplexExpr::If {
            cond: $cond,
            then_expr: Box::new(anf!$then),
            else_expr: Box::new(anf!$else),
        }
    };
}

/// Build atoms
#[macro_export]
macro_rules! atom {
    (var $name:ident) => { Atom::Var(VarId::new(stringify!($name))) };
    (int $n:expr) => { Atom::Int($n) };
    (bool $b:expr) => { Atom::Bool($b) };
    (nil) => { Atom::Nil };
    (void) => { Atom::Void };
}

/// Build complex expressions  
#[macro_export]
macro_rules! complex {
    (prim $op:ident($($arg:expr),*)) => {
        ComplexExpr::PrimApp {
            op: PrimOp::$op,
            args: vec![$($arg),*],
        }
    };
    
    (app $func:expr, [$($arg:expr),*]) => {
        ComplexExpr::App {
            func: $func,
            args: vec![$($arg),*],
        }
    };
    
    (closure $label:expr, [$($cap:expr),*]) => {
        ComplexExpr::MakeClosure {
            label: $label.to_string(),
            captures: vec![$($cap),*],
        }
    };
}
```

### Usage

```rust
// Building factorial body:
let body = anf! {
    let t0 = complex!(prim Eq(atom!(var n), atom!(int 0)));
    let result = if atom!(var t0) => {
        return atom!(int 1)
    } else {
        let t1 = complex!(prim Sub(atom!(var n), atom!(int 1)));
        let t2 = complex!(app atom!(var factorial), [atom!(var t1)]);
        tailcall atom!(var mul), [atom!(var n), atom!(var t2)]
    };
    return atom!(var result)
};
```

---

## 6. GC Child Iteration Macro

### Problem
`for_each_child` has repetitive match arms for each object type.

### Solution: `define_heap_types!`

```rust
// runtime/src/heap_types.rs

/// Define heap object types with automatic child iteration
#[macro_export]
macro_rules! define_heap_types {
    (
        $(
            $name:ident ($type_tag:expr) {
                $(fields: { $($field:ident : $field_ty:ty),* $(,)? })?
                $(children: [$($child:ident),* $(,)?])?
                $(variable_children: $var_child_fn:expr)?
            }
        ),* $(,)?
    ) => {
        // Generate struct definitions
        $(
            #[repr(C)]
            pub struct $name {
                pub header: HeapObject,
                $($($field: $field_ty,)*)?
            }
        )*
        
        impl GarbageCollector {
            /// Iterate over all child values of a heap object
            pub fn for_each_child<F>(&mut self, obj: *mut HeapObject, mut f: F)
            where
                F: FnMut(&mut Self, *mut HeapObject)
            {
                unsafe {
                    match (*obj).type_tag {
                        $(
                            $type_tag => {
                                let typed = obj as *mut $name;
                                
                                // Static children
                                $($(
                                    let child_val = (*typed).$child;
                                    if child_val.is_pointer() {
                                        f(self, child_val.as_ptr());
                                    }
                                )*)?
                                
                                // Variable children (e.g., closure captures)
                                $(
                                    for child_val in $var_child_fn(typed) {
                                        if child_val.is_pointer() {
                                            f(self, child_val.as_ptr());
                                        }
                                    }
                                )?
                            }
                        )*
                        _ => {}
                    }
                }
            }
            
            /// Calculate object size
            pub fn object_size(&self, obj: *mut HeapObject) -> usize {
                unsafe {
                    match (*obj).type_tag {
                        $(
                            $type_tag => {
                                define_heap_types!(@size $name $(, $var_child_fn, obj)?)
                            }
                        )*
                        _ => std::mem::size_of::<HeapObject>()
                    }
                }
            }
        }
    };
    
    // Size calculation helpers
    (@size $name:ident) => {
        std::mem::size_of::<$name>()
    };
    
    (@size $name:ident, $var_fn:expr, $obj:expr) => {{
        let typed = $obj as *const $name;
        std::mem::size_of::<$name>() + $var_fn(typed).len() * 8
    }};
}
```

### Usage

```rust
define_heap_types! {
    Pair (ObjectType::Pair) {
        fields: { car: SchemeValue, cdr: SchemeValue }
        children: [car, cdr]
    },
    
    BoxCell (ObjectType::Box) {
        fields: { value: SchemeValue }
        children: [value]
    },
    
    Closure (ObjectType::Closure) {
        fields: { 
            func_ptr: *const (), 
            num_captures: usize 
        }
        variable_children: |c: *const Closure| unsafe { (*c).captures() }
    },
    
    BoxedFloat (ObjectType::Float) {
        fields: { value: f64 }
        // No children
    },
    
    SchemeString (ObjectType::String) {
        fields: { len: usize }
        // No children, variable size handled separately
    },
    
    Bounce (ObjectType::Bounce) {
        fields: {
            func_ptr: *const (),
            args: *mut SchemeValue,
            argc: usize
        }
        // Args are handled specially by trampoline
    },
}
```

---

## 7. Symbol/Keyword Recognition Macro

### Problem
ANF transformer needs to recognize special forms:

```rust
match &elements[0] {
    Value::Symbol(s) if s == "quote" => ...
    Value::Symbol(s) if s == "if" => ...
    Value::Symbol(s) if s == "lambda" => ...
    // ... many more
}
```

### Solution: `match_special_form!`

```rust
// src/compiler/anf_macros.rs

/// Match special forms by symbol name
#[macro_export]
macro_rules! match_special_form {
    ($expr:expr, $elements:expr, $tail_pos:expr;
        $($form:ident => $handler:expr),*
        $(,)?
        ; else => $default:expr
    ) => {
        match $expr {
            $(
                Value::Symbol(s) if s == stringify!($form) => {
                    $handler(&$elements[1..], $tail_pos)
                }
            )*
            _ => $default
        }
    };
}
```

### Usage

```rust
fn transform_list(&mut self, expr: &Value, tail_position: bool) -> AnfExpr {
    let elements = self.list_to_vec(expr);
    if elements.is_empty() {
        return AnfExpr::Return(Atom::Nil);
    }
    
    match_special_form!(&elements[0], elements, tail_position;
        quote => self.transform_quote,
        if => self.transform_if,
        lambda => self.transform_lambda,
        let => self.transform_let,
        begin => self.transform_begin,
        define => self.transform_define,
        set! => self.transform_set,
        and => self.transform_and,
        or => self.transform_or,
        cond => self.transform_cond;
        else => self.transform_application(&elements, tail_position)
    )
}
```

---

## 8. Tag Constants Macro

### Problem
Tag constants are defined in multiple places and need to stay in sync.

### Solution: `define_tags!`

```rust
// shared between compiler and runtime
// src/tags.rs (or runtime/src/tags.rs)

#[macro_export]
macro_rules! define_tags {
    (
        tag_bits: $bits:expr;
        immediates: {
            $($imm_name:ident = $imm_tag:expr),* $(,)?
        }
        specials: {
            $($spec_name:ident = $spec_id:expr),* $(,)?
        }
        heap_types: {
            $($heap_name:ident = $heap_id:expr),* $(,)?
        }
    ) => {
        pub const TAG_BITS: u64 = $bits;
        pub const TAG_MASK: u64 = (1 << TAG_BITS) - 1;
        
        // Immediate type tags
        $(pub const $imm_name: u64 = $imm_tag;)*
        
        // Special value constants (tag = SPECIAL_TAG)
        const SPECIAL_TAG: u64 = 0b011;
        $(pub const $spec_name: u64 = ($spec_id << TAG_BITS) | SPECIAL_TAG;)*
        
        // Heap object type tags
        #[repr(u8)]
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        pub enum ObjectType {
            $($heap_name = $heap_id),*
        }
    };
}

// Usage:
define_tags! {
    tag_bits: 3;
    
    immediates: {
        POINTER_TAG = 0b000,
        FIXNUM_TAG = 0b001,
        CHAR_TAG = 0b010,
        SPECIAL_TAG = 0b011,
    }
    
    specials: {
        NIL_VALUE = 0,
        TRUE_VALUE = 1,
        FALSE_VALUE = 2,
        VOID_VALUE = 3,
        EOF_VALUE = 4,
    }
    
    heap_types: {
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
}
```

---

## 9. Complete Macro Module Structure

```
src/
├── macros.rs                 # Existing lisp! macro
├── compiler/
│   ├── macros.rs             # Compiler macros
│   │   ├── anf!              # ANF construction
│   │   ├── atom!             # Atom construction
│   │   ├── complex!          # ComplexExpr construction
│   │   ├── match_special_form!
│   │   └── define_primitives!
│   ├── qbe_macros.rs         # QBE IR macros
│   │   ├── qbe!              # Single instruction
│   │   └── qbe_block!        # Multiple instructions
│   └── ...
└── runtime/
    └── src/
        ├── macros.rs         # Runtime macros
        │   ├── define_rt_fn! 
        │   ├── define_rt_fns!
        │   ├── with_typed!
        │   ├── with_pair!, with_closure!, etc.
        │   ├── is_type!
        │   └── define_heap_types!
        ├── tags.rs           # Shared tag definitions
        │   └── define_tags!
        └── ...
```

---

## 10. Benefits Summary

| Macro | Lines Saved | Benefit |
|-------|-------------|---------|
| `define_rt_fn!` | ~3 per function | Consistent FFI export |
| `with_typed!` | ~8 per type check | Safe, readable type access |
| `define_primitives!` | ~50+ | Single source of truth for primitives |
| `qbe!` | ~5 per instruction | Readable IR generation |
| `anf!` | ~3 per expression | Declarative ANF construction |
| `define_heap_types!` | ~40+ | Auto-generate child iteration |
| `define_tags!` | ~20 | Sync tags between compiler/runtime |

**Total estimated reduction**: 30-40% less boilerplate code

---

## Approval

Does this macro design meet your requirements for reducing redundancy? Ready to incorporate into the main architecture document and proceed to `/sc:workflow`?
