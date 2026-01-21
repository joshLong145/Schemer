//! Code generation from ANF IR to QBE IR
//!
//! This module translates the closure-converted ANF IR into QBE intermediate
//! representation, which can then be compiled to native assembly.

use std::collections::HashMap;

use log::{debug, info, trace};

use crate::compiler::anf::{AnfExpr, AnfProgram, Atom, ComplexExpr, FunctionDef, PrimOp, VarId};
use crate::compiler::primitives::{get_primitive_impl, InlineOp, PrimImpl};
use crate::compiler::qbe::{
    QbeBlock, QbeData, QbeDataItem, QbeFunction, QbeInst, QbeModule, QbeOp, QbeType, QbeValue,
};
use crate::tags;

/// Code generator state
pub struct CodeGenerator {
    /// Counter for generating unique temporaries
    temp_counter: usize,
    /// Counter for generating unique labels
    label_counter: usize,
    /// Current function being generated
    current_function: Option<String>,
    /// Variable to QBE temporary mapping
    var_map: HashMap<VarId, String>,
    /// Pending blocks to emit (for control flow)
    pending_blocks: Vec<QbeBlock>,
    /// String literals from the program (for length info)
    strings: Vec<String>,
}

impl CodeGenerator {
    pub fn new() -> Self {
        Self {
            temp_counter: 0,
            label_counter: 0,
            current_function: None,
            var_map: HashMap::new(),
            pending_blocks: Vec::new(),
            strings: Vec::new(),
        }
    }

    /// Generate a fresh QBE temporary name
    fn fresh_temp(&mut self) -> String {
        let n = self.temp_counter;
        self.temp_counter += 1;
        format!("t{}", n)
    }

    /// Generate a fresh label name
    fn fresh_label(&mut self, prefix: &str) -> String {
        let n = self.label_counter;
        self.label_counter += 1;
        format!("{}_{}", prefix, n)
    }

    /// Get the QBE temporary for a variable
    fn get_var(&self, var: &VarId) -> QbeValue {
        if let Some(temp) = self.var_map.get(var) {
            QbeValue::Temp(temp.clone())
        } else {
            // Shouldn't happen if ANF is well-formed
            panic!("Undefined variable: {:?}", var);
        }
    }

    /// Bind a variable to a QBE temporary
    fn bind_var(&mut self, var: &VarId) -> String {
        let temp = self.fresh_temp();
        self.var_map.insert(var.clone(), temp.clone());
        temp
    }

    /// Add a jump instruction to an existing pending block
    fn add_jump_to_block(&mut self, block_label: &str, target: &str) {
        for block in &mut self.pending_blocks {
            if block.label == block_label {
                block.instructions.push(QbeInst::Jmp(target.to_string()));
                return;
            }
        }
    }

    /// Check if instruction list ends with a terminator (jmp, jnz, ret, hlt)
    fn is_terminated(insts: &[QbeInst]) -> bool {
        matches!(
            insts.last(),
            Some(QbeInst::Jmp(_))
                | Some(QbeInst::Jnz { .. })
                | Some(QbeInst::Ret(_))
                | Some(QbeInst::Hlt)
        )
    }

    /// Sanitize identifier for QBE (replace hyphens with underscores)
    fn sanitize_ident(name: &str) -> String {
        name.replace('-', "_").replace('?', "_p").replace('!', "_b")
    }

    /// Generate QBE module from ANF program
    pub fn generate(&mut self, program: &AnfProgram) -> QbeModule {
        info!(
            target: "codegen",
            "Starting code generation: {} functions, {} strings, {} symbols",
            program.functions.len(),
            program.strings.len(),
            program.symbols.len()
        );

        let mut module = QbeModule::new();

        // Store strings for later use in generate_atom
        self.strings = program.strings.clone();

        // Generate data section for strings (raw bytes)
        for (idx, s) in program.strings.iter().enumerate() {
            trace!(target: "codegen", "Data section: str_{} = {:?}", idx, s);
            let data = QbeData {
                export: false,
                name: format!("str_{}", idx),
                items: vec![QbeDataItem::String(s.clone())],
            };
            module.data.push(data);
        }

        // Generate data section for string value globals (will hold tagged pointers)
        for idx in 0..program.strings.len() {
            let data = QbeData {
                export: false,
                name: format!("str_val_{}", idx),
                items: vec![QbeDataItem::Zero(8)], // 8 bytes for a 64-bit value
            };
            module.data.push(data);
        }

        // Generate data section for symbols
        for (idx, s) in program.symbols.iter().enumerate() {
            trace!(target: "codegen", "Data section: sym_{} = {:?}", idx, s);
            let data = QbeData {
                export: false,
                name: format!("sym_{}", idx),
                items: vec![QbeDataItem::String(s.clone())],
            };
            module.data.push(data);
        }

        // Generate functions
        for func in &program.functions {
            let qbe_func = self.generate_function(func);
            module.functions.push(qbe_func);
        }

        // Generate entry point wrapper (main)
        let main_func = self.generate_main(&program.entry, &program.strings);
        module.functions.push(main_func);

        info!(
            target: "codegen",
            "Code generation complete: {} QBE functions generated",
            module.functions.len()
        );

        module
    }

