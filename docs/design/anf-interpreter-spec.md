# ANF-Based Interpreter Redesign — Implementation Guide

## 1. Overview

### 1.1 Problem with the current interpreter

`core/src/eval.rs` walks raw `Value` S-expressions directly and re-implements
special-form desugaring independently of `core/src/compiler/anf.rs`. The two
implementations have already drifted (`and`/`or` in `env.rs` are eager 2-arg
builtins; `anf.rs::transform_and`/`transform_or` are variadic, short-circuiting
special forms) and will keep drifting every time a form is added or changed in
one place and not the other.

### 1.2 Design goal

Make the parser → ANF lowering (`AnfTransformer::transform_program`) the single
place that defines what Scheme special forms *mean*. Both backends consume the
same `AnfProgram`:

```
Source (.scm)
   │
   ├─ Parser ──> Value AST
   │
   └─ AnfTransformer::transform_program ──> AnfProgram
         │                                     │
         ├─ closure.rs + codegen.rs ──> QBE    └─ NEW: eval/anf_interp.rs ──> Value
         (compiled backend, unchanged)              (interpreted backend, this spec)
```

The interpreter becomes a small tree-walker over `AnfExpr`/`ComplexExpr`/`Atom`
(5 + 9 + 9 node kinds) instead of over the full `Value`/`SchemeList` grammar
plus a dozen hand-written `eval_value_*` special-form functions. Any future
special form (or `and`/`or`-style bug fix) is written once, in
`AnfTransformer`, and both backends get it automatically.

### 1.3 Non-goals

- No suspension/resume, no serialization, no step budgets (dropped earlier in
  this design pass).
- No closure-conversion reuse from `closure.rs` — that pass computes explicit
  free-variable capture lists for a register-based native calling convention.
  The interpreter doesn't need it (see §4).
- No first-class continuations, `dynamic-wind`, or resumable exceptions — the
  compiled runtime doesn't have them either (`packages/runtime/src/exceptions.rs`
  is escape-only setjmp/longjmp), so the interpreter shouldn't invent them.
- No FFI execution of compiled `.a` extensions from interpret mode. Extension
  `.a` files speak the compiled runtime's tagged-`u64` ABI
  (`core/src/tags.rs`); the interpreter's `Value` is a Rust enum
  (`core/src/types/value.rs`). Bridging those is a separate future project.
  This spec only guarantees the interpreter has *one* dispatch seam
  (`PrimOp::ExtCall`, §7) where such a bridge would plug in.

## 2. Module layout

```
core/src/eval/
  mod.rs        — pub fn eval_source(src: &str, session: &mut Session) -> Result<Value, String>
                  (new top-level entry point; replaces direct use of eval_value)
  session.rs    — Session { globals: HashMap<String, Value>, functions: HashMap<String, Rc<FunctionDef>>,
                             strings: Vec<String>, symbols: Vec<String>, ext_table: HashMap<String, ProcedureFn> }
                  (functions keyed by Rc, not by value — see §6.1 review finding)
  interp.rs     — eval_anf, eval_complex, eval_atom, apply_closure  (the actual machine)
  value_bridge.rs — Atom -> Value, Value -> Atom (for closures/globals crossing the ANF boundary)
  primitives.rs — the native (non-ANF) implementations backing PrimOp, keyed by PrimOp variant
```

`core/src/eval.rs` (today's file) is deleted once callers migrate. `core/src/env.rs`
shrinks to just `std_const_exp()`-equivalent global seeding, if anything remains there at all
(most of its content — the `+`, `car`, `cons`, etc. — moves into `primitives.rs` as direct
`PrimOp` implementations, since ANF already classifies these as `PrimOp`, not as
`Value`-symbol-keyed procedures).

`core/src/bin/cli/common.rs` changes from:
```rust
match read_all(&buffer) {
    Ok(exprs) => for expr in exprs { eval_value(expr, &env, &mut defs)?; }
}
```
to:
```rust
let mut session = Session::new();
match read_all(&buffer) {
    Ok(exprs) => session.eval_program(exprs)?,   // parses once, transforms once, runs once
}
```
This is a real behavior change worth calling out: today each top-level form is parsed *and
evaluated* independently, so `(define x 1)` followed later by `(display x)` works because both
share the same `defs` map across separate `eval_value` calls. Under ANF, `transform_program`
expects the *whole* program's forms up front to build one `AnfProgram` (it needs to see all
`define`s to lift lambdas and resolve `entry`). REPL mode (one form at a time, interactively) is
handled differently — see §8.

