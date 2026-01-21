//! Closure Conversion
//!
//! Transforms first-class functions into explicit closure objects.
//! After conversion, all functions are top-level with explicit environment parameters.

use std::collections::HashSet;

use log::{debug, info, trace};

use super::anf::*;

/// Closure converter
pub struct ClosureConverter {
    /// Generated lifted functions
    lifted_functions: Vec<FunctionDef>,
    /// Map from function label to free variables (for populating MakeClosure captures)
    func_free_vars: std::collections::HashMap<String, Vec<VarId>>,
}

impl ClosureConverter {
    pub fn new() -> Self {
        Self {
            lifted_functions: Vec::new(),
            func_free_vars: std::collections::HashMap::new(),
        }
    }

    /// Convert a program, making all closures explicit
    pub fn convert(&mut self, program: AnfProgram) -> AnfProgram {
        info!(
            target: "closure",
            "Starting closure conversion for {} functions",
            program.functions.len()
        );

        // First pass: analyze direct free variables for all functions
        let mut functions_with_fv: Vec<FunctionDef> = program
            .functions
            .into_iter()
            .map(|mut f| {
                let bound: HashSet<_> = f.params.iter().cloned().collect();
                f.free_vars = self.analyze_free_vars(&f.body, &bound);
                // Store free vars for this function so MakeClosure can look it up
                let fv_list: Vec<_> = f.free_vars.iter().cloned().collect();

                debug!(
                    target: "closure",
                    "Function '{}': direct free vars = {:?}, params = {:?}",
                    f.label,
                    fv_list,
                    f.params.iter().map(|p| p.name()).collect::<Vec<_>>()
                );

                self.func_free_vars.insert(f.label.clone(), fv_list);
                f
            })
            .collect();

        // Second pass: propagate free variables transitively
        // If function A creates closure for function B, A needs B's free vars too
        debug!(target: "closure", "Starting transitive free variable propagation");
        self.propagate_free_vars(&mut functions_with_fv);

        // Log final captures after propagation
        for f in &functions_with_fv {
            if !f.free_vars.is_empty() {
                info!(
                    target: "closure",
                    "Function '{}' captures: {:?}",
                    f.label,
                    f.free_vars.iter().map(|v| v.name()).collect::<Vec<_>>()
                );
            }
        }

        // Third pass: convert functions
        debug!(target: "closure", "Converting function bodies");
        let converted_functions: Vec<_> = functions_with_fv
            .into_iter()
            .map(|f| self.convert_function(f))
            .collect();

        // Convert entry point
        let entry = self.convert_expr(&program.entry, &HashSet::new());

        // Combine all functions
        let mut all_functions = converted_functions;
        all_functions.append(&mut self.lifted_functions);

        info!(
            target: "closure",
            "Closure conversion complete: {} total functions",
            all_functions.len()
        );

        AnfProgram {
            functions: all_functions,
            entry,
            strings: program.strings,
            symbols: program.symbols,
        }
    }

    /// Propagate free variables transitively through MakeClosure expressions
    fn propagate_free_vars(&mut self, functions: &mut [FunctionDef]) {
        // Iterate until no changes (fixed point)
        let mut iteration = 0;
        loop {
            iteration += 1;
            let mut changed = false;

            trace!(
                target: "closure",
                "Transitive propagation iteration {}",
                iteration
            );

            for func in functions.iter_mut() {
                let bound: HashSet<_> = func.params.iter().cloned().collect();
                let new_fv = self.collect_transitive_free_vars(&func.body, &bound);

                // Add any new free variables
                for v in new_fv {
                    if !func.free_vars.contains(&v) {
                        debug!(
                            target: "closure",
                            "Transitive capture: '{}' now captures '{}' (iteration {})",
                            func.label,
                            v.name(),
                            iteration
                        );
                        func.free_vars.insert(v.clone());
                        changed = true;
                    }
                }

                // Update the map
                let fv_list: Vec<_> = func.free_vars.iter().cloned().collect();
                self.func_free_vars.insert(func.label.clone(), fv_list);
            }

            if !changed {
                debug!(
                    target: "closure",
                    "Transitive propagation converged after {} iterations",
                    iteration
                );
                break;
            }
        }
    }