    /// Generate a QBE function from ANF function definition
    fn generate_function(&mut self, func: &FunctionDef) -> QbeFunction {
        self.var_map.clear();
        self.pending_blocks.clear();
        self.temp_counter = 0;
        self.label_counter = 0;
        self.current_function = Some(func.label.clone());

        debug!(
            target: "codegen",
            "Generating function '{}': {} params, has_env={}, {} free vars",
            func.label,
            func.params.len(),
            func.has_env,
            func.free_vars.len()
        );

        // Build parameter list
        let mut params = Vec::new();

        // If function has environment, first param is the closure
        if func.has_env {
            let env_temp = format!("env");
            self.var_map.insert(VarId::new("__env"), env_temp.clone());
            params.push((QbeType::Long, env_temp));
            trace!(target: "codegen", "  Function '{}': added __env parameter", func.label);
        }

        // Regular parameters
        for param in &func.params {
            let temp = format!("p_{}", Self::sanitize_ident(param.name()));
            self.var_map.insert(param.clone(), temp.clone());
            params.push((QbeType::Long, temp.clone()));
            trace!(target: "codegen", "  Function '{}': param '{}' -> {}", func.label, param.name(), temp);
        }

        // Generate function body
        let entry_block = self.generate_expr(&func.body, "start");

        // Collect all blocks: entry + any pending blocks from control flow
        let mut blocks = vec![entry_block];
        blocks.append(&mut self.pending_blocks);

        info!(
            target: "codegen",
            "Function '{}' generated: {} blocks",
            func.label,
            blocks.len()
        );

        for block in &blocks {
            trace!(
                target: "codegen",
                "  Block '{}': {} instructions",
                block.label,
                block.instructions.len()
            );
        }

        QbeFunction {
            export: true, // Export all Scheme functions for now
            name: func.label.clone(),
            params,
            return_type: Some(QbeType::Long), // All Scheme values are 64-bit
            blocks,
        }
    }

    /// Generate the main entry point
    fn generate_main(&mut self, entry: &AnfExpr, strings: &[String]) -> QbeFunction {
        self.var_map.clear();
        self.pending_blocks.clear();
        self.temp_counter = 0;
        self.label_counter = 0;
        self.current_function = Some("main".to_string());

        // Create start block with initialization
        let mut start_insts = Vec::new();

        // Call runtime initialization
        start_insts.push(QbeInst::Call {
            dest: None,
            func: QbeValue::Global("scm_init".to_string()),
            args: vec![],
        });

        // Pre-allocate string literals as tagged Scheme values and store in globals
        for (idx, s) in strings.iter().enumerate() {
            let temp = format!("t_str_{}", idx);
            // Allocate the string
            start_insts.push(QbeInst::Call {
                dest: Some(temp.clone()),
                func: QbeValue::Global("scm_alloc_string".to_string()),
                args: vec![
                    (QbeType::Long, QbeValue::Global(format!("str_{}", idx))),
                    (QbeType::Long, QbeValue::Const(s.len() as i64)),
                ],
            });
            // Store in global
            start_insts.push(QbeInst::Store {
                ty: QbeType::Long,
                value: QbeValue::Temp(temp),
                addr: QbeValue::Global(format!("str_val_{}", idx)),
            });
        }

        // Generate the entry expression, storing result in a temp
        let result_temp = self.fresh_temp();
        self.generate_expr_to_dest(entry, &result_temp, &mut start_insts);

        // Epilogue: shutdown and return
        let epilogue = vec![
            QbeInst::Call {
                dest: None,
                func: QbeValue::Global("scm_shutdown".to_string()),
                args: vec![],
            },
            QbeInst::Ret(Some(QbeValue::Const(0))),
        ];

        // If control flow was split, epilogue goes in the last block (join)
        // Otherwise it goes in start
        if self.pending_blocks.is_empty() {
            start_insts.extend(epilogue);
        } else {
            // Find the last block (should be a join block) and add epilogue
            if let Some(last_block) = self.pending_blocks.last_mut() {
                last_block.instructions.extend(epilogue);
            }
        }

        let mut blocks = vec![QbeBlock {
            label: "start".to_string(),
            instructions: start_insts,
        }];

        blocks.append(&mut self.pending_blocks);

        QbeFunction {
            export: true,
            name: "main".to_string(),
            params: vec![],
            return_type: Some(QbeType::Word),
            blocks,
        }
    }