## 3. Data model recap (already exists, unchanged)

Reusing `core/src/compiler/anf.rs` types as-is:

- `Atom` — `Var(VarId) | Int | Float | Bool | Char | String(idx) | Symbol(idx) | Nil | Void`
- `ComplexExpr` — `PrimApp{op, args} | App{func,args} | TailApp{func,args} | MakeClosure{label,captures} | ClosureRef{closure,index} | If{cond,then,else} | MakeBox(Atom) | ReadBox(VarId) | WriteBox{box_var,value}`
- `AnfExpr` — `Return(Atom) | Let{var,value,body} | Seq{effect,body} | TailCall{func,args} | Halt(Atom)`
- `FunctionDef` — `{label, source_name, params: Vec<VarId>, has_env, body: AnfExpr, free_vars}`
- `AnfProgram` — `{functions: Vec<FunctionDef>, entry: AnfExpr, strings: Vec<String>, symbols: Vec<String>}`

No changes required to these types for the interpreter (the compiler continues to own them).
One addition needed for §7: `PrimOp::ExtCall(String)` per the extensions spec — additive, doesn't
affect the interpreter's other variants.

## 4. Why closure conversion (`closure.rs`) is skipped

> Types below are shown with `label: String` for readability; the final types are `Symbol`-keyed
> per §12 (identifier interning) — see §12.5 for the as-built field types.

`closure.rs` rewrites `MakeClosure{label, captures}` so `captures` is an explicit, ordered list
of free variables, and sets `FunctionDef.has_env`/`free_vars` — because QBE-generated functions
are plain C-ABI functions with no lexical environment; captured variables must be materialized
into an explicit closure struct (`Closure.captures[i]`, per `runtime_types` memory) and indexed
by position.

The interpreter doesn't have that constraint — it can represent a closure as:

```rust
pub struct Closure {
    pub label: String,          // key into Session.functions
    pub env: Rc<Env>,           // the frame in scope at creation time, plus its parent chain
}
```

i.e., skip explicit free-variable capture entirely and close over the whole lexical environment,
exactly like today's `Procedure { params, body, env: defs.clone() }` — except cheap (`Rc` clone,
not `HashMap` clone) because of the environment representation in §5. `AnfTransformer`'s raw
(pre-closure-conversion) output already has correct `VarId` references inside lambda bodies —
`captures` is simply unused/ignored by the interpreter. This is the single biggest simplification
this redesign gets from reusing ANF: **the interpreter needs zero new logic for variable capture.**

## 5. Environment representation

> `VarId` below is shown wrapping `String`, matching today's `anf.rs`; after §12,
> `bindings: Vec<(VarId, Value)>` becomes `Vec<(Symbol, Value)>` and comparisons/hashes are on
> `Symbol` (`Copy`, no per-binding string clone) — see §12.5.

ANF's `Let{var, value, body}` introduces exactly one binding at a time and immediately nests the
continuation inside it — this is naturally a persistent linked environment, not a
clone-the-whole-map-per-call structure:

```rust
pub enum Env {
    Empty,
    Frame { bindings: Vec<(VarId, Value)>, parent: Rc<Env> },
}

impl Env {
    fn lookup(&self, var: &VarId) -> Option<&Value> {
        match self {
            Env::Empty => None,
            Env::Frame { bindings, parent } =>
                bindings.iter().rev().find(|(v, _)| v == var).map(|(_, val)| val)
                    .or_else(|| parent.lookup(var)),
        }
    }
    /// One binding (Let/Seq) or many bound together (a call's params) — same frame shape either way.
    fn extend(self: &Rc<Self>, bindings: Vec<(VarId, Value)>) -> Rc<Env> {
        Rc::new(Env::Frame { bindings, parent: self.clone() })
    }
}
```

A frame batches whatever bindings are introduced *together*: one `(var, value)` pair for each
`Let`, all of a closure's params in a single frame at call time. This avoids allocating one `Rc`
per parameter on every call while keeping the same parent-chain/capture-by-reference semantics —
a closure captures a single `Rc<Env>` pointing at the frame in scope where it was created (§4),
and only that frame's ancestor chain is kept alive, bounded by lexical nesting depth, not by
program size or by "the whole environment" as a bulk structure.