    /// Collect free variables including transitive ones from MakeClosure
    fn collect_transitive_free_vars(
        &self,
        expr: &AnfExpr,
        bound: &HashSet<VarId>,
    ) -> HashSet<VarId> {
        match expr {
            AnfExpr::Return(atom) => self.free_vars_atom(atom, bound),

            AnfExpr::Let { var, value, body } => {
                let mut fv = self.collect_transitive_free_vars_complex(value, bound);
                let mut new_bound = bound.clone();
                new_bound.insert(var.clone());
                fv.extend(self.collect_transitive_free_vars(body, &new_bound));
                fv
            }

            AnfExpr::Seq { effect, body } => {
                let mut fv = self.collect_transitive_free_vars_complex(effect, bound);
                fv.extend(self.collect_transitive_free_vars(body, bound));
                fv
            }

            AnfExpr::TailCall { func, args } => {
                let mut fv = self.free_vars_atom(func, bound);
                for arg in args {
                    fv.extend(self.free_vars_atom(arg, bound));
                }
                fv
            }

            AnfExpr::Halt(atom) => self.free_vars_atom(atom, bound),
        }
    }

    fn collect_transitive_free_vars_complex(
        &self,
        expr: &ComplexExpr,
        bound: &HashSet<VarId>,
    ) -> HashSet<VarId> {
        match expr {
            ComplexExpr::MakeClosure { label, captures } => {
                let mut fv: HashSet<VarId> = captures
                    .iter()
                    .filter(|v| !bound.contains(*v))
                    .cloned()
                    .collect();
                // Add the target function's free variables (transitive)
                if let Some(target_fv) = self.func_free_vars.get(label) {
                    for v in target_fv {
                        if !bound.contains(v) {
                            fv.insert(v.clone());
                        }
                    }
                }
                fv
            }
            // If expressions can contain nested ANF expressions with MakeClosures
            ComplexExpr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                let mut fv = self.free_vars_atom(cond, bound);
                fv.extend(self.collect_transitive_free_vars(then_expr, bound));
                fv.extend(self.collect_transitive_free_vars(else_expr, bound));
                fv
            }
            // For other expressions, delegate to the regular analysis
            other => self.free_vars_complex(other, bound),
        }
    }

    fn convert_function(&mut self, mut func: FunctionDef) -> FunctionDef {
        let bound: HashSet<_> = func.params.iter().cloned().collect();

        // ALL functions need the closure as first parameter (calling convention)
        func.has_env = true;

        // If function has free variables, convert body with environment access
        if !func.free_vars.is_empty() {
            debug!(
                target: "closure",
                "Converting function '{}' with {} captures: {:?}",
                func.label,
                func.free_vars.len(),
                func.free_vars.iter().map(|v| v.name()).collect::<Vec<_>>()
            );
            let _env_param = VarId::new("__env");
            func.body = self.convert_expr_with_env(&func.body, &bound, &func.free_vars);
        } else {
            trace!(
                target: "closure",
                "Converting function '{}' with no captures",
                func.label
            );
            func.body = self.convert_expr(&func.body, &bound);
        }

        func
    }

    fn convert_expr(&mut self, expr: &AnfExpr, bound: &HashSet<VarId>) -> AnfExpr {
        match expr {
            AnfExpr::Return(atom) => AnfExpr::Return(atom.clone()),

            AnfExpr::Let { var, value, body } => {
                let converted_value = self.convert_complex(value, bound);

                let mut new_bound = bound.clone();
                new_bound.insert(var.clone());

                AnfExpr::Let {
                    var: var.clone(),
                    value: converted_value,
                    body: Box::new(self.convert_expr(body, &new_bound)),
                }
            }

            AnfExpr::Seq { effect, body } => AnfExpr::Seq {
                effect: self.convert_complex(effect, bound),
                body: Box::new(self.convert_expr(body, bound)),
            },

            AnfExpr::TailCall { func, args } => AnfExpr::TailCall {
                func: func.clone(),
                args: args.clone(),
            },

            AnfExpr::Halt(atom) => AnfExpr::Halt(atom.clone()),
        }
    }

    fn convert_expr_with_env(
        &mut self,
        expr: &AnfExpr,
        bound: &HashSet<VarId>,
        free_vars: &HashSet<VarId>,
    ) -> AnfExpr {
        // Build closure references for free variables
        let free_var_list: Vec<_> = free_vars.iter().cloned().collect();
        let mut result = self.convert_expr(expr, bound);

        // Prepend ClosureRef bindings for each free variable
        for (index, var) in free_var_list.iter().enumerate().rev() {
            result = AnfExpr::Let {
                var: var.clone(),
                value: ComplexExpr::ClosureRef {
                    closure: VarId::new("__env"),
                    index,
                },
                body: Box::new(result),
            };
        }

        result
    }

    fn convert_complex(&mut self, expr: &ComplexExpr, bound: &HashSet<VarId>) -> ComplexExpr {
        match expr {
            ComplexExpr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                trace!(
                    target: "closure",
                    "Processing If expression in closure conversion"
                );
                ComplexExpr::If {
                    cond: cond.clone(),
                    then_expr: Box::new(self.convert_expr(then_expr, bound)),
                    else_expr: Box::new(self.convert_expr(else_expr, bound)),
                }
            }

            ComplexExpr::MakeClosure { label, captures: _ } => {
                // Look up the function's free variables to use as captures
                let captures = self.func_free_vars.get(label).cloned().unwrap_or_default();

                debug!(
                    target: "closure",
                    "MakeClosure for '{}': captures = {:?}",
                    label,
                    captures.iter().map(|v| v.name()).collect::<Vec<_>>()
                );

                ComplexExpr::MakeClosure {
                    label: label.clone(),
                    captures,
                }
            }

            // Other complex expressions pass through
            other => other.clone(),
        }
    }

    /// Analyze free variables in an expression
    fn analyze_free_vars(&self, expr: &AnfExpr, bound: &HashSet<VarId>) -> HashSet<VarId> {
        match expr {
            AnfExpr::Return(atom) => self.free_vars_atom(atom, bound),

            AnfExpr::Let { var, value, body } => {
                let mut fv = self.free_vars_complex(value, bound);
                let mut new_bound = bound.clone();
                new_bound.insert(var.clone());
                fv.extend(self.analyze_free_vars(body, &new_bound));
                fv
            }

            AnfExpr::Seq { effect, body } => {
                let mut fv = self.free_vars_complex(effect, bound);
                fv.extend(self.analyze_free_vars(body, bound));
                fv
            }

            AnfExpr::TailCall { func, args } => {
                let mut fv = self.free_vars_atom(func, bound);
                for arg in args {
                    fv.extend(self.free_vars_atom(arg, bound));
                }
                fv
            }

            AnfExpr::Halt(atom) => self.free_vars_atom(atom, bound),
        }
    }

    fn free_vars_complex(&self, expr: &ComplexExpr, bound: &HashSet<VarId>) -> HashSet<VarId> {
        match expr {
            ComplexExpr::PrimApp { args, .. } => {
                let mut fv = HashSet::new();
                for arg in args {
                    fv.extend(self.free_vars_atom(arg, bound));
                }
                fv
            }

            ComplexExpr::App { func, args } | ComplexExpr::TailApp { func, args } => {
                let mut fv = self.free_vars_atom(func, bound);
                for arg in args {
                    fv.extend(self.free_vars_atom(arg, bound));
                }
                fv
            }

            ComplexExpr::MakeClosure { captures, .. } => captures
                .iter()
                .filter(|v| !bound.contains(*v))
                .cloned()
                .collect(),

            ComplexExpr::ClosureRef { closure, .. } => {
                if bound.contains(closure) {
                    HashSet::new()
                } else {
                    let mut fv = HashSet::new();
                    fv.insert(closure.clone());
                    fv
                }
            }

            ComplexExpr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                let mut fv = self.free_vars_atom(cond, bound);
                fv.extend(self.analyze_free_vars(then_expr, bound));
                fv.extend(self.analyze_free_vars(else_expr, bound));
                fv
            }

            ComplexExpr::MakeBox(atom) => self.free_vars_atom(atom, bound),

            ComplexExpr::ReadBox(var) => {
                if bound.contains(var) {
                    HashSet::new()
                } else {
                    let mut fv = HashSet::new();
                    fv.insert(var.clone());
                    fv
                }
            }

            ComplexExpr::WriteBox { box_var, value } => {
                let mut fv = self.free_vars_atom(value, bound);
                if !bound.contains(box_var) {
                    fv.insert(box_var.clone());
                }
                fv
            }
        }
    }

    fn free_vars_atom(&self, atom: &Atom, bound: &HashSet<VarId>) -> HashSet<VarId> {
        match atom {
            Atom::Var(v) if !bound.contains(v) => {
                let mut fv = HashSet::new();
                fv.insert(v.clone());
                fv
            }
            _ => HashSet::new(),
        }
    }
}