    /// Generate a QBE block for an ANF expression
    fn generate_expr(&mut self, expr: &AnfExpr, label: &str) -> QbeBlock {
        let mut instructions = Vec::new();
        self.generate_expr_into(expr, &mut instructions);

        QbeBlock {
            label: label.to_string(),
            instructions,
        }
    }

    /// Generate instructions for an expression, appending to the given vector
    fn generate_expr_into(&mut self, expr: &AnfExpr, insts: &mut Vec<QbeInst>) {
        match expr {
            AnfExpr::Return(atom) => {
                let val = self.generate_atom(atom, insts);
                insts.push(QbeInst::Ret(Some(val)));
            }

            AnfExpr::Let { var, value, body } => {
                let dest = self.bind_var(var);
                if let Some(join_label) = self.generate_complex_into(value, &dest, insts) {
                    // Control flow was split - continuation goes in join block
                    // Insert position for join block (after if's then/else blocks)
                    let insert_pos = self.pending_blocks.len();
                    let mut join_block = QbeBlock::new(&join_label);
                    self.generate_expr_into(body, &mut join_block.instructions);
                    // Insert at correct position to maintain block order
                    self.pending_blocks.insert(insert_pos, join_block);
                } else {
                    self.generate_expr_into(body, insts);
                }
            }

            AnfExpr::Seq { effect, body } => {
                let discard = self.fresh_temp();
                if let Some(join_label) = self.generate_complex_into(effect, &discard, insts) {
                    // Insert position for join block (after if's then/else blocks)
                    let insert_pos = self.pending_blocks.len();
                    let mut join_block = QbeBlock::new(&join_label);
                    self.generate_expr_into(body, &mut join_block.instructions);
                    // Insert at correct position to maintain block order
                    self.pending_blocks.insert(insert_pos, join_block);
                } else {
                    self.generate_expr_into(body, insts);
                }
            }

            AnfExpr::TailCall { func, args } => {
                // For now, just generate a regular call and return
                // TODO: Implement proper trampoline-based TCO
                let func_val = self.generate_atom(func, insts);
                let arg_vals = self.generate_args(args, insts);

                let result = self.fresh_temp();
                insts.push(QbeInst::Call {
                    dest: Some(result.clone()),
                    func: func_val,
                    args: arg_vals,
                });
                insts.push(QbeInst::Ret(Some(QbeValue::Temp(result))));
            }

            AnfExpr::Halt(atom) => {
                // Display the error and abort
                let val = self.generate_atom(atom, insts);
                insts.push(QbeInst::Call {
                    dest: None,
                    func: QbeValue::Global("scm_display".to_string()),
                    args: vec![(QbeType::Long, val)],
                });
                insts.push(QbeInst::Hlt);
            }
        }
    }

    /// Generate instructions for an expression, storing final value in dest (for if branches)
    /// Returns labels of blocks that need a terminal jump added
    fn generate_expr_to_dest(
        &mut self,
        expr: &AnfExpr,
        dest: &str,
        insts: &mut Vec<QbeInst>,
    ) -> Vec<String> {
        match expr {
            AnfExpr::Return(atom) => {
                let val = self.generate_atom(atom, insts);
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Copy(val),
                });
                vec![] // Current block needs terminator (caller adds jmp)
            }

            AnfExpr::Let { var, value, body } => {
                let var_dest = self.bind_var(var);
                if let Some(join_label) = self.generate_complex_into(value, &var_dest, insts) {
                    // Insert position for join block (after if's then/else blocks)
                    let insert_pos = self.pending_blocks.len();
                    let mut join_block = QbeBlock::new(&join_label);
                    let nested =
                        self.generate_expr_to_dest(body, dest, &mut join_block.instructions);
                    let block_label = join_block.label.clone();
                    // Insert at correct position to maintain block order
                    self.pending_blocks.insert(insert_pos, join_block);
                    // This join block and any nested blocks need terminators
                    let mut result = vec![block_label];
                    result.extend(nested);
                    result
                } else {
                    self.generate_expr_to_dest(body, dest, insts)
                }
            }