- Function application: build one new frame binding all of `param[i] -> arg_value[i]`, chained
  onto the closure's captured `env` (**not** onto the caller's env — this is what makes it lexical
  scoping rather than the current interpreter's dynamic-ish `local_defs.extend(outer_defs)`
  hybrid in `apply_value_procedure`).
- `Let`/`Seq` in a function body: extend the current env by one single-binding frame, recursing
  into `body` with the new `Rc<Env>`. `Rc::clone` is O(1); no map is ever fully copied.
- Lookup is O(depth of lexical nesting in that function) frames, each a short linear scan over its
  own `bindings` — in practice small (ANF's per-function `Let` chains rarely exceed a few dozen
  frames). A future optimization, if lookup ever shows up as a bottleneck, is a lexical-addressing
  pass over `VarId`s (SICP-style: resolve each `Var` to a static `(frame_depth, slot)` pair ahead
  of time, turning lookup into O(depth) array indexing with no string comparison) — real, separate
  engineering effort, not part of this pass.

Global top-level `define`s are **not** part of this chain — they live in `Session.globals:
HashMap<String, Value>` (mutable, shared, looked up when `Env::lookup` bottoms out at
`Env::Empty` and the variable is a top-level name rather than a local). This mirrors how `set!`
in R7RS is only specified for already-bound variables and matches today's top-level `defs`
threading through the REPL.

## 6. Evaluation algorithm

```rust
pub fn eval_anf(expr: &AnfExpr, env: &Rc<Env>, session: &mut Session) -> Result<Value, String> {
    match expr {
        AnfExpr::Return(atom) => eval_atom(atom, env, session),

        AnfExpr::Let { var, value, body } => {
            let v = eval_complex(value, env, session)?;
            let env2 = env.extend(var.clone(), v);
            eval_anf(body, &env2, session)   // NOT a tail call in Rust terms (body is boxed AnfExpr)
        }

        AnfExpr::Seq { effect, body } => {
            eval_complex(effect, env, session)?;   // discard result, e.g. Display/Newline/WriteBox
            eval_anf(body, env, session)
        }

        AnfExpr::TailCall { func, args } => {
            // See §6.1 — trampolined, not recursed.
            apply_tail(func, args, env, session)
        }

        AnfExpr::Halt(atom) => Err(format!("halt: {:?}", eval_atom(atom, env, session)?)),
    }
}
```

`eval_complex` handles the 9 `ComplexExpr` variants: arithmetic/list `PrimOp`s call directly into
`primitives.rs`; `If` recurses into `eval_anf` on whichever branch; `MakeClosure` builds a
`Closure{label, env: env.clone()}` (§4); `ClosureRef`/`MakeBox`/`ReadBox`/`WriteBox` are direct,
small operations on `Value` (boxes represented as `Value::Box(Rc<RefCell<Value>>)` — a **new**
`Value` variant needed since today's interpreter has no mutable-cell representation; the compiled
runtime already has one, `Box` in `runtime_types`, so this is filling a gap, not inventing
divergence). `App` (non-tail) evaluates to a `Value` by calling `apply_closure` and returning
normally (native Rust recursion — acceptable, since tail position is exactly what ANF's
`TailApp`/`TailCall` distinction exists to protect).

### 6.1 Tail calls

This is the one place the interpreter *does* need an explicit loop instead of straightforward
recursion, precisely because `AnfExpr::TailCall`/`ComplexExpr::TailApp` exist to mark tail
position — ignoring that distinction would silently reintroduce unbounded Rust-stack growth for
self-recursive loops (the exact problem the current interpreter papers over with a 1 GB-stack
thread in `common.rs`).

```rust
fn apply_tail(func: &Atom, args: &[Atom], env: &Rc<Env>, session: &mut Session) -> Result<Value, String> {
    let mut closure = eval_atom(func, env, session)?.expect_closure()?;
    let mut arg_values: Vec<Value> = args.iter().map(|a| eval_atom(a, env, session)).collect::<Result<_,_>>()?;

    loop {
        let def: Rc<FunctionDef> = session.functions.get(&closure.label).ok_or("undefined function")?.clone();
        let bindings = def.params.iter().cloned().zip(arg_values.into_iter()).collect();
        let call_env = closure.env.extend(bindings);
        match step_body(&def.body, &call_env, session)? {
            Step::Value(v) => return Ok(v),
            Step::TailInto { next_closure, next_args } => {
                closure = next_closure;
                arg_values = next_args;
                // loop again instead of recursing — this is the trampoline
            }
        }
    }
}
```