impl Default for ClosureConverter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::anf::{AnfTransformer, VarId};
    use crate::parser::read_all;

    /// Helper function to transform Scheme source code into an ANF program
    fn transform(s: &str) -> AnfProgram {
        let exprs = read_all(s).unwrap();
        let mut transformer = AnfTransformer::new();
        transformer.transform_program(exprs).unwrap()
    }

    /// Helper to create a VarId from a string
    fn var(s: &str) -> VarId {
        VarId(s.to_string())
    }

    #[test]
    fn test_simple_lambda_no_free_vars() {
        // (lambda (x) x) - a simple identity function with no free variables
        let program = transform("(lambda (x) x)");
        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // The lambda should have been lifted to a function
        // It should have no free variables since x is a parameter
        for func in &converted.functions {
            if func.label.starts_with("lambda") {
                assert!(
                    func.free_vars.is_empty(),
                    "Identity lambda should have no free variables, but found: {:?}",
                    func.free_vars
                );
            }
        }
    }

    #[test]
    fn test_lambda_with_free_variable() {
        // (let ((y 1)) (lambda (x) y)) - lambda captures y from enclosing scope
        let program = transform("(let ((y 1)) (lambda (x) y))");
        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Find the lambda function - it should capture y
        let lambda_func = converted
            .functions
            .iter()
            .find(|f| f.label.starts_with("lambda"));

        assert!(
            lambda_func.is_some(),
            "Should have a lambda function in the program"
        );

        let lambda = lambda_func.unwrap();
        assert!(
            lambda.free_vars.contains(&var("y")),
            "Lambda should capture 'y', but free_vars is: {:?}",
            lambda.free_vars
        );
    }

    #[test]
    fn test_nested_lambdas() {
        // (lambda (x) (lambda (y) x)) - inner lambda captures x from outer
        let program = transform("(lambda (x) (lambda (y) x))");
        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Should have two lambda functions
        let lambda_funcs: Vec<_> = converted
            .functions
            .iter()
            .filter(|f| f.label.starts_with("lambda"))
            .collect();

        assert!(
            lambda_funcs.len() >= 2,
            "Should have at least 2 lambda functions, found: {}",
            lambda_funcs.len()
        );

        // The inner lambda (which takes y) should capture x
        // Find the lambda that has 'y' as a parameter
        let inner_lambda = lambda_funcs.iter().find(|f| f.params.contains(&var("y")));

        if let Some(inner) = inner_lambda {
            assert!(
                inner.free_vars.contains(&var("x")),
                "Inner lambda should capture 'x', but free_vars is: {:?}",
                inner.free_vars
            );
        }
    }

    #[test]
    fn test_multiple_free_vars() {
        // (let ((a 1) (b 2)) (lambda (x) (+ a b))) - lambda captures both a and b
        let program = transform("(let ((a 1) (b 2)) (lambda (x) (+ a b)))");
        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Find the lambda function
        let lambda_func = converted
            .functions
            .iter()
            .find(|f| f.label.starts_with("lambda"));

        assert!(
            lambda_func.is_some(),
            "Should have a lambda function in the program"
        );

        let lambda = lambda_func.unwrap();
        assert!(
            lambda.free_vars.contains(&var("a")),
            "Lambda should capture 'a', but free_vars is: {:?}",
            lambda.free_vars
        );
        assert!(
            lambda.free_vars.contains(&var("b")),
            "Lambda should capture 'b', but free_vars is: {:?}",
            lambda.free_vars
        );
    }

    #[test]
    fn test_no_capture_of_bound_vars() {
        // (lambda (x) (let ((y 1)) y)) - y is bound inside, not free
        let program = transform("(lambda (x) (let ((y 1)) y))");
        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Find the lambda function
        let lambda_func = converted
            .functions
            .iter()
            .find(|f| f.label.starts_with("lambda"));

        assert!(
            lambda_func.is_some(),
            "Should have a lambda function in the program"
        );

        let lambda = lambda_func.unwrap();
        // y is bound inside the lambda body, not free
        assert!(
            !lambda.free_vars.contains(&var("y")),
            "Lambda should NOT capture 'y' (it's bound inside), but free_vars is: {:?}",
            lambda.free_vars
        );
        // x is a parameter, also not free
        assert!(
            !lambda.free_vars.contains(&var("x")),
            "Lambda should NOT capture 'x' (it's a parameter), but free_vars is: {:?}",
            lambda.free_vars
        );
    }

    #[test]
    fn test_recursive_function() {
        // (define (f x) (f x)) - f is recursive, should be in scope
        // Note: In ANF, (define (f x) ...) creates a lambda that gets lifted
        let program = transform("(define (f x) (f x))");
        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // The define creates a lambda which gets lifted to a function
        // Check that we have at least one function in the program
        assert!(
            !converted.functions.is_empty(),
            "Should have at least one function after closure conversion"
        );

        // Find the lambda function (may be named lambda_N)
        let lambda_func = converted
            .functions
            .iter()
            .find(|f| f.label.starts_with("lambda"));

        assert!(
            lambda_func.is_some(),
            "Should have a lambda function for the define"
        );

        let func = lambda_func.unwrap();
        // The function should have exactly 1 parameter (x)
        assert_eq!(
            func.params.len(),
            1,
            "Recursive function should have 1 parameter"
        );

        // The function f references itself - but since f is bound at top level
        // and the lambda refers to it, f might appear as a free variable
        // depending on how the ANF transformation handles recursive defines.
        // This test verifies the conversion doesn't crash on recursive definitions.
    }

    #[test]
    fn test_converter_is_reusable() {
        // Test that a converter can be reused for multiple programs
        let mut converter = ClosureConverter::new();

        let program1 = transform("(lambda (x) x)");
        let _converted1 = converter.convert(program1);

        let program2 = transform("(let ((y 1)) (lambda (x) y))");
        let converted2 = converter.convert(program2);

        // Second conversion should still work correctly
        let lambda_func = converted2
            .functions
            .iter()
            .find(|f| f.label.starts_with("lambda"));

        assert!(
            lambda_func.is_some(),
            "Second conversion should produce a lambda function"
        );
    }

    #[test]
    fn test_closure_conversion_preserves_strings_and_symbols() {
        // Test that string and symbol tables are preserved through conversion
        let program = transform("(lambda (x) \"hello\")");
        let original_strings = program.strings.clone();
        let original_symbols = program.symbols.clone();

        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        assert_eq!(
            converted.strings, original_strings,
            "String table should be preserved"
        );
        assert_eq!(
            converted.symbols, original_symbols,
            "Symbol table should be preserved"
        );
    }

    // ========================================
    // Transitive Closure Capture Tests
    // ========================================
    // These tests verify that when an inner lambda references a global (or outer scope variable),
    // the outer lambda that creates it must also capture that variable transitively.

    #[test]
    fn test_transitive_capture_inner_lambda_needs_global() {
        // Test: Inner lambda references a global function, outer lambda must capture it
        //
        // (define my-fn (lambda (x lst) #t))
        // (define make-pred
        //   (lambda (captured)
        //     (lambda (x) (my-fn x captured))))
        //
        // The inner lambda needs both `my-fn` (global) and `captured` (from outer scope).
        // `make-pred` must transitively capture `my-fn` to pass it to the inner lambda.
        let program = transform(
            r#"
            (define my-fn (lambda (x lst) #t))
            (define make-pred
              (lambda (captured)
                (lambda (x) (my-fn x captured))))
            "#,
        );

        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Find make-pred's lambda (the one with parameter "captured")
        let make_pred_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("captured")));

        assert!(
            make_pred_lambda.is_some(),
            "Should have make-pred lambda with 'captured' parameter"
        );

        let make_pred = make_pred_lambda.unwrap();

        // make-pred should capture my-fn transitively (because the inner lambda needs it)
        assert!(
            make_pred.free_vars.contains(&var("my-fn")),
            "make-pred should transitively capture 'my-fn', but free_vars is: {:?}",
            make_pred.free_vars
        );

        // Find the inner lambda (the one with parameter "x" that isn't my-fn)
        let inner_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("x")) && !f.params.contains(&var("lst")));

        assert!(
            inner_lambda.is_some(),
            "Should have inner lambda with 'x' parameter"
        );

        let inner = inner_lambda.unwrap();

        // Inner lambda should capture both my-fn and captured
        assert!(
            inner.free_vars.contains(&var("my-fn")),
            "Inner lambda should capture 'my-fn', but free_vars is: {:?}",
            inner.free_vars
        );
        assert!(
            inner.free_vars.contains(&var("captured")),
            "Inner lambda should capture 'captured', but free_vars is: {:?}",
            inner.free_vars
        );
    }

    #[test]
    fn test_transitive_capture_nested_lambda_global_reference() {
        // Test: Nested lambda referencing a global - outer wrapper must capture it
        //
        // (define outer-fn (lambda (x) x))
        // (define wrapper
        //   (lambda (y)
        //     (lambda (z) (outer-fn z))))
        //
        // The inner lambda needs outer-fn, so wrapper must capture it.
        let program = transform(
            r#"
            (define outer-fn (lambda (x) x))
            (define wrapper
              (lambda (y)
                (lambda (z) (outer-fn z))))
            "#,
        );

        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Find wrapper's lambda (the one with parameter "y")
        let wrapper_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("y")));

        assert!(
            wrapper_lambda.is_some(),
            "Should have wrapper lambda with 'y' parameter"
        );

        let wrapper = wrapper_lambda.unwrap();

        // wrapper should capture outer-fn transitively
        assert!(
            wrapper.free_vars.contains(&var("outer-fn")),
            "wrapper should transitively capture 'outer-fn', but free_vars is: {:?}",
            wrapper.free_vars
        );

        // Find the inner lambda (the one with parameter "z")
        let inner_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("z")));

        assert!(
            inner_lambda.is_some(),
            "Should have inner lambda with 'z' parameter"
        );

        let inner = inner_lambda.unwrap();

        // Inner lambda should capture outer-fn
        assert!(
            inner.free_vars.contains(&var("outer-fn")),
            "Inner lambda should capture 'outer-fn', but free_vars is: {:?}",
            inner.free_vars
        );
    }

    #[test]
    fn test_transitive_capture_closure_inside_if_branch() {
        // Test: Closure created inside an if branch - critical bug case
        // This was the bug that caused game-of-life to fail.
        //
        // (define checker (lambda (x y) #t))
        // (define test-fn
        //   (lambda (lst)
        //     (if (null? lst)
        //         '()
        //         (lambda (x) (checker x lst)))))
        //
        // The lambda inside the else branch needs checker and lst.
        // test-fn must transitively capture checker.
        let program = transform(
            r#"
            (define checker (lambda (x y) #t))
            (define test-fn
              (lambda (lst)
                (if (null? lst)
                    '()
                    (lambda (x) (checker x lst)))))
            "#,
        );

        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Find test-fn's lambda (the one with parameter "lst")
        let test_fn_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("lst")));

        assert!(
            test_fn_lambda.is_some(),
            "Should have test-fn lambda with 'lst' parameter"
        );

        let test_fn = test_fn_lambda.unwrap();

        // test-fn should capture checker transitively (inner lambda in if branch needs it)
        assert!(
            test_fn.free_vars.contains(&var("checker")),
            "test-fn should transitively capture 'checker' from if branch, but free_vars is: {:?}",
            test_fn.free_vars
        );
    }

    #[test]
    fn test_transitive_capture_closure_inside_cond() {
        // Test: Closure created inside a cond expression (compiles to nested ifs)
        //
        // (define my-check (lambda (x lst) (> x 0)))
        // (define test-rec
        //   (lambda (input)
        //     (cond
        //       ((null? input) '())
        //       (else (lambda (x) (my-check x input))))))
        //
        // The lambda inside cond's else branch needs my-check.
        // test-rec must transitively capture my-check.
        let program = transform(
            r#"
            (define my-check (lambda (x lst) (> x 0)))
            (define test-rec
              (lambda (input)
                (cond
                  ((null? input) '())
                  (else (lambda (x) (my-check x input))))))
            "#,
        );

        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Find test-rec's lambda (the one with parameter "input")
        let test_rec_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("input")));

        assert!(
            test_rec_lambda.is_some(),
            "Should have test-rec lambda with 'input' parameter"
        );

        let test_rec = test_rec_lambda.unwrap();

        // test-rec should capture my-check transitively (inner lambda in cond needs it)
        assert!(
            test_rec.free_vars.contains(&var("my-check")),
            "test-rec should transitively capture 'my-check' from cond branch, but free_vars is: {:?}",
            test_rec.free_vars
        );
    }

    #[test]
    fn test_transitive_capture_multiple_levels() {
        // Test: Three levels of nesting - transitive capture through multiple layers
        //
        // (define global-fn (lambda (x) x))
        // (define outer
        //   (lambda (a)
        //     (lambda (b)
        //       (lambda (c) (global-fn c)))))
        //
        // The innermost lambda needs global-fn.
        // Both middle and outer lambdas must capture it transitively.
        let program = transform(
            r#"
            (define global-fn (lambda (x) x))
            (define outer
              (lambda (a)
                (lambda (b)
                  (lambda (c) (global-fn c)))))
            "#,
        );

        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Find outer's lambda (parameter "a")
        let outer_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("a")));

        assert!(
            outer_lambda.is_some(),
            "Should have outer lambda with 'a' parameter"
        );

        // Find middle lambda (parameter "b")
        let middle_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("b")));

        assert!(
            middle_lambda.is_some(),
            "Should have middle lambda with 'b' parameter"
        );

        // Find inner lambda (parameter "c")
        let inner_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("c")));

        assert!(
            inner_lambda.is_some(),
            "Should have inner lambda with 'c' parameter"
        );

        let outer = outer_lambda.unwrap();
        let middle = middle_lambda.unwrap();
        let inner = inner_lambda.unwrap();

        // All three should capture global-fn
        assert!(
            inner.free_vars.contains(&var("global-fn")),
            "Inner lambda should capture 'global-fn', but free_vars is: {:?}",
            inner.free_vars
        );
        assert!(
            middle.free_vars.contains(&var("global-fn")),
            "Middle lambda should transitively capture 'global-fn', but free_vars is: {:?}",
            middle.free_vars
        );
        assert!(
            outer.free_vars.contains(&var("global-fn")),
            "Outer lambda should transitively capture 'global-fn', but free_vars is: {:?}",
            outer.free_vars
        );
    }

    #[test]
    fn test_transitive_capture_with_let_binding() {
        // Test: Lambda inside let binding still propagates captures
        //
        // (define helper (lambda (x) x))
        // (define main-fn
        //   (lambda (y)
        //     (let ((inner (lambda (z) (helper z))))
        //       inner)))
        //
        // The inner lambda needs helper, main-fn must capture it.
        let program = transform(
            r#"
            (define helper (lambda (x) x))
            (define main-fn
              (lambda (y)
                (let ((inner (lambda (z) (helper z))))
                  inner)))
            "#,
        );

        let mut converter = ClosureConverter::new();
        let converted = converter.convert(program);

        // Find main-fn's lambda (parameter "y")
        let main_fn_lambda = converted
            .functions
            .iter()
            .find(|f| f.params.contains(&var("y")));

        assert!(
            main_fn_lambda.is_some(),
            "Should have main-fn lambda with 'y' parameter"
        );

        let main_fn = main_fn_lambda.unwrap();

        // main-fn should capture helper transitively
        assert!(
            main_fn.free_vars.contains(&var("helper")),
            "main-fn should transitively capture 'helper', but free_vars is: {:?}",
            main_fn.free_vars
        );
    }
}