            AnfExpr::Seq { effect, body } => {
                let discard = self.fresh_temp();
                if let Some(join_label) = self.generate_complex_into(effect, &discard, insts) {
                    // Insert position for join block (after if's then/else blocks)
                    let insert_pos = self.pending_blocks.len();
                    let mut join_block = QbeBlock::new(&join_label);
                    let nested =
                        self.generate_expr_to_dest(body, dest, &mut join_block.instructions);
                    let block_label = join_block.label.clone();
                    // Insert at correct position to maintain block order
                    self.pending_blocks.insert(insert_pos, join_block);
                    let mut result = vec![block_label];
                    result.extend(nested);
                    result
                } else {
                    self.generate_expr_to_dest(body, dest, insts)
                }
            }

            AnfExpr::TailCall { func, args } => {
                // In a branch context, tail call becomes regular call
                let func_val = self.generate_atom(func, insts);
                let arg_vals = self.generate_args(args, insts);

                insts.push(QbeInst::Call {
                    dest: Some(dest.to_string()),
                    func: func_val,
                    args: arg_vals,
                });
                vec![]
            }

            AnfExpr::Halt(atom) => {
                let val = self.generate_atom(atom, insts);
                insts.push(QbeInst::Call {
                    dest: None,
                    func: QbeValue::Global("scm_display".to_string()),
                    args: vec![(QbeType::Long, val)],
                });
                insts.push(QbeInst::Hlt);
                vec![] // Halt is terminal
            }
        }
    }

    /// Generate a QBE value for an atom, emitting load instructions if needed
    /// Returns the QbeValue to use for the atom
    fn generate_atom(&mut self, atom: &Atom, insts: &mut Vec<QbeInst>) -> QbeValue {
        match atom {
            Atom::Var(var) => self.get_var(var),
            Atom::Int(n) => {
                // Tag the integer as a fixnum
                QbeValue::Const(tags::make_fixnum(*n) as i64)
            }
            Atom::Float(_f) => {
                // TODO: Handle floats (box them or use NaN tagging)
                unimplemented!("Float compilation not yet implemented")
            }
            Atom::Bool(b) => {
                if *b {
                    QbeValue::Const(tags::VALUE_TRUE as i64)
                } else {
                    QbeValue::Const(tags::VALUE_FALSE as i64)
                }
            }
            Atom::Char(c) => QbeValue::Const(tags::make_char(*c) as i64),
            Atom::String(idx) => {
                // Load from global holding pre-allocated string value
                let temp = self.fresh_temp();
                insts.push(QbeInst::Load {
                    dest: temp.clone(),
                    ty: QbeType::Long,
                    addr: QbeValue::Global(format!("str_val_{}", idx)),
                });
                QbeValue::Temp(temp)
            }
            Atom::Symbol(idx) => {
                // Reference to symbol in data section
                QbeValue::Global(format!("sym_{}", idx))
            }
            Atom::Nil => QbeValue::Const(tags::VALUE_NIL as i64),
            Atom::Void => QbeValue::Const(tags::VALUE_VOID as i64),
        }
    }

    /// Generate QBE call arguments from a list of atoms
    fn generate_args(&mut self, args: &[Atom], insts: &mut Vec<QbeInst>) -> Vec<(QbeType, QbeValue)> {
        let mut result = Vec::with_capacity(args.len());
        for a in args {
            result.push((QbeType::Long, self.generate_atom(a, insts)));
        }
        result
    }

    /// Generate instructions for a complex expression, storing result in dest
    /// Returns the label of the continuation block if control flow was split (e.g., If)
    fn generate_complex_into(
        &mut self,
        expr: &ComplexExpr,
        dest: &str,
        insts: &mut Vec<QbeInst>,
    ) -> Option<String> {
        match expr {
            ComplexExpr::PrimApp { op, args } => {
                self.generate_prim_app(op, args, dest, insts);
                None
            }

            ComplexExpr::App { func, args } => {
                let func_val = self.generate_atom(func, insts);
                let arg_vals = self.generate_args(args, insts);

                let func_ptr = self.fresh_temp();
                insts.push(QbeInst::Call {
                    dest: Some(func_ptr.clone()),
                    func: QbeValue::Global("scm_closure_func".to_string()),
                    args: vec![(QbeType::Long, func_val.clone())],
                });

                let mut full_args = vec![(QbeType::Long, func_val)];
                full_args.extend(arg_vals);

                insts.push(QbeInst::Call {
                    dest: Some(dest.to_string()),
                    func: QbeValue::Temp(func_ptr),
                    args: full_args,
                });
                None
            }

            ComplexExpr::TailApp { func, args } => {
                let func_val = self.generate_atom(func, insts);
                let arg_vals = self.generate_args(args, insts);

                let func_ptr = self.fresh_temp();
                insts.push(QbeInst::Call {
                    dest: Some(func_ptr.clone()),
                    func: QbeValue::Global("scm_closure_func".to_string()),
                    args: vec![(QbeType::Long, func_val.clone())],
                });

                let mut full_args = vec![(QbeType::Long, func_val)];
                full_args.extend(arg_vals);

                insts.push(QbeInst::Call {
                    dest: Some(dest.to_string()),
                    func: QbeValue::Temp(func_ptr),
                    args: full_args,
                });
                None
            }

            ComplexExpr::MakeClosure { label, captures } => {
                let ncaptures = captures.len() as i64;

                let func_name = self.current_function.as_deref().unwrap_or("unknown");
                debug!(
                    target: "codegen",
                    "MakeClosure in '{}': target='{}', {} captures",
                    func_name,
                    label,
                    ncaptures
                );

                insts.push(QbeInst::Call {
                    dest: Some(dest.to_string()),
                    func: QbeValue::Global("scm_alloc_closure".to_string()),
                    args: vec![
                        (QbeType::Long, QbeValue::Global(label.clone())),
                        (QbeType::Long, QbeValue::Const(ncaptures)),
                    ],
                });

                // Handle captures - defer self-references
                let mut deferred: Vec<usize> = Vec::new();
                for (i, var) in captures.iter().enumerate() {
                    if self.var_map.contains_key(var) {
                        let val = self.get_var(var);
                        trace!(
                            target: "codegen",
                            "  Capture slot {}: '{}' (bound)",
                            i,
                            var.name()
                        );
                        insts.push(QbeInst::Call {
                            dest: None,
                            func: QbeValue::Global("scm_closure_set".to_string()),
                            args: vec![
                                (QbeType::Long, QbeValue::Temp(dest.to_string())),
                                (QbeType::Long, QbeValue::Const(i as i64)),
                                (QbeType::Long, val),
                            ],
                        });
                    } else {
                        trace!(
                            target: "codegen",
                            "  Capture slot {}: '{}' (deferred/self-ref)",
                            i,
                            var.name()
                        );
                        deferred.push(i);
                    }
                }
                // Self-references: closure captures itself
                for i in deferred {
                    insts.push(QbeInst::Call {
                        dest: None,
                        func: QbeValue::Global("scm_closure_set".to_string()),
                        args: vec![
                            (QbeType::Long, QbeValue::Temp(dest.to_string())),
                            (QbeType::Long, QbeValue::Const(i as i64)),
                            (QbeType::Long, QbeValue::Temp(dest.to_string())),
                        ],
                    });
                }
                None
            }

            ComplexExpr::ClosureRef { closure, index } => {
                let closure_val = self.get_var(closure);
                insts.push(QbeInst::Call {
                    dest: Some(dest.to_string()),
                    func: QbeValue::Global("scm_closure_ref".to_string()),
                    args: vec![
                        (QbeType::Long, closure_val),
                        (QbeType::Long, QbeValue::Const(*index as i64)),
                    ],
                });
                None
            }

            ComplexExpr::If {
                cond,
                then_expr,
                else_expr,
            } => {
                let cond_val = self.generate_atom(cond, insts);

                let is_false = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: is_false.clone(),
                    ty: QbeType::Word,
                    op: QbeOp::Ceql(cond_val, QbeValue::Const(tags::VALUE_FALSE as i64)),
                });

                let then_label = self.fresh_label("then");
                let else_label = self.fresh_label("else");
                let join_label = self.fresh_label("join");

                let func_name = self.current_function.as_deref().unwrap_or("unknown");
                debug!(
                    target: "codegen",
                    "If branch in '{}': then={}, else={}, join={}",
                    func_name,
                    then_label,
                    else_label,
                    join_label
                );

                insts.push(QbeInst::Jnz {
                    cond: QbeValue::Temp(is_false),
                    if_true: else_label.clone(),
                    if_false: then_label.clone(),
                });

                // Generate then block
                let mut then_block = QbeBlock::new(&then_label);
                let then_needs_jmp =
                    self.generate_expr_to_dest(then_expr, dest, &mut then_block.instructions);
                // Only add jmp if block doesn't already have a terminator
                if !Self::is_terminated(&then_block.instructions) {
                    then_block
                        .instructions
                        .push(QbeInst::Jmp(join_label.clone()));
                }
                trace!(
                    target: "codegen",
                    "  Then block '{}': {} instructions, terminated={}",
                    then_label,
                    then_block.instructions.len(),
                    Self::is_terminated(&then_block.instructions)
                );
                self.pending_blocks.push(then_block);

                // Add jumps to nested blocks from then branch
                for label in &then_needs_jmp {
                    trace!(target: "codegen", "  Adding jump from nested block '{}' to join", label);
                    self.add_jump_to_block(label, &join_label);
                }

                // Generate else block
                let mut else_block = QbeBlock::new(&else_label);
                let else_needs_jmp =
                    self.generate_expr_to_dest(else_expr, dest, &mut else_block.instructions);
                // Only add jmp if block doesn't already have a terminator
                if !Self::is_terminated(&else_block.instructions) {
                    else_block
                        .instructions
                        .push(QbeInst::Jmp(join_label.clone()));
                }
                trace!(
                    target: "codegen",
                    "  Else block '{}': {} instructions, terminated={}",
                    else_label,
                    else_block.instructions.len(),
                    Self::is_terminated(&else_block.instructions)
                );
                self.pending_blocks.push(else_block);

                // Add jumps to nested blocks from else branch
                for label in &else_needs_jmp {
                    trace!(target: "codegen", "  Adding jump from nested block '{}' to join", label);
                    self.add_jump_to_block(label, &join_label);
                }

                trace!(
                    target: "codegen",
                    "  If complete: {} pending blocks total",
                    self.pending_blocks.len()
                );

                // Return join label - continuation goes there
                Some(join_label)
            }

            ComplexExpr::MakeBox(val) => {
                let val = self.generate_atom(val, insts);
                insts.push(QbeInst::Call {
                    dest: Some(dest.to_string()),
                    func: QbeValue::Global("scm_alloc_box".to_string()),
                    args: vec![],
                });
                insts.push(QbeInst::Call {
                    dest: None,
                    func: QbeValue::Global("scm_box_set".to_string()),
                    args: vec![
                        (QbeType::Long, QbeValue::Temp(dest.to_string())),
                        (QbeType::Long, val),
                    ],
                });
                None
            }

            ComplexExpr::ReadBox(var) => {
                let box_val = self.get_var(var);
                insts.push(QbeInst::Call {
                    dest: Some(dest.to_string()),
                    func: QbeValue::Global("scm_box_ref".to_string()),
                    args: vec![(QbeType::Long, box_val)],
                });
                None
            }

            ComplexExpr::WriteBox { box_var, value } => {
                let box_val = self.get_var(box_var);
                let val = self.generate_atom(value, insts);
                insts.push(QbeInst::Call {
                    dest: None,
                    func: QbeValue::Global("scm_box_set".to_string()),
                    args: vec![(QbeType::Long, box_val), (QbeType::Long, val)],
                });
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Copy(QbeValue::Const(tags::VALUE_VOID as i64)),
                });
                None
            }
        }
    }

    /// Generate instructions for a primitive application
    fn generate_prim_app(
        &mut self,
        op: &PrimOp,
        args: &[Atom],
        dest: &str,
        insts: &mut Vec<QbeInst>,
    ) {
        let impl_kind = get_primitive_impl(op);

        match impl_kind {
            PrimImpl::Inline(inline_op) => {
                self.generate_inline_op(&inline_op, args, dest, insts);
            }
            PrimImpl::RuntimeCall(name) => {
                let arg_vals = self.generate_args(args, insts);

                insts.push(QbeInst::Call {
                    dest: Some(dest.to_string()),
                    func: QbeValue::Global(name.to_string()),
                    args: arg_vals,
                });
            }
        }
    }

    /// Generate inline operations
    fn generate_inline_op(
        &mut self,
        op: &InlineOp,
        args: &[Atom],
        dest: &str,
        insts: &mut Vec<QbeInst>,
    ) {
        match op {
            InlineOp::Identity => {
                // Just copy the value
                let val = self.generate_atom(&args[0], insts);
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Copy(val),
                });
            }

            InlineOp::Add => {
                // Fixnum addition: untag, add, retag
                // ((a >> 3) + (b >> 3)) << 3
                // Simplified: a + b - (one tag) because both have TAG_FIXNUM=0
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Add(a, b),
                });
            }

            InlineOp::Sub => {
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Sub(a, b),
                });
            }

            InlineOp::Mul => {
                // Multiplication: need to untag, multiply, retag
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);

                // Untag a
                let a_untagged = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: a_untagged.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Sar(a, QbeValue::Const(tags::TAG_BITS as i64)),
                });

                // Multiply (b already has the tag shift built in)
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Mul(QbeValue::Temp(a_untagged), b),
                });
            }

            InlineOp::Div => {
                // Division: untag both, divide, retag
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);

                let a_untagged = self.fresh_temp();
                let b_untagged = self.fresh_temp();
                let result_untagged = self.fresh_temp();

                insts.push(QbeInst::Assign {
                    dest: a_untagged.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Sar(a, QbeValue::Const(tags::TAG_BITS as i64)),
                });
                insts.push(QbeInst::Assign {
                    dest: b_untagged.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Sar(b, QbeValue::Const(tags::TAG_BITS as i64)),
                });
                insts.push(QbeInst::Assign {
                    dest: result_untagged.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Div(QbeValue::Temp(a_untagged), QbeValue::Temp(b_untagged)),
                });
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Shl(
                        QbeValue::Temp(result_untagged),
                        QbeValue::Const(tags::TAG_BITS as i64),
                    ),
                });
            }

            InlineOp::Mod => {
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);

                let a_untagged = self.fresh_temp();
                let b_untagged = self.fresh_temp();
                let result_untagged = self.fresh_temp();

                insts.push(QbeInst::Assign {
                    dest: a_untagged.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Sar(a, QbeValue::Const(tags::TAG_BITS as i64)),
                });
                insts.push(QbeInst::Assign {
                    dest: b_untagged.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Sar(b, QbeValue::Const(tags::TAG_BITS as i64)),
                });
                insts.push(QbeInst::Assign {
                    dest: result_untagged.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Rem(QbeValue::Temp(a_untagged), QbeValue::Temp(b_untagged)),
                });
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Shl(
                        QbeValue::Temp(result_untagged),
                        QbeValue::Const(tags::TAG_BITS as i64),
                    ),
                });
            }

            InlineOp::NumEq => {
                // Compare two tagged fixnums directly (tags are same)
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);

                let eq = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: eq.clone(),
                    ty: QbeType::Word,
                    op: QbeOp::Ceql(a, b),
                });

                // Convert to boolean: eq ? #t : #f
                let true_val = QbeValue::Const(tags::VALUE_TRUE as i64);
                let false_val = QbeValue::Const(tags::VALUE_FALSE as i64);

                // Use select or branch
                // QBE doesn't have select, so we use: result = false + (true - false) * eq
                let diff = self.fresh_temp();
                let scaled = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: diff.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Sub(true_val.clone(), false_val.clone()),
                });

                // Zero-extend eq to long
                let eq_long = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: eq_long.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Extuw(QbeValue::Temp(eq)),
                });

                insts.push(QbeInst::Assign {
                    dest: scaled.clone(),
                    ty: QbeType::Long,
                    op: QbeOp::Mul(QbeValue::Temp(diff), QbeValue::Temp(eq_long)),
                });
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Add(false_val, QbeValue::Temp(scaled)),
                });
            }

            InlineOp::Lt => {
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);

                let cmp = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: cmp.clone(),
                    ty: QbeType::Word,
                    op: QbeOp::Csltl(a, b),
                });

                // Convert to boolean
                self.generate_bool_from_cmp(&cmp, dest, insts);
            }

            InlineOp::Gt => {
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);

                let cmp = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: cmp.clone(),
                    ty: QbeType::Word,
                    op: QbeOp::Csgtl(a, b),
                });

                self.generate_bool_from_cmp(&cmp, dest, insts);
            }

            InlineOp::Le => {
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);

                let cmp = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: cmp.clone(),
                    ty: QbeType::Word,
                    op: QbeOp::Cslel(a, b),
                });

                self.generate_bool_from_cmp(&cmp, dest, insts);
            }

            InlineOp::Ge => {
                let a = self.generate_atom(&args[0], insts);
                let b = self.generate_atom(&args[1], insts);

                let cmp = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: cmp.clone(),
                    ty: QbeType::Word,
                    op: QbeOp::Csgel(a, b),
                });

                self.generate_bool_from_cmp(&cmp, dest, insts);
            }

            InlineOp::Not => {
                // (not x) = #t if x is #f, else #f
                let a = self.generate_atom(&args[0], insts);

                let is_false = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: is_false.clone(),
                    ty: QbeType::Word,
                    op: QbeOp::Ceql(a, QbeValue::Const(tags::VALUE_FALSE as i64)),
                });

                self.generate_bool_from_cmp(&is_false, dest, insts);
            }

            InlineOp::IsNull => {
                // (null? x) = #t if x is nil
                let a = self.generate_atom(&args[0], insts);

                let is_nil = self.fresh_temp();
                insts.push(QbeInst::Assign {
                    dest: is_nil.clone(),
                    ty: QbeType::Word,
                    op: QbeOp::Ceql(a, QbeValue::Const(tags::VALUE_NIL as i64)),
                });

                self.generate_bool_from_cmp(&is_nil, dest, insts);
            }

            InlineOp::List => {
                // (list a b c) => cons(a, cons(b, cons(c, nil)))
                // Build from end
                let mut cur = QbeValue::Const(tags::VALUE_NIL as i64);
                for arg in args.iter().rev() {
                    let elem = self.generate_atom(arg, insts);
                    let tmp = self.fresh_temp();
                    insts.push(QbeInst::Call {
                        dest: Some(tmp.clone()),
                        func: QbeValue::Global("scm_cons".to_string()),
                        args: vec![(QbeType::Long, elem), (QbeType::Long, cur)],
                    });
                    cur = QbeValue::Temp(tmp);
                }
                insts.push(QbeInst::Assign {
                    dest: dest.to_string(),
                    ty: QbeType::Long,
                    op: QbeOp::Copy(cur),
                });
            }
        }
    }

    /// Generate boolean (#t/#f) from a comparison result (0/1)
    fn generate_bool_from_cmp(&mut self, cmp: &str, dest: &str, insts: &mut Vec<QbeInst>) {
        let true_val = QbeValue::Const(tags::VALUE_TRUE as i64);
        let false_val = QbeValue::Const(tags::VALUE_FALSE as i64);

        let diff = self.fresh_temp();
        let cmp_long = self.fresh_temp();
        let scaled = self.fresh_temp();

        insts.push(QbeInst::Assign {
            dest: diff.clone(),
            ty: QbeType::Long,
            op: QbeOp::Sub(true_val.clone(), false_val.clone()),
        });

        insts.push(QbeInst::Assign {
            dest: cmp_long.clone(),
            ty: QbeType::Long,
            op: QbeOp::Extuw(QbeValue::Temp(cmp.to_string())),
        });

        insts.push(QbeInst::Assign {
            dest: scaled.clone(),
            ty: QbeType::Long,
            op: QbeOp::Mul(QbeValue::Temp(diff), QbeValue::Temp(cmp_long)),
        });

        insts.push(QbeInst::Assign {
            dest: dest.to_string(),
            ty: QbeType::Long,
            op: QbeOp::Add(false_val, QbeValue::Temp(scaled)),
        });
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::anf::AnfTransformer;
    use crate::compiler::closure::ClosureConverter;
    use crate::parser::read_all;

    fn compile(s: &str) -> QbeModule {
        let exprs = read_all(s).unwrap();
        let mut anf = AnfTransformer::new();
        let program = anf.transform_program(exprs).unwrap();
        let mut closure = ClosureConverter::new();
        let program = closure.convert(program);
        let mut codegen = CodeGenerator::new();
        codegen.generate(&program)
    }

    #[test]
    fn test_simple_constant() {
        let module = compile("42");
        // Should generate main function
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "main");
    }

    #[test]
    fn test_arithmetic() {
        let module = compile("(+ 1 2)");
        // Should generate main function with call to scm_add
        assert_eq!(module.functions.len(), 1);
        assert_eq!(module.functions[0].name, "main");
    }

    #[test]
    fn test_lambda() {
        let module = compile("(lambda (x) x)");
        // Should generate lambda function + main
        assert_eq!(module.functions.len(), 2);
    }

    #[test]
    fn test_string_literal() {
        let module = compile("\"hello\"");
        // Should generate data section entry for string
        assert!(!module.data.is_empty());
        assert_eq!(module.functions.len(), 1);
    }

    #[test]
    fn test_function_call() {
        let module = compile("((lambda (x) x) 5)");
        // Should generate lambda function + main with call
        assert_eq!(module.functions.len(), 2);
    }

    #[test]
    fn test_debug_lambda_call() {
        let module = compile(
            r#"
            (define foo (lambda (x) (+ x 1)))
            (display (foo 1))
        "#,
        );
        println!("=== QBE IR for lambda call ===");
        println!("{}", module.to_string());
    }

    #[test]
    fn test_debug_quote() {
        let exprs = read_all("(display '(1 2 3))").unwrap();
        let mut anf = AnfTransformer::new();
        let program = anf.transform_program(exprs).unwrap();
        println!("=== ANF for quote ===");
        println!("Entry: {:?}", program.entry);
    }

    #[test]
    fn test_debug_recursion() {
        let module = compile(
            r#"
            (define f (lambda (n) (if (= n 0) 1 (f (- n 1)))))
            (f 3)
        "#,
        );
        println!("=== QBE IR for recursion ===");
        println!("{}", module.to_string());
    }
}