**Review finding (folded in):** `session.functions` must be `HashMap<String, Rc<FunctionDef>>`, not
`HashMap<String, FunctionDef>`. `FunctionDef` owns a full `AnfExpr` body tree (`anf.rs:177-192`);
cloning it by value on *every trampoline iteration* — the exact hot path this loop exists to make
cheap — would deep-copy that tree per step instead of bumping a pointer. `FunctionDef`s are
immutable program data once `AnfProgram` is produced, so many call sites sharing one `Rc` is the
correct ownership model, not a workaround. `Session.functions` is populated once with
`Rc::new(fn_def)` per entry when a program (or REPL line, §8) is loaded.

`step_body` runs `eval_anf`-like logic but, on hitting `TailCall`/`TailApp`, returns
`Step::TailInto` instead of recursing — i.e., it's the same `eval_anf` with the `TailCall` arm
changed to *return* rather than *call*. (In the actual implementation this is most naturally one
function with an `in_tail: bool`-free design: `eval_anf` always returns `Value`, and only the
outermost `apply_tail` loop unwinds trampoline steps — implementation detail to settle in code
review, not a semantic question.)

This gives proper O(1)-stack tail calls in the interpreter for the first time — a genuine
capability improvement, obtained as a side effect of adopting ANF rather than as separate scope.

## 7. Primitives and the extension seam

`ComplexExpr::PrimApp { op: PrimOp, args }` dispatches on the `PrimOp` enum directly — a `match`
in `primitives.rs`, one arm per variant (`Add`, `Car`, `Cons`, `Display`, ...), operating on
already-evaluated `Value` arguments. This *is* the natural extension seam described in the
`library-extensions-spec.md`:

```rust
// core/src/compiler/anf.rs (additive, per extensions spec §6.2)
pub enum PrimOp {
    // ... existing ~30 variants ...
    ExtCall(String),   // C symbol name, e.g. "scm_net_tcp_connect"
}
```

```rust
// core/src/eval/primitives.rs
pub fn call_prim(op: &PrimOp, args: Vec<Value>, session: &mut Session) -> Result<Value, String> {
    check_arity(op, args.len())?;   // single generic check, see below — arms no longer self-check
    match op {
        PrimOp::Add => numeric_binop(args, Number::add),
        // ... existing arms, ported 1:1 from today's env.rs bodies, arity checks REMOVED ...
        PrimOp::ExtCall(symbol) => session.ext_table.call(symbol, args),
    }
}
```

**Review finding (folded in):** the original plan — port each `env.rs` builtin body verbatim,
including its own inline `if args.len() < 2 { return Err(...) }` — duplicates arity information
the compiler already maintains centrally in `RuntimeFn`/`RUNTIME_FUNCTIONS`
(`core/src/compiler/primitives.rs:102-376`, e.g. `scm_cons` arity 2, `scm_car` arity 1), reachable
via the existing `get_runtime_fn(name)`. Declaring arity a second time, by hand, per `PrimOp` arm
is exactly the kind of duplication that let `and`/`or` drift between `env.rs` and `anf.rs` in the
first place (§1.1). Instead:

```rust
fn primop_arity(op: &PrimOp) -> Arity {
    match get_primitive_impl(op) {
        PrimImpl::RuntimeCall(name) => Arity::Exact(get_runtime_fn(name).unwrap().arity),
        PrimImpl::Inline(_) => match op {
            PrimOp::Not | PrimOp::IsNull => Arity::Exact(1),
            PrimOp::List => Arity::AtLeast(0),
            _ => Arity::Exact(2),   // Add/Sub/Mul/Div/Mod/NumEq/Lt/Gt/Le/Ge — all binary
        },
    }
}

fn check_arity(op: &PrimOp, n: usize) -> Result<(), String> { /* one generic check against primop_arity(op) */ }
```

This reuses `get_primitive_impl`/`get_runtime_fn` (both already `pub fn` in
`core/src/compiler/primitives.rs`) as the single source of truth for `RuntimeCall`-backed ops, and
only hardcodes the small, stable set of `Inline` ops that have no `RuntimeFn` entry. `Arity` is the
same enum from the earlier `PrimTable` draft (§ dropped in favor of this ANF-based design, but the
type is still useful here).

