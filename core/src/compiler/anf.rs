//! A-Normal Form (ANF) Intermediate Representation
//!
//! ANF ensures all intermediate values are explicitly named and all arguments
//! to operations are atomic (variables or constants). This simplifies code generation.

use std::collections::HashSet;

use crate::types::Value;

/// Unique identifier for variables/temporaries
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VarId(pub String);

impl VarId {
    pub fn new(name: &str) -> Self {
        VarId(name.to_string())
    }

    pub fn temp(id: u64) -> Self {
        VarId(format!("_t{}", id))
    }

    pub fn is_temp(&self) -> bool {
        self.0.starts_with("_t")
    }

    pub fn name(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for VarId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Atomic expressions - can be used directly as operands
#[derive(Clone, Debug)]
pub enum Atom {
    /// Variable reference
    Var(VarId),
    /// Integer literal (61-bit fixnum)
    Int(i64),
    /// Floating point literal
    Float(f64),
    /// Boolean literal
    Bool(bool),
    /// Character literal
    Char(char),
    /// String literal (index into string table)
    String(usize),
    /// Symbol literal (index into symbol table)
    Symbol(usize),
    /// Nil / empty list
    Nil,
    /// Void value
    Void,
}

/// Primitive operations
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PrimOp {
    // Special
    Identity, // Returns its argument unchanged (used for ANF normalization)
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    // Comparison
    NumEq,
    Lt,
    Gt,
    Le,
    Ge,
    // Type predicates
    IsNull,
    IsPair,
    IsNumber,
    IsBool,
    IsSymbol,
    IsString,
    IsProc,
    IsChar,
    // List operations
    Cons,
    Car,
    Cdr,
    SetCar,
    SetCdr,
    // Equality
    Eq,
    Eqv,
    // I/O
    Display,
    Newline,
    // Logic
    Not,
    // List construction
    List,
}

/// Complex expressions - must be bound to a variable
#[derive(Clone, Debug)]
pub enum ComplexExpr {
    /// Primitive operation: (op arg1 arg2 ...)
    PrimApp { op: PrimOp, args: Vec<Atom> },

    /// Function application (non-tail): result = func(args...)
    App { func: Atom, args: Vec<Atom> },

    /// Tail call application
    TailApp { func: Atom, args: Vec<Atom> },

    /// Lambda/closure creation (after closure conversion, captures explicit)
    MakeClosure {
        /// Unique function label
        label: String,
        /// Free variables to capture
        captures: Vec<VarId>,
    },

    /// Read from closure environment
    ClosureRef {
        /// Closure variable
        closure: VarId,
        /// Index into captures
        index: usize,
    },

    /// Conditional expression
    If {
        cond: Atom,
        then_expr: Box<AnfExpr>,
        else_expr: Box<AnfExpr>,
    },

    /// Create mutable box
    MakeBox(Atom),

    /// Read from mutable box
    ReadBox(VarId),

    /// Write to mutable box
    WriteBox { box_var: VarId, value: Atom },
}

/// ANF expressions
#[derive(Clone, Debug)]
pub enum AnfExpr {
    /// Return an atomic value
    Return(Atom),

    /// Let binding: let var = complex_expr in body
    Let {
        var: VarId,
        value: ComplexExpr,
        body: Box<AnfExpr>,
    },

    /// Sequential execution (for side effects)
    Seq {
        /// Expression to execute (result discarded)
        effect: ComplexExpr,
        /// Continuation
        body: Box<AnfExpr>,
    },

    /// Tail call (terminal - doesn't return normally)
    TailCall { func: Atom, args: Vec<Atom> },

    /// Halt/error (terminal)
    Halt(Atom),
}

/// Top-level function definition (after closure conversion)
#[derive(Clone, Debug)]
pub struct FunctionDef {
    /// Unique function label/name
    pub label: String,
    /// Original Scheme name (for backtraces)
    pub source_name: Option<String>,
    /// Parameter names
    pub params: Vec<VarId>,
    /// Whether this function receives a closure environment
    pub has_env: bool,
    /// Function body
    pub body: AnfExpr,
    /// Set of free variables (before closure conversion)
    pub free_vars: HashSet<VarId>,
}

/// Complete ANF program
#[derive(Clone, Debug)]
pub struct AnfProgram {
    /// All function definitions
    pub functions: Vec<FunctionDef>,
    /// Entry point expression (main)
    pub entry: AnfExpr,
    /// String literals (interned, indexed)
    pub strings: Vec<String>,
    /// Symbol literals (interned, indexed)
    pub symbols: Vec<String>,
}

/// ANF Transformer - converts AST to ANF
pub struct AnfTransformer {
    temp_counter: u64,
    label_counter: u64,
    functions: Vec<FunctionDef>,
    strings: Vec<String>,
    symbols: Vec<String>,
    /// Map from string to index for interning
    string_map: std::collections::HashMap<String, usize>,
    symbol_map: std::collections::HashMap<String, usize>,
}

impl AnfTransformer {
    pub fn new() -> Self {
        Self {
            temp_counter: 0,
            label_counter: 0,
            functions: Vec::new(),
            strings: Vec::new(),
            symbols: Vec::new(),
            string_map: std::collections::HashMap::new(),
            symbol_map: std::collections::HashMap::new(),
        }
    }

    fn fresh_temp(&mut self) -> VarId {
        let id = self.temp_counter;
        self.temp_counter += 1;
        VarId::temp(id)
    }

    fn fresh_label(&mut self, prefix: &str) -> String {
        let id = self.label_counter;
        self.label_counter += 1;
        format!("{}_{}", prefix, id)
    }

    fn intern_string(&mut self, s: &str) -> usize {
        if let Some(&idx) = self.string_map.get(s) {
            idx
        } else {
            let idx = self.strings.len();
            self.strings.push(s.to_string());
            self.string_map.insert(s.to_string(), idx);
            idx
        }
    }

    fn intern_symbol(&mut self, s: &str) -> usize {
        if let Some(&idx) = self.symbol_map.get(s) {
            idx
        } else {
            let idx = self.symbols.len();
            self.symbols.push(s.to_string());
            self.symbol_map.insert(s.to_string(), idx);
            idx
        }
    }

    /// Transform a program (list of top-level expressions)
    pub fn transform_program(&mut self, exprs: Vec<Value>) -> Result<AnfProgram, String> {
        // Wrap all top-level expressions in a begin
        let entry = self.transform_top_level(exprs)?;

        Ok(AnfProgram {
            functions: std::mem::take(&mut self.functions),
            entry,
            strings: std::mem::take(&mut self.strings),
            symbols: std::mem::take(&mut self.symbols),
        })
    }

    fn transform_top_level(&mut self, exprs: Vec<Value>) -> Result<AnfExpr, String> {
        if exprs.is_empty() {
            return Ok(AnfExpr::Return(Atom::Void));
        }

        // Process expressions in sequence
        let mut result = self.transform(&exprs[exprs.len() - 1], true)?;

        // Process in reverse, building up Seq expressions
        for expr in exprs[..exprs.len() - 1].iter().rev() {
            let anf = self.transform(expr, false)?;
            result = self.sequence(anf, result);
        }

        Ok(result)
    }

    /// Sequence two ANF expressions (first for effect, second for result)
    fn sequence(&self, first: AnfExpr, second: AnfExpr) -> AnfExpr {
        match first {
            AnfExpr::Return(Atom::Void) => second,
            AnfExpr::Return(_atom) => {
                // If first just returns a value, we can discard it
                // unless it has side effects (which Return(atom) doesn't)
                second
            }
            AnfExpr::Let { var, value, body } => AnfExpr::Let {
                var,
                value,
                body: Box::new(self.sequence(*body, second)),
            },
            AnfExpr::Seq { effect, body } => AnfExpr::Seq {
                effect,
                body: Box::new(self.sequence(*body, second)),
            },
            AnfExpr::TailCall { func, args } => {
                // TailCall shouldn't appear in begin's non-tail position normally,
                // but if it does, convert to regular App wrapped in Seq
                AnfExpr::Seq {
                    effect: ComplexExpr::App { func, args },
                    body: Box::new(second),
                }
            }
            AnfExpr::Halt(atom) => {
                // Halt is terminal
                AnfExpr::Halt(atom)
            }
        }
    }

    /// Transform an expression, tracking tail position
    pub fn transform(&mut self, expr: &Value, tail_pos: bool) -> Result<AnfExpr, String> {
        match expr {
            Value::Number(n) => Ok(AnfExpr::Return(self.number_to_atom(n))),
            Value::Boolean(b) => Ok(AnfExpr::Return(Atom::Bool(*b))),
            Value::Char(c) => Ok(AnfExpr::Return(Atom::Char(*c))),
            Value::Nil => Ok(AnfExpr::Return(Atom::Nil)),
            Value::Void => Ok(AnfExpr::Return(Atom::Void)),
            Value::String(s) => {
                let idx = self.intern_string(s);
                Ok(AnfExpr::Return(Atom::String(idx)))
            }
            Value::Symbol(s) => {
                // Symbols as expressions are variable references
                Ok(AnfExpr::Return(Atom::Var(VarId::new(s))))
            }
            Value::List(list) => {
                let elements = list.to_vec();
                self.transform_list(&elements, tail_pos)
            }
            Value::Pair(p) => {
                // Convert pair to list for processing
                let elements = self.pair_to_vec(p);
                self.transform_list(&elements, tail_pos)
            }
            _ => Err(format!("Cannot transform value: {:?}", expr)),
        }
    }

    fn number_to_atom(&self, n: &crate::types::value::Number) -> Atom {
        match n {
            crate::types::value::Number::Int(i) => Atom::Int(*i),
            crate::types::value::Number::Float(f) => Atom::Float(*f),
        }
    }

    fn pair_to_vec(&self, pair: &std::sync::Arc<(Value, Value)>) -> Vec<Value> {
        let mut result = vec![pair.0.clone()];
        let mut current = &pair.1;

        loop {
            match current {
                Value::Nil => break,
                Value::Pair(p) => {
                    result.push(p.0.clone());
                    current = &p.1;
                }
                Value::List(list) => {
                    result.extend(list.to_vec());
                    break;
                }
                other => {
                    result.push(other.clone());
                    break;
                }
            }
        }

        result
    }

    fn transform_list(&mut self, elements: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        if elements.is_empty() {
            return Ok(AnfExpr::Return(Atom::Nil));
        }

        // Check for special forms
        if let Value::Symbol(s) = &elements[0] {
            match s.as_str() {
                "quote" => return self.transform_quote(&elements[1..]),
                "if" => return self.transform_if(&elements[1..], tail_pos),
                "lambda" => return self.transform_lambda(&elements[1..]),
                "let" => return self.transform_let(&elements[1..], tail_pos),
                "let*" => return self.transform_let_star(&elements[1..], tail_pos),
                "letrec" => return self.transform_letrec(&elements[1..], tail_pos),
                "begin" => return self.transform_begin(&elements[1..], tail_pos),
                "define" => return self.transform_define(&elements[1..]),
                "set!" => return self.transform_set(&elements[1..]),
                "and" => return self.transform_and(&elements[1..], tail_pos),
                "or" => return self.transform_or(&elements[1..], tail_pos),
                "cond" => return self.transform_cond(&elements[1..], tail_pos),
                _ => {}
            }
        }

        // Regular application
        self.transform_application(elements, tail_pos)
    }

    fn transform_quote(&mut self, args: &[Value]) -> Result<AnfExpr, String> {
        if args.is_empty() {
            return Err("quote requires an argument".to_string());
        }
        self.quote_value(&args[0])
    }

    fn quote_value(&mut self, value: &Value) -> Result<AnfExpr, String> {
        match value {
            Value::Number(n) => Ok(AnfExpr::Return(self.number_to_atom(n))),
            Value::Boolean(b) => Ok(AnfExpr::Return(Atom::Bool(*b))),
            Value::Char(c) => Ok(AnfExpr::Return(Atom::Char(*c))),
            Value::Nil => Ok(AnfExpr::Return(Atom::Nil)),
            Value::String(s) => {
                let idx = self.intern_string(s);
                Ok(AnfExpr::Return(Atom::String(idx)))
            }
            Value::Symbol(s) => {
                let idx = self.intern_symbol(s);
                Ok(AnfExpr::Return(Atom::Symbol(idx)))
            }
            Value::List(list) => {
                // Build list from quoted elements
                self.quote_list(&list.to_vec())
            }
            Value::Pair(p) => {
                let elements = self.pair_to_vec(p);
                self.quote_list(&elements)
            }
            _ => Err(format!("Cannot quote: {:?}", value)),
        }
    }

    fn quote_list(&mut self, elements: &[Value]) -> Result<AnfExpr, String> {
        if elements.is_empty() {
            return Ok(AnfExpr::Return(Atom::Nil));
        }

        // Build list from the end: '(1 2 3) => cons(1, cons(2, cons(3, nil)))
        // For '(1 2): let t0 = 2 in let t1 = cons(t0, nil) in let t2 = 1 in let t3 = cons(t2, t1) in return t3
        //
        // We need to process right-to-left and build the ANF in proper order where
        // each variable is defined before it's used.
        
        // Phase 1: Collect all element ANFs and assign variables
        // Process right-to-left to match cons order
        struct ElemInfo {
            elem_anf: AnfExpr,
            elem_var: VarId,
            cons_var: VarId,
        }
        
        let mut infos: Vec<ElemInfo> = Vec::new();
        for elem in elements.iter().rev() {
            let elem_anf = self.quote_value(elem)?;
            let elem_var = self.fresh_temp();
            let cons_var = self.fresh_temp();
            infos.push(ElemInfo { elem_anf, elem_var, cons_var });
        }
        
        // Phase 2: Build the expression from inside out
        // The innermost element (rightmost, first in infos) conses with nil
        // Each subsequent element conses with the previous cons_var
        
        // Start with the return of the final (leftmost) list head
        let final_cons_var = infos.last().unwrap().cons_var.clone();
        let mut result = AnfExpr::Return(Atom::Var(final_cons_var));
        
        // Build from innermost (rightmost element) to outermost (leftmost element)
        // We iterate in reverse over infos (which gives us left-to-right in original list)
        for (i, info) in infos.iter().enumerate().rev() {
            let tail = if i == 0 {
                Atom::Nil // Rightmost element conses with nil
            } else {
                Atom::Var(infos[i - 1].cons_var.clone()) // Use previous cons result
            };
            
            // Build: let cons_var = cons(elem_var, tail) in <result>
            result = AnfExpr::Let {
                var: info.cons_var.clone(),
                value: ComplexExpr::PrimApp {
                    op: PrimOp::Cons,
                    args: vec![Atom::Var(info.elem_var.clone()), tail],
                },
                body: Box::new(result),
            };
            
            // Prepend the element computation
            result = self.prepend_anf_returning_to(info.elem_anf.clone(), info.elem_var.clone(), result);
        }
        
        Ok(result)
    }
    
    /// Helper: given an ANF expression that computes a value, make it store
    /// that value into `dest_var` and then continue with `continuation`
    fn prepend_anf_returning_to(&self, anf: AnfExpr, dest_var: VarId, continuation: AnfExpr) -> AnfExpr {
        match anf {
            AnfExpr::Return(atom) => {
                // Simple case: just bind the atom
                AnfExpr::Let {
                    var: dest_var,
                    value: ComplexExpr::PrimApp {
                        op: PrimOp::Identity,
                        args: vec![atom],
                    },
                    body: Box::new(continuation),
                }
            }
            AnfExpr::Let { var, value, body } => {
                // Recurse into the body, then wrap with this Let
                AnfExpr::Let {
                    var,
                    value,
                    body: Box::new(self.prepend_anf_returning_to(*body, dest_var, continuation)),
                }
            }
            AnfExpr::Seq { effect, body } => {
                AnfExpr::Seq {
                    effect,
                    body: Box::new(self.prepend_anf_returning_to(*body, dest_var, continuation)),
                }
            }
            // TailCall and Halt shouldn't appear in quote contexts
            _ => {
                // Fallback - shouldn't happen for quote
                AnfExpr::Seq {
                    effect: ComplexExpr::PrimApp {
                        op: PrimOp::Identity,
                        args: vec![Atom::Void],
                    },
                    body: Box::new(continuation),
                }
            }
        }
    }

    fn transform_if(&mut self, args: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        if args.len() < 2 {
            return Err("if requires at least 2 arguments".to_string());
        }

        let cond_expr = &args[0];
        let then_expr = &args[1];
        let else_expr = args.get(2).cloned().unwrap_or(Value::Void);

        // Normalize condition to atom
        let (cond_bindings, cond_atom) = self.normalize_to_atom(cond_expr)?;

        // Transform branches
        let then_anf = Box::new(self.transform(then_expr, tail_pos)?);
        let else_anf = Box::new(self.transform(&else_expr, tail_pos)?);

        // Build if expression
        let if_complex = ComplexExpr::If {
            cond: cond_atom,
            then_expr: then_anf,
            else_expr: else_anf,
        };

        // If in tail position, we don't need to bind result
        if tail_pos {
            // Wrap bindings around the if
            let result_var = self.fresh_temp();
            let mut result = AnfExpr::Let {
                var: result_var.clone(),
                value: if_complex,
                body: Box::new(AnfExpr::Return(Atom::Var(result_var))),
            };

            for (var, complex) in cond_bindings.into_iter().rev() {
                result = AnfExpr::Let {
                    var,
                    value: complex,
                    body: Box::new(result),
                };
            }

            Ok(result)
        } else {
            let result_var = self.fresh_temp();
            let mut result = AnfExpr::Let {
                var: result_var.clone(),
                value: if_complex,
                body: Box::new(AnfExpr::Return(Atom::Var(result_var))),
            };

            for (var, complex) in cond_bindings.into_iter().rev() {
                result = AnfExpr::Let {
                    var,
                    value: complex,
                    body: Box::new(result),
                };
            }

            Ok(result)
        }
    }

    fn transform_lambda(&mut self, args: &[Value]) -> Result<AnfExpr, String> {
        if args.len() < 2 {
            return Err("lambda requires parameters and body".to_string());
        }

        let params = self.extract_params(&args[0])?;
        let body = if args.len() == 2 {
            args[1].clone()
        } else {
            // Multiple body expressions - wrap in begin
            Value::List(std::sync::Arc::new(crate::types::SchemeList::from_vec(
                std::iter::once(Value::Symbol("begin".to_string()))
                    .chain(args[1..].iter().cloned())
                    .collect(),
            )))
        };

        // Transform body (in tail position)
        let body_anf = self.transform(&body, true)?;

        // Create function definition
        let label = self.fresh_label("lambda");
        let param_vars: Vec<VarId> = params.iter().map(|p| VarId::new(p)).collect();

        let func_def = FunctionDef {
            label: label.clone(),
            source_name: None,
            params: param_vars,
            has_env: false, // Will be set during closure conversion
            body: body_anf,
            free_vars: HashSet::new(), // Will be computed during closure conversion
        };

        self.functions.push(func_def);

        // Return a Let binding that creates the closure
        let closure_var = self.fresh_temp();
        Ok(AnfExpr::Let {
            var: closure_var.clone(),
            value: ComplexExpr::MakeClosure {
                label,
                captures: Vec::new(), // Will be filled during closure conversion
            },
            body: Box::new(AnfExpr::Return(Atom::Var(closure_var))),
        })
    }

    fn extract_params(&self, value: &Value) -> Result<Vec<String>, String> {
        match value {
            Value::Nil => Ok(vec![]),
            Value::List(list) => {
                let mut params = Vec::new();
                for item in list.to_vec() {
                    if let Value::Symbol(s) = item {
                        params.push(s.clone());
                    } else {
                        return Err(format!("Invalid parameter: {:?}", item));
                    }
                }
                Ok(params)
            }
            Value::Symbol(s) => {
                // Rest parameter (variadic)
                Ok(vec![s.clone()])
            }
            _ => Err(format!("Invalid parameter list: {:?}", value)),
        }
    }

    fn transform_let(&mut self, args: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        if args.len() < 2 {
            return Err("let requires bindings and body".to_string());
        }

        let bindings = self.extract_bindings(&args[0])?;
        let body = if args.len() == 2 {
            args[1].clone()
        } else {
            Value::List(std::sync::Arc::new(crate::types::SchemeList::from_vec(
                std::iter::once(Value::Symbol("begin".to_string()))
                    .chain(args[1..].iter().cloned())
                    .collect(),
            )))
        };

        // Transform body
        let mut result = self.transform(&body, tail_pos)?;

        // Add bindings in reverse order
        for (name, value) in bindings.into_iter().rev() {
            let (value_bindings, value_complex) = self.normalize_to_complex(&value)?;

            // Wrap value bindings
            result = AnfExpr::Let {
                var: VarId::new(&name),
                value: value_complex,
                body: Box::new(result),
            };

            for (var, complex) in value_bindings.into_iter().rev() {
                result = AnfExpr::Let {
                    var,
                    value: complex,
                    body: Box::new(result),
                };
            }
        }

        Ok(result)
    }

    fn transform_let_star(&mut self, args: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        // let* is sequential let - same implementation as let for ANF
        self.transform_let(args, tail_pos)
    }

    fn transform_letrec(&mut self, args: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        // letrec needs boxing for mutual recursion
        // For now, treat like let (works for simple cases)
        self.transform_let(args, tail_pos)
    }

    fn extract_bindings(&self, value: &Value) -> Result<Vec<(String, Value)>, String> {
        let list = match value {
            Value::Nil => return Ok(vec![]),
            Value::List(l) => l,
            _ => return Err(format!("Invalid bindings: {:?}", value)),
        };

        let mut bindings = Vec::new();
        for item in list.to_vec() {
            match item {
                Value::List(binding) => {
                    let binding_vec = binding.to_vec();
                    if binding_vec.len() != 2 {
                        return Err("Binding must have exactly 2 elements".to_string());
                    }
                    if let Value::Symbol(name) = &binding_vec[0] {
                        bindings.push((name.clone(), binding_vec[1].clone()));
                    } else {
                        return Err(format!("Invalid binding name: {:?}", binding_vec[0]));
                    }
                }
                _ => return Err(format!("Invalid binding: {:?}", item)),
            }
        }

        Ok(bindings)
    }

    fn transform_begin(&mut self, args: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        if args.is_empty() {
            return Ok(AnfExpr::Return(Atom::Void));
        }

        // Last expression is in tail position if we are
        let mut result = self.transform(&args[args.len() - 1], tail_pos)?;

        // Process earlier expressions for effect
        for expr in args[..args.len() - 1].iter().rev() {
            let effect_anf = self.transform(expr, false)?;
            result = self.sequence(effect_anf, result);
        }

        Ok(result)
    }

    fn transform_define(&mut self, args: &[Value]) -> Result<AnfExpr, String> {
        if args.is_empty() {
            return Err("define requires arguments".to_string());
        }

        match &args[0] {
            Value::Symbol(name) => {
                // Variable definition: (define x expr)
                if args.len() < 2 {
                    return Err("define requires a value".to_string());
                }
                let (bindings, complex) = self.normalize_to_complex(&args[1])?;

                let mut result = AnfExpr::Let {
                    var: VarId::new(name),
                    value: complex,
                    body: Box::new(AnfExpr::Return(Atom::Void)),
                };

                for (var, c) in bindings.into_iter().rev() {
                    result = AnfExpr::Let {
                        var,
                        value: c,
                        body: Box::new(result),
                    };
                }

                Ok(result)
            }
            Value::List(list) => {
                // Function definition: (define (f x y) body)
                let items = list.to_vec();
                if items.is_empty() {
                    return Err("Invalid define syntax".to_string());
                }

                let name = match &items[0] {
                    Value::Symbol(s) => s.clone(),
                    _ => return Err("Function name must be a symbol".to_string()),
                };

                let params: Vec<_> = items[1..].to_vec();
                let params_list = Value::List(std::sync::Arc::new(
                    crate::types::SchemeList::from_vec(params),
                ));

                // Build lambda
                let mut lambda_args = vec![params_list];
                lambda_args.extend(args[1..].iter().cloned());

                let lambda_anf = self.transform_lambda(&lambda_args)?;

                // Bind to name
                match lambda_anf {
                    AnfExpr::Return(atom) => Ok(AnfExpr::Let {
                        var: VarId::new(&name),
                        value: ComplexExpr::MakeClosure {
                            label: match &atom {
                                Atom::Var(v) => v.0.clone(),
                                _ => return Err("Expected lambda label".to_string()),
                            },
                            captures: vec![],
                        },
                        body: Box::new(AnfExpr::Return(Atom::Void)),
                    }),
                    other => Ok(other),
                }
            }
            _ => Err(format!("Invalid define syntax: {:?}", args[0])),
        }
    }

    fn transform_set(&mut self, args: &[Value]) -> Result<AnfExpr, String> {
        if args.len() != 2 {
            return Err("set! requires exactly 2 arguments".to_string());
        }

        let name = match &args[0] {
            Value::Symbol(s) => s.clone(),
            _ => return Err("set! target must be a symbol".to_string()),
        };

        let (bindings, value_atom) = self.normalize_to_atom(&args[1])?;

        // For now, treat set! as a write to a box
        // Full implementation requires tracking mutable variables
        let mut result = AnfExpr::Seq {
            effect: ComplexExpr::WriteBox {
                box_var: VarId::new(&name),
                value: value_atom,
            },
            body: Box::new(AnfExpr::Return(Atom::Void)),
        };

        for (var, complex) in bindings.into_iter().rev() {
            result = AnfExpr::Let {
                var,
                value: complex,
                body: Box::new(result),
            };
        }

        Ok(result)
    }

    fn transform_and(&mut self, args: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        if args.is_empty() {
            return Ok(AnfExpr::Return(Atom::Bool(true)));
        }

        if args.len() == 1 {
            return self.transform(&args[0], tail_pos);
        }

        // (and a b c) => (if a (and b c) #f)
        let rest = Value::List(std::sync::Arc::new(crate::types::SchemeList::from_vec(
            std::iter::once(Value::Symbol("and".to_string()))
                .chain(args[1..].iter().cloned())
                .collect(),
        )));

        let if_expr = vec![args[0].clone(), rest, Value::Boolean(false)];
        self.transform_if(&if_expr, tail_pos)
    }

    fn transform_or(&mut self, args: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        if args.is_empty() {
            return Ok(AnfExpr::Return(Atom::Bool(false)));
        }

        if args.len() == 1 {
            return self.transform(&args[0], tail_pos);
        }

        // (or a b c) => (let ((t a)) (if t t (or b c)))
        let temp = self.fresh_temp();
        let (bindings, first_atom) = self.normalize_to_atom(&args[0])?;

        let rest = Value::List(std::sync::Arc::new(crate::types::SchemeList::from_vec(
            std::iter::once(Value::Symbol("or".to_string()))
                .chain(args[1..].iter().cloned())
                .collect(),
        )));

        let rest_anf = self.transform(&rest, tail_pos)?;

        let if_complex = ComplexExpr::If {
            cond: Atom::Var(temp.clone()),
            then_expr: Box::new(AnfExpr::Return(Atom::Var(temp.clone()))),
            else_expr: Box::new(rest_anf),
        };

        let result_var = self.fresh_temp();
        let mut result = AnfExpr::Let {
            var: temp,
            value: ComplexExpr::PrimApp {
                op: PrimOp::Identity,
                args: vec![first_atom],
            },
            body: Box::new(AnfExpr::Let {
                var: result_var.clone(),
                value: if_complex,
                body: Box::new(AnfExpr::Return(Atom::Var(result_var))),
            }),
        };

        for (var, complex) in bindings.into_iter().rev() {
            result = AnfExpr::Let {
                var,
                value: complex,
                body: Box::new(result),
            };
        }

        Ok(result)
    }

    fn transform_cond(&mut self, args: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        if args.is_empty() {
            return Ok(AnfExpr::Return(Atom::Void));
        }

        // Convert cond to nested if
        self.cond_to_if(args, tail_pos)
    }

    fn cond_to_if(&mut self, clauses: &[Value], tail_pos: bool) -> Result<AnfExpr, String> {
        if clauses.is_empty() {
            return Ok(AnfExpr::Return(Atom::Void));
        }

        let clause = match &clauses[0] {
            Value::List(l) => l.to_vec(),
            _ => return Err("Invalid cond clause".to_string()),
        };

        if clause.is_empty() {
            return Err("Empty cond clause".to_string());
        }

        // Check for else clause
        if let Value::Symbol(s) = &clause[0] {
            if s == "else" {
                if clause.len() < 2 {
                    return Ok(AnfExpr::Return(Atom::Void));
                }
                // Handle multiple body expressions in else clause
                if clause.len() == 2 {
                    return self.transform(&clause[1], tail_pos);
                } else {
                    // Wrap multiple expressions in begin
                    let begin_body: Vec<Value> = std::iter::once(Value::Symbol("begin".to_string()))
                        .chain(clause[1..].iter().cloned())
                        .collect();
                    return self.transform(&Value::List(std::sync::Arc::new(
                        crate::types::SchemeList::from_vec(begin_body),
                    )), tail_pos);
                }
            }
        }

        let condition = &clause[0];
        // Handle multiple body expressions in clause
        let body = if clause.len() == 2 {
            clause[1].clone()
        } else if clause.len() > 2 {
            // Wrap multiple expressions in begin
            let begin_body: Vec<Value> = std::iter::once(Value::Symbol("begin".to_string()))
                .chain(clause[1..].iter().cloned())
                .collect();
            Value::List(std::sync::Arc::new(
                crate::types::SchemeList::from_vec(begin_body),
            ))
        } else {
            condition.clone() // Return condition value if no body
        };

        // Get the else branch (rest of cond clauses)
        let rest_anf = self.cond_to_if(&clauses[1..], tail_pos)?;

        // Transform condition to atom
        let (cond_bindings, cond_atom) = self.normalize_to_atom(condition)?;

        // Transform body (in tail position if we are)
        let then_anf = Box::new(self.transform(&body, tail_pos)?);

        // Build the If expression - use fresh temp to avoid variable shadowing
        let cond_result_var = self.fresh_temp();
        let if_expr = AnfExpr::Let {
            var: cond_result_var.clone(),
            value: ComplexExpr::If {
                cond: cond_atom,
                then_expr: then_anf,
                else_expr: Box::new(rest_anf),
            },
            body: Box::new(AnfExpr::Return(Atom::Var(cond_result_var))),
        };

        // Wrap with condition bindings
        let mut result = if_expr;
        for (var, complex) in cond_bindings.into_iter().rev() {
            result = AnfExpr::Let {
                var,
                value: complex,
                body: Box::new(result),
            };
        }

        Ok(result)
    }

    fn transform_application(
        &mut self,
        elements: &[Value],
        tail_pos: bool,
    ) -> Result<AnfExpr, String> {
        if elements.is_empty() {
            return Ok(AnfExpr::Return(Atom::Nil));
        }

        // Check if it's a primitive operation
        if let Value::Symbol(s) = &elements[0] {
            if let Some(prim_op) = self.get_primitive(s) {
                return self.transform_prim_app(prim_op, &elements[1..], tail_pos);
            }
        }

        // Regular function application
        let mut all_bindings = Vec::new();
        let mut args = Vec::new();

        for elem in elements {
            let (bindings, atom) = self.normalize_to_atom(elem)?;
            all_bindings.extend(bindings);
            args.push(atom);
        }

        let func = args.remove(0);

        let app = if tail_pos {
            ComplexExpr::TailApp { func, args }
        } else {
            ComplexExpr::App { func, args }
        };

        let result_var = self.fresh_temp();
        let mut result = AnfExpr::Let {
            var: result_var.clone(),
            value: app,
            body: Box::new(AnfExpr::Return(Atom::Var(result_var))),
        };

        for (var, complex) in all_bindings.into_iter().rev() {
            result = AnfExpr::Let {
                var,
                value: complex,
                body: Box::new(result),
            };
        }

        Ok(result)
    }

    fn transform_prim_app(
        &mut self,
        op: PrimOp,
        args: &[Value],
        _tail_pos: bool,
    ) -> Result<AnfExpr, String> {
        let mut all_bindings = Vec::new();
        let mut arg_atoms = Vec::new();

        for arg in args {
            let (bindings, atom) = self.normalize_to_atom(arg)?;
            all_bindings.extend(bindings);
            arg_atoms.push(atom);
        }

        let prim_complex = ComplexExpr::PrimApp {
            op,
            args: arg_atoms,
        };

        let result_var = self.fresh_temp();
        let mut result = AnfExpr::Let {
            var: result_var.clone(),
            value: prim_complex,
            body: Box::new(AnfExpr::Return(Atom::Var(result_var))),
        };

        for (var, complex) in all_bindings.into_iter().rev() {
            result = AnfExpr::Let {
                var,
                value: complex,
                body: Box::new(result),
            };
        }

        Ok(result)
    }

    fn get_primitive(&self, name: &str) -> Option<PrimOp> {
        match name {
            "+" => Some(PrimOp::Add),
            "-" => Some(PrimOp::Sub),
            "*" => Some(PrimOp::Mul),
            "/" => Some(PrimOp::Div),
            "modulo" | "remainder" => Some(PrimOp::Mod),
            "=" => Some(PrimOp::NumEq),
            "<" => Some(PrimOp::Lt),
            ">" => Some(PrimOp::Gt),
            "<=" => Some(PrimOp::Le),
            ">=" => Some(PrimOp::Ge),
            "null?" => Some(PrimOp::IsNull),
            "pair?" => Some(PrimOp::IsPair),
            "number?" => Some(PrimOp::IsNumber),
            "boolean?" => Some(PrimOp::IsBool),
            "symbol?" => Some(PrimOp::IsSymbol),
            "string?" => Some(PrimOp::IsString),
            "procedure?" => Some(PrimOp::IsProc),
            "char?" => Some(PrimOp::IsChar),
            "cons" => Some(PrimOp::Cons),
            "car" => Some(PrimOp::Car),
            "cdr" => Some(PrimOp::Cdr),
            "set-car!" => Some(PrimOp::SetCar),
            "set-cdr!" => Some(PrimOp::SetCdr),
            "eq?" => Some(PrimOp::Eq),
            "eqv?" => Some(PrimOp::Eqv),
            "display" => Some(PrimOp::Display),
            "newline" => Some(PrimOp::Newline),
            "not" => Some(PrimOp::Not),
            "list" => Some(PrimOp::List),
            _ => None,
        }
    }

    /// Normalize an expression to an atomic value
    /// Returns (let-bindings, atom)
    fn normalize_to_atom(
        &mut self,
        expr: &Value,
    ) -> Result<(Vec<(VarId, ComplexExpr)>, Atom), String> {
        match expr {
            // Already atomic
            Value::Number(n) => Ok((vec![], self.number_to_atom(n))),
            Value::Boolean(b) => Ok((vec![], Atom::Bool(*b))),
            Value::Char(c) => Ok((vec![], Atom::Char(*c))),
            Value::Nil => Ok((vec![], Atom::Nil)),
            Value::Void => Ok((vec![], Atom::Void)),
            Value::String(s) => {
                let idx = self.intern_string(s);
                Ok((vec![], Atom::String(idx)))
            }
            Value::Symbol(s) => Ok((vec![], Atom::Var(VarId::new(s)))),

            // Complex - needs let-binding
            _ => {
                let temp = self.fresh_temp();
                let (bindings, complex) = self.normalize_to_complex(expr)?;

                let mut all_bindings = bindings;
                all_bindings.push((temp.clone(), complex));

                Ok((all_bindings, Atom::Var(temp)))
            }
        }
    }

    /// Normalize an expression to a complex expression
    fn normalize_to_complex(
        &mut self,
        expr: &Value,
    ) -> Result<(Vec<(VarId, ComplexExpr)>, ComplexExpr), String> {
        let anf = self.transform(expr, false)?;
        self.anf_to_complex(anf)
    }

    /// Extract complex expression from ANF
    fn anf_to_complex(
        &self,
        anf: AnfExpr,
    ) -> Result<(Vec<(VarId, ComplexExpr)>, ComplexExpr), String> {
        match anf {
            AnfExpr::Return(atom) => {
                // Wrap atom in identity
                Ok((
                    vec![],
                    ComplexExpr::PrimApp {
                        op: PrimOp::Identity,
                        args: vec![atom],
                    },
                ))
            }
            AnfExpr::Let { var, value, body } => {
                let (mut bindings, final_complex) = self.anf_to_complex(*body)?;
                bindings.insert(0, (var, value));
                Ok((bindings, final_complex))
            }
            _ => Err("Cannot extract complex from this ANF form".to_string()),
        }
    }
}

impl Default for AnfTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::read_all;

    /// Helper function to parse Scheme source code into a Vec<Value>
    fn parse(s: &str) -> Vec<Value> {
        read_all(s).expect("Failed to parse")
    }

    /// Helper to transform and return the ANF program
    fn transform(s: &str) -> Result<AnfProgram, String> {
        let exprs = parse(s);
        let mut transformer = AnfTransformer::new();
        transformer.transform_program(exprs)
    }

    /// Helper to check if an expression contains a Let binding
    fn has_let_binding(expr: &AnfExpr) -> bool {
        matches!(expr, AnfExpr::Let { .. })
    }

    /// Helper to check if an expression contains a TailCall or TailApp
    fn has_tail_call(expr: &AnfExpr) -> bool {
        match expr {
            AnfExpr::TailCall { .. } => true,
            AnfExpr::Let { value, body, .. } => {
                matches!(value, ComplexExpr::TailApp { .. }) || has_tail_call(body)
            }
            AnfExpr::Seq { body, .. } => has_tail_call(body),
            _ => false,
        }
    }

    /// Helper to check if an expression contains an If
    fn has_if(expr: &AnfExpr) -> bool {
        match expr {
            AnfExpr::Let { value, body, .. } => {
                matches!(value, ComplexExpr::If { .. }) || has_if(body)
            }
            AnfExpr::Seq { body, .. } => has_if(body),
            _ => false,
        }
    }

    /// Helper to count Let bindings in an expression
    fn count_let_bindings(expr: &AnfExpr) -> usize {
        match expr {
            AnfExpr::Let { body, .. } => 1 + count_let_bindings(body),
            AnfExpr::Seq { body, .. } => count_let_bindings(body),
            _ => 0,
        }
    }

    #[test]
    fn test_simple_arithmetic() {
        // (+ 1 2) should transform to ANF with a let binding for the result
        let result = transform("(+ 1 2)").expect("Failed to transform simple arithmetic");

        // The entry point should contain a let binding for the primitive application
        // ANF normalizes (+ 1 2) to: let t0 = (+ 1 2) in return t0
        assert!(
            has_let_binding(&result.entry),
            "Simple arithmetic should produce a Let binding"
        );
    }

    #[test]
    fn test_nested_arithmetic() {
        // (+ (* 2 3) 4) should sequence operations properly
        // First compute (* 2 3), bind to temp, then compute (+ temp 4)
        let result = transform("(+ (* 2 3) 4)").expect("Failed to transform nested arithmetic");

        // Should have nested Let bindings
        assert!(
            has_let_binding(&result.entry),
            "Nested arithmetic should produce Let bindings"
        );

        // Check that we have at least two Let bindings (one for *, one for +)
        if let AnfExpr::Let { body, .. } = &result.entry {
            assert!(
                has_let_binding(body),
                "Nested arithmetic should have nested Let bindings"
            );
        } else {
            panic!("Expected Let binding at entry point");
        }
    }

    #[test]
    fn test_lambda() {
        // (lambda (x) x) should create a function definition
        let result = transform("(lambda (x) x)").expect("Failed to transform lambda");

        // Lambda creates a function and a closure at the entry point
        assert!(
            !result.functions.is_empty(),
            "Lambda should create a function definition"
        );

        // The function should have one parameter
        let func = &result.functions[0];
        assert_eq!(func.params.len(), 1, "Lambda should have one parameter");
    }

    #[test]
    fn test_let_expression() {
        // (let ((x 1)) x) should transform to a Let binding
        let result = transform("(let ((x 1)) x)").expect("Failed to transform let expression");

        // The entry should have a Let binding for x
        assert!(
            has_let_binding(&result.entry),
            "Let expression should produce Let binding"
        );
    }

    #[test]
    fn test_if_expression() {
        // (if #t 1 2) should transform with proper control flow
        let result = transform("(if #t 1 2)").expect("Failed to transform if expression");

        // Should contain an If complex expression
        assert!(
            has_if(&result.entry),
            "If expression should produce If in ANF"
        );
    }

    #[test]
    fn test_define() {
        // (define x 5) should transform to a global binding
        let result = transform("(define x 5)").expect("Failed to transform define");

        // Define at top level should produce a Let for the variable binding
        assert!(
            has_let_binding(&result.entry),
            "Define should produce a Let binding for the variable"
        );
    }

    #[test]
    fn test_begin_with_side_effects() {
        // (begin (+ 1 2) (+ 3 4)) - operations with non-trivial expressions
        // The sequence function optimizes away simple returns, so we need
        // expressions that produce Let bindings
        let result =
            transform("(begin (+ 1 2) (+ 3 4))").expect("Failed to transform begin with effects");

        // Each (+ a b) produces a Let binding, and sequencing them should preserve both
        // The result should have at least 2 Let bindings
        let count = count_let_bindings(&result.entry);
        assert!(
            count >= 2,
            "Begin with arithmetic should produce multiple Let bindings, got {}",
            count
        );
    }

    #[test]
    fn test_tail_call_in_lambda() {
        // Create a lambda with a tail call: (lambda (x) (f x))
        // where f is defined first
        let result = transform("(define f (lambda (x) x)) (lambda (y) (f y))")
            .expect("Failed to transform lambda with tail call");

        // Should have two functions (one for f, one for the second lambda)
        assert!(
            result.functions.len() >= 2,
            "Should have at least two function definitions"
        );
    }

    #[test]
    fn test_function_define_creates_function() {
        // (define (add a b) (+ a b)) should create a function
        let result =
            transform("(define (add a b) (+ a b))").expect("Failed to transform function define");

        // Should have created a function
        assert!(
            !result.functions.is_empty(),
            "Function define should create a function definition"
        );

        // The function should have two parameters
        let func = &result.functions[0];
        assert_eq!(func.params.len(), 2, "add should have two parameters");
    }

    #[test]
    fn test_recursive_function() {
        // (define (f x) (f x)) - a recursive function
        let result =
            transform("(define (f x) (f x))").expect("Failed to transform recursive function");

        // Should have a function definition
        assert!(
            !result.functions.is_empty(),
            "Should have function f defined"
        );

        // The function body should contain a tail call (TailApp in this implementation)
        let func = &result.functions[0];
        assert!(
            has_tail_call(&func.body),
            "Recursive call in tail position should produce TailCall/TailApp"
        );
    }

    #[test]
    fn test_multiple_defines() {
        // Multiple defines should all be processed
        let result =
            transform("(define a 1) (define b 2)").expect("Failed to transform multiple defines");

        // Should have Let bindings for both defines
        let count = count_let_bindings(&result.entry);
        assert!(
            count >= 2,
            "Multiple defines should produce multiple Let bindings, got {}",
            count
        );
    }

    #[test]
    fn test_string_interning() {
        // String literals should be interned
        let result = transform("\"hello\"").expect("Failed to transform string");

        assert!(
            !result.strings.is_empty(),
            "String literal should be interned"
        );
        assert!(
            result.strings.contains(&"hello".to_string()),
            "Interned strings should contain 'hello'"
        );
    }

    #[test]
    fn test_quote_symbol() {
        // Quoted symbols should be interned
        let result = transform("'foo").expect("Failed to transform quoted symbol");

        assert!(
            !result.symbols.is_empty(),
            "Quoted symbol should be interned"
        );
        assert!(
            result.symbols.contains(&"foo".to_string()),
            "Interned symbols should contain 'foo'"
        );
    }
}