`session.ext_table: HashMap<String, ProcedureFn>` is exactly the registration point a future
interpret-mode extension bridge would populate — same shape as `get_primitive_impl` maps
`PrimOp::ExtCall(symbol) -> PrimImpl::RuntimeCall(symbol)` for codegen. **What populates it is
explicitly out of scope here** (§1.3) — until a marshalling shim exists, `ExtCall` primitives
simply aren't resolvable in interpret mode and should produce a clear
`"extension function '{symbol}' not available in interpreted mode"` error rather than a panic.

This also fixes the `and`/`or`/`map`/`filter` bugs from the earlier review as a consequence of
the architecture, not as patches:
- `and`/`or` are `AnfExpr` produced by `transform_and`/`transform_or`'s `if`-desugaring — the
  interpreter never sees an `and`/`or` node at all, only the `If` nodes they were rewritten into.
  There is no second implementation to drift.
- `map`/`filter` (defined in `lib/prelude.scm` per §2.5 of the extensions spec, *not* as Rust
  primitives) become ordinary Scheme closures applied via the same `apply_closure`/`apply_tail`
  path as user code — there's no bespoke Rust closure capturing (or failing to capture) an `env`
  parameter, because there's only one environment representation in the whole system (§5).

## 8. REPL mode

`transform_program` wants the whole program's `define`s up front (§2), but a REPL evaluates one
line at a time, immediately, with visible side effects. **Re-transforming the full input history
on every line and re-evaluating the resulting `entry` from scratch is not viable**: `entry` wraps
*all* forms seen so far in one `begin` (`transform_top_level`), so re-running it replays every
prior side effect — `(display "hi")` on line 1 prints again when line 2 is submitted, `set!`s
re-fire, etc. This was considered and rejected, not a fallback option.

The correct approach: keep **one long-lived `AnfTransformer` instance** for the whole REPL
session and call `transform_program` exactly once per new line, evaluating only that line's
`entry`; earlier entries are never touched again. This relies on `AnfTransformer`'s existing
per-instance state:

- `temp_counter`/`label_counter` are plain fields, never reset by `transform_program` — confirmed
  by reading the method (`core/src/compiler/anf.rs:266-277`), it only `mem::take`s `functions`,
  `strings`, `symbols`. So temp/label names never collide across lines for free.
- `functions` accumulated by earlier lines were already drained into earlier `AnfProgram`s (via
  `mem::take`); `Session.functions` (a separate, persistent `HashMap<String, FunctionDef>` owned
  by the interpreter, not the transformer) accumulates each new line's `program.functions` via
  `.extend(...)` so later lines can call lambdas defined by earlier ones.

**Blocking prerequisite found while checking this**: `transform_program` drains `self.strings`/
`self.symbols` via `mem::take` on every call, but does **not** reset `self.string_map`/
`self.symbol_map` (the dedup maps that decide whether a literal gets a new index or reuses one)
alongside them. Call it twice on one instance and a string literal repeated on line 2 that was
already interned on line 1 hits the stale `string_map` entry — returning an index into line 1's
(already-drained-and-returned) `strings` vec, not line 2's fresh one. `Atom::String(idx)` in line
2's `AnfProgram` would then be wrong. This is latent today only because nothing calls
`transform_program` twice on one instance yet. **Required fix in `AnfTransformer` before REPL
mode can use this approach**: either reset `string_map`/`symbol_map` together with the vecs each
call (simplest — line-local interning, `Session` keeps its own cumulative
`strings: Vec<String>`/`symbols: Vec<String>` and remaps indices on `.extend`), or stop draining
the vecs at all and have `transform_program` return cumulative snapshots instead of taking them.
Recommend the former — it keeps `AnfTransformer` itself simple and pushes accumulation to
`Session`, which already needs to accumulate `functions` the same way.

## 9. Rollout plan

0. **Prerequisite, lands first, compiler-only**: identifier interning (§12) — `core/src/interner.rs`
   (`Symbol`/`Interner`), `VarId`/`FunctionDef.label` switch to `Symbol`, `AnfProgram` gains
   `interner: Interner`, remove `impl Display for VarId`, update the 11 `VarId::new`/`VarId::temp`
   call sites across `anf.rs`/`codegen.rs`. Validated entirely against the existing compiler test
   suite — no interpreter code exists yet to depend on it. Steps 2+ below are written against
   `Symbol`-keyed types directly (§12.5), not against the earlier `String`-based sketches in §4/§5.
1. Add `Value::Box(Rc<RefCell<Value>>)` variant (needed for `MakeBox`/`ReadBox`/`WriteBox`,
   i.e. `set!`/mutable captured variables) — currently missing from `core/src/types/value.rs`.
2. `core/src/eval/session.rs` + `Env` (§5, `Symbol`-keyed per §12.5) + `Closure` (§4, `Symbol`-keyed
   per §12.5) — no behavior yet, just types.
3. `core/src/eval/primitives.rs` — port every `env.rs` builtin body to a `PrimOp` match arm, plus
   `primop_arity`/`check_arity` (§7 review finding) as the single generic arity check instead of
   per-arm checks (mechanical port otherwise; arithmetic/comparison/pairs/predicates/display
   unchanged, `and`/`or` deleted entirely since they no longer exist as primitives).
4. `core/src/eval/interp.rs` — `eval_anf`/`eval_complex`/`eval_atom`/`apply_closure`/`apply_tail`
   per §6, with `Session.functions: HashMap<Symbol, Rc<FunctionDef>>` (§6.1/§12.5 review finding —
   clone the `Rc`, not the `FunctionDef`). Unit-test directly against hand-built `AnfExpr` fixtures
   (no parser needed yet).
5. Wire `Session::eval_program(exprs: Vec<Value>) -> Result<Value, String>` = `parser output ->
   AnfTransformer::transform_program -> eval_anf(program.entry, ...)`, running the *existing*
   `AnfTransformer` (as modified by step 0).
6. Port `core/tests` from calling `eval_value` to calling `Session::eval_program`; this is where
   parity gaps (if any remain) surface as test failures.
7. Fix the `AnfTransformer` `string_map`/`symbol_map` drain gap (§8) — required before step 8's
   `repl()` can reuse one transformer instance across lines; does not affect single-shot
   `parse_and_run_scheme` or the compiled backend (neither calls `transform_program` more than
   once per instance today), so this is additive and low-risk.
8. Update `core/src/bin/cli/common.rs`: `parse_and_run_scheme` per §2 (one `Session`, one
   `transform_program` call for the whole file); `repl` per §8 (one long-lived `AnfTransformer` +
   `Session`, one `transform_program` call per line). Remove the 1 GB stack-size thread spawn in
   `parse_and_run_scheme` — no longer needed once tail calls are O(1) stack (non-tail recursion
   depth is bounded by source-level nesting, not loop iteration count).
9. Delete `core/src/eval.rs` and the now-dead builtin bodies in `core/src/env.rs`.

## 10. Testing

- Fixture-based `eval_anf` unit tests bypassing the parser (step 4 above) — fast, precise,
  isolate interpreter bugs from ANF-transform bugs.
- End-to-end tests reusing today's `core/tests` Scheme-source-string test cases unchanged, now
  routed through `Session::eval_program` — this is the regression net proving the rewrite is
  behavior-preserving except for the deliberate fixes (`and`/`or` short-circuiting/variadic,
  proper tail calls).
- New tests specifically for what changes: `(and #f (car '()))` doesn't error (short-circuits
  before evaluating `(car '())`); a self-recursive tail loop of >10<sup>6</sup> iterations
  completes without stack overflow and without the dedicated big-stack thread.
- Extension seam: a test registering a fake `session.ext_table` entry and confirming
  `PrimOp::ExtCall` dispatches to it — validates §7 without needing real FFI.

## 11. Decisions from review

1. **`Env::lookup` falls back to `Session.globals`** (§5) — no new `Atom::Global` variant. Not
   worth pushing into the shared frontend at this stage.
2. **Frame-chain capture stands, no `closure.rs` reuse** (§4/§5) — reframed during review: a
   closure capturing `Rc<Env>` only keeps alive the ancestor frame chain it's actually nested
   inside (bounded by lexical depth), not "the whole environment" as a bulk structure, so the
   retention concern from the first draft was overstated. Frames batch bindings introduced
   together (§5) to keep per-call allocation to one `Rc` instead of one per parameter. Real
   free-variable analysis (i.e. reusing `closure.rs`) or lexical-addressed (de Bruijn) slots
   remain available as later optimizations if lookup or retention ever show up as measured
   problems — not needed for v1.
3. **REPL re-transforming full history on every line is rejected**, not deferred — it replays
   side effects (§8). Replaced with: one long-lived `AnfTransformer` + `Session` per REPL process,
   one `transform_program` call per line, evaluating only that line's `entry`. This has a hard
   prerequisite: `AnfTransformer`'s `string_map`/`symbol_map` must be reset alongside
   `strings`/`symbols` on each `transform_program` call (§8) — currently they aren't, which is
   harmless today (nothing calls it twice on one instance) but would silently produce wrong
   `Atom::String`/`Atom::Symbol` indices on a REPL's second line. Tracked as rollout step 7.

Decision on the item raised in review-but-deferred to discuss further (§4/§5's "no reusable
pattern exists" verdict on plain-`String` `VarId`): **accepted for refactor — identifier
interning, §12.**

## 12. Identifier interning (cross-cutting refactor: `anf.rs`, `closure.rs`, `codegen.rs`)

### 12.1 What changes and why

Today `VarId` (`anf.rs:9-11`) is `pub struct VarId(pub String)`, and `FunctionDef.label`
(`anf.rs:177-192`) is a plain `String`. Every environment lookup compares `VarId`s by string
equality, and `Session.functions` (§6.1) would hash a `String` on every call/tail-loop iteration.
`VarId::clone()` also heap-allocates (cloning the inner `String`) every time a binding is
recorded — which happens on every `Env::extend` call (§5), i.e. every `Let` and every function
call.

Confirmed via Serena that this isn't interpreter-local — `VarId::new`/`VarId::temp` are already
called from both `anf.rs` and `codegen.rs` (11 call sites total), and codegen synthesizes its own
ad hoc identifiers post-transform (`self.var_map.insert(VarId::new("__env"), ...)` at
`codegen.rs:191`) — so `VarId` is a shared type actively used by three pipeline stages, not
something the interpreter can privately swap out underneath them. This has to be one refactor to
`anf.rs`'s core type, validated against the compiler's existing test suite, not an interpreter-only
change.

### 12.2 Design

New shared module `core/src/interner.rs` (top-level — used by `compiler/` and the new `eval/`,
so it doesn't belong under either):

```rust
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Symbol(u32);

#[derive(Default)]
pub struct Interner {
    strings: Vec<String>,
    map: HashMap<String, Symbol>,   // dedup: repeated names (e.g. "x" in many lambdas) share one Symbol
}

impl Interner {
    pub fn intern(&mut self, s: &str) -> Symbol {
        if let Some(&sym) = self.map.get(s) { return sym; }
        let sym = Symbol(self.strings.len() as u32);
        self.strings.push(s.to_string());
        self.map.insert(s.to_string(), sym);
        sym
    }
    pub fn resolve(&self, sym: Symbol) -> &str { &self.strings[sym.0 as usize] }
}
```

This is deliberately the same shape as the runtime's existing `SymbolTable` (per `runtime_types`
memory: `symbols: Vec<String>`, find-or-create by linear/hashed lookup) — consistent style, not a
new idea introduced from outside the codebase. **One explicit difference worth calling out so it
isn't copied by accident**: the runtime's table is a `static mut SYMBOL_TABLE` (a global,
`unsafe`-accessed singleton, appropriate there because compiled Scheme programs need process-wide
symbol identity for `eq?`/`symbol->string` at arbitrary points with no natural "owner" to thread
it through). `Interner` here has a natural owner — it's created once by `AnfTransformer` and
threaded through the pipeline by value (§12.3) — so it should **not** be a global/`static mut`.
Don't replicate that pattern here; there's no need for it and it would reintroduce exactly the
kind of unsafe global-mutable-state smell `m15-anti-pattern` flags.

`VarId` and `FunctionDef.label` change to wrap `Symbol`:
```rust
pub struct VarId(pub Symbol);       // was String
pub struct FunctionDef { pub label: Symbol, /* ... */ }   // was String
```
`ComplexExpr::MakeClosure { label: Symbol, .. }` and the interpreter's `Closure { label: Symbol,
env: Rc<Env> }` (§4, revised) follow. `VarId::new(name: &str, interner: &mut Interner) -> VarId`
and `VarId::temp(n: u64, interner: &mut Interner) -> VarId` both now take the interner (format the
temp name, then intern it — `__t0`/`__t1`/... temps are still deduped-by-construction since each
is textually unique, but the call now goes through the same `intern` path uniformly rather than
having a separate un-interned code path for temps vs. source names).

`impl Display for VarId` (`anf.rs:31-35`, currently `write!(f, "{}", self.0)`) can no longer work —
`Display::fmt` only has `&self`, no way to reach an external `Interner`. This is removed, not
worked around (e.g. no cached `Option<String>` kept alongside `Symbol` just to preserve `Display`
— that would defeat the point, since it's exactly the allocation-per-`VarId` this refactor removes).
Call sites that need text (error messages, QBE label emission, backtraces via `source_name`) resolve
explicitly: `interner.resolve(var.0)`. This isn't a new awkwardness — `Atom::String(usize)` and
`Atom::Symbol(usize)` already require the surrounding `AnfProgram`'s `strings`/`symbols` table to
print as text; `VarId` just joins that existing "needs external context to render" convention
instead of being the odd one out with its own private `Display` impl.

### 12.3 Threading the interner through the pipeline

`AnfProgram` gains a field: `pub interner: Interner`. Each pipeline stage already takes/returns
`AnfProgram` by value and mutates fields in place (per the extensions spec's pipeline diagram —
closure conversion fills in `has_env`/`captures`, for instance), so the interner flows through for
free with no new plumbing: `AnfTransformer` owns it during `transform_program`, hands it off inside
the returned `AnfProgram`, `closure.rs` and `codegen.rs` receive it already populated and can
`.intern()` any new synthesized names (like codegen's `"__env"`) into the *same* table rather than
inventing a second one.

### 12.4 Two different reset policies — do not conflate them

§8 already decided that `AnfTransformer.string_map`/`symbol_map` (dedup for **literal**
`Atom::String`/`Atom::Symbol` values) must be reset alongside `strings`/`symbols` on every
`transform_program` call, so each REPL line gets line-local literal tables that `Session`
re-accumulates with remapped indices.

The identifier `Interner` from this section is the opposite: it must **never** be reset or drained
across `transform_program` calls on the same `AnfTransformer` instance. If it were reset per REPL
line, `(define x 1)` on line 1 and `(display x)` on line 2 would intern `"x"` twice, get two
different `Symbol`s, and `Session.globals: HashMap<Symbol, Value>` (§12.5) would look up the wrong
key — `x` would appear undefined on line 2. Concretely: `Interner` is a plain field on
`AnfTransformer` that `transform_program` must never `mem::take` (unlike `strings`/`symbols`,
which it explicitly should, per §8). This only matters for REPL mode; single-shot compilation and
`parse_and_run_scheme` call `transform_program` once, so the distinction is invisible there.

### 12.5 Effect on the interpreter design (§4–§7)

- `Env::Frame { bindings: Vec<(Symbol, Value)>, parent: Rc<Env> }` — `Symbol` is `Copy`, so
  `Env::extend` no longer clones a `String` per binding (compounds the §11-item-1 fix: cloning got
  cheaper *and* less frequent).
- `Closure { label: Symbol, env: Rc<Env> }`; `Session.functions: HashMap<Symbol, Rc<FunctionDef>>`
  — lookups hash a `u32` instead of a `String`.
- `Session.globals: HashMap<Symbol, Value>` (was `HashMap<String, Value>`) — kept consistent with
  the function table rather than left as a mismatched exception; `Env::lookup`'s fallback to
  globals (§11 item 1) now compares `Symbol`s the whole way down, no string comparison anywhere in
  the hot lookup path.
- `session.ext_table: HashMap<String, ProcedureFn>` (§7) is **unaffected** — it's keyed by the
  extension's C symbol name (e.g. `"scm_net_tcp_connect"`), which arrives from extension metadata
  outside the ANF pipeline and is resolved directly into `PrimOp::ExtCall(String)` at transform
  time (extensions spec §6.4), never stored as a `VarId`. Not a hot-path lookup (one call per
  extension invocation site), so no reason to intern it too.

### 12.6 Rollout sequencing

This refactor touches `anf.rs`/`closure.rs`/`codegen.rs` (compiled backend) before the interpreter
exists at all, so it should land **before** rollout step 2 in §9, as its own preparatory change,
validated purely against the existing compiler test suite (no interpreter code depends on it yet).
Building the interpreter's `Env`/`Session`/`Closure` on top of still-`String` `VarId`, then
redoing them for `Symbol` right after, would be duplicated work.
