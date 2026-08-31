//! Compile-to-slots: lower a parsed AST to a flat postfix program.
//!
//! The iterative evaluator resolves every name at run time, on every
//! evaluation: each operator node costs a string-keyed registry lookup, an
//! `Rc` clone, and an indirect call. This module resolves names one time per
//! expression instead:
//!
//! - Variables become slots. A slot is bound to an `Expression` parameter or
//!   to a context name; context slots are refilled once per `eval` call, so
//!   context variables never become stale snapshots.
//! - Builtin operators (registered by `register_default_math_functions` and
//!   not shadowed by a user registration) become direct opcodes.
//! - Other functions resolve to a per-program table of cloned `Rc`
//!   implementations, called by index.
//! - Local expression functions are inlined at compile time; their arguments
//!   evaluate into temp slots.
//! - Constant subtrees fold at compile time, using the same implementations
//!   the VM uses at run time.
//!
//! Anything the compiler cannot prove it handles identically to the
//! iterative evaluator — unknown functions (which must stay lazy errors),
//! arrays, attributes, recursive expression functions, extreme nesting —
//! returns [`CompileOutcome::Fallback`], and the caller evaluates that
//! expression with the iterative engine. Behavior is preserved exactly; the
//! compiled path is only an implementation shortcut.
//!
//! # Why two engines (a deliberate decision, 2026-08)
//!
//! The iterative evaluator stays, on purpose, even though this path is
//! ~6x faster:
//!
//! 1. It is the independent correctness oracle. The differential proptest
//!    (`tests/compile_differential_test.rs`) asserts both engines agree
//!    bit-for-bit; delete one and the test becomes self-referential.
//! 2. It covers what this compiler cannot yet express: runtime recursion
//!    (a `Call` instruction with a depth cap), arrays/attributes, and lazy
//!    unknown-function errors without a fallback target.
//! 3. It backs public API: `interp()`, `eval_ast`, `EvalEngine`,
//!    `eval_with_engine`. Removing it is a semver-major break.
//!
//! The invariant that keeps this sound: **any change to expression
//! semantics must land in both engines, and the differential proptest must
//! stay green.** If language features start arriving regularly and the
//! double-implementation tax bites, the exit path is to grow this compiler
//! to full coverage first (see issue #10), then retire the iterative
//! engine in a major version.
//!
//! Instructions and slot bindings are allocated in the expression arena, so
//! programs die together with the ASTs they were compiled from
//! (`expr_batch_clear` resets both). The function table is a heap `Vec` on
//! purpose: it holds `Rc` clones, and the arena never runs `Drop`, so
//! keeping refcounts there would leak the implementations.

use crate::Real;
use crate::context::EvalContext;
use crate::error::ExprError;
use crate::types::{
    AstExpr, ExpressionFunctionMap, FunctionName, HString, LogicalOperator, TryIntoFunctionName,
    TryIntoHeaplessString,
};
use alloc::format;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use bumpalo::Bump;

/// A cloned native-function implementation, callable by table index.
type NativeImpl = Rc<dyn Fn(&[Real]) -> Real>;

/// Nesting deeper than this falls back to the iterative evaluator, which
/// enforces its own runtime depth limit. This keeps compiled behavior (and
/// the compiler's own recursion) inside the envelope the iterative path
/// already defines.
const MAX_COMPILE_DEPTH: usize = 500;

/// One VM instruction. Kept small on purpose: 16 bytes, versus the ~96-byte
/// `EvalOp` the iterative evaluator moves per node.
#[derive(Clone, Copy, Debug)]
pub enum Instr {
    /// Push a literal (or folded) constant.
    PushConst(Real),
    /// Push the value of a slot.
    LoadSlot(u16),
    /// Pop into a slot (inlined function arguments).
    StoreSlot(u16),
    // Direct opcodes. Each mirrors the exact expression the corresponding
    // builtin registration uses, so results are bit-identical.
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Pow,
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    /// Call a table function with `argc` arguments from the stack top.
    CallNative { table_idx: u16, argc: u8 },
    /// Unconditional jump to an absolute instruction index.
    Jump(u16),
    /// Pop; jump if the value is zero.
    JumpIfZero(u16),
    /// Pop; jump if the value is not zero.
    JumpIfNotZero(u16),
}

/// What a slot is bound to.
#[derive(Clone, Debug)]
pub enum SlotBinding {
    /// The value of the i-th `Expression` parameter, copied in per eval.
    Param(u16),
    /// A name resolved against the context per eval: variables, constants,
    /// the parent chain, then the builtin constants (pi/e/tau). May stay
    /// unresolved; loading an unresolved slot reproduces the iterative
    /// evaluator's lazy unknown-variable error.
    CtxName(HString),
    /// Temp storage for an inlined expression-function argument.
    Temp,
}

/// A compiled expression, ready to run.
pub struct CompiledProgram<'arena> {
    instrs: &'arena [Instr],
    slots: &'arena [SlotBinding],
    /// Function table. Heap-allocated so the `Rc` clones are dropped when
    /// the program is (the arena never runs `Drop`).
    funcs: Vec<NativeImpl>,
    /// Arity per table entry, checked at call time — the iterative
    /// evaluator only reports arity errors when a call actually executes,
    /// and short-circuit operators can skip calls.
    func_arities: Vec<u8>,
    /// Names per table entry, for error messages.
    func_names: Vec<FunctionName>,
    /// Slot values; `Param` and `CtxName` entries are refilled per eval.
    slot_values: Vec<Real>,
    /// False for a `CtxName` slot that did not resolve this batch.
    slot_ok: Vec<bool>,
    /// Value stack, reused across evals. Capacity is the exact maximum the
    /// program can need.
    stack: Vec<Real>,
}

/// Result of compiling one expression.
pub enum CompileOutcome<'arena> {
    Compiled(CompiledProgram<'arena>),
    /// Evaluate this expression with the iterative engine instead.
    Fallback,
}

// ---------------------------------------------------------------------------
// Direct opcode resolution
// ---------------------------------------------------------------------------

fn direct_op_for(name: &str, argc: usize) -> Option<Instr> {
    let op = match (name, argc) {
        ("+", 2) | ("add", 2) => Instr::Add,
        ("-", 2) | ("sub", 2) => Instr::Sub,
        ("*", 2) | ("mul", 2) => Instr::Mul,
        ("/", 2) | ("div", 2) => Instr::Div,
        ("%", 2) | ("fmod", 2) => Instr::Mod,
        ("neg", 1) => Instr::Neg,
        ("^", 2) | ("pow", 2) => Instr::Pow,
        ("<", 2) => Instr::Lt,
        (">", 2) => Instr::Gt,
        ("<=", 2) => Instr::Le,
        (">=", 2) => Instr::Ge,
        ("==", 2) => Instr::Eq,
        ("!=", 2) => Instr::Ne,
        _ => return None,
    };
    Some(op)
}

#[inline]
fn apply_binary(op: Instr, a: Real, b: Real) -> Result<Real, ExprError> {
    Ok(match op {
        Instr::Add => a + b,
        Instr::Sub => a - b,
        Instr::Mul => a * b,
        Instr::Div => a / b,
        Instr::Mod => a % b,
        Instr::Pow => pow_impl(a, b)?,
        Instr::Lt => {
            if a < b {
                1.0
            } else {
                0.0
            }
        }
        Instr::Gt => {
            if a > b {
                1.0
            } else {
                0.0
            }
        }
        Instr::Le => {
            if a <= b {
                1.0
            } else {
                0.0
            }
        }
        Instr::Ge => {
            if a >= b {
                1.0
            } else {
                0.0
            }
        }
        Instr::Eq => {
            if a == b {
                1.0
            } else {
                0.0
            }
        }
        Instr::Ne => {
            if a != b {
                1.0
            } else {
                0.0
            }
        }
        _ => return Err(ExprError::Other("not a binary opcode".to_string())),
    })
}

// The builtin "^"/"pow" registrations exist only with libm, and Pow is only
// emitted when that builtin resolved, so the not-libm arm is unreachable in
// practice.
#[cfg(any(feature = "libm", test))]
#[inline]
fn pow_impl(a: Real, b: Real) -> Result<Real, ExprError> {
    Ok(crate::functions::pow(a, b))
}

#[cfg(not(any(feature = "libm", test)))]
#[inline]
fn pow_impl(_a: Real, _b: Real) -> Result<Real, ExprError> {
    Err(ExprError::Other("pow requires the libm feature".to_string()))
}

// ---------------------------------------------------------------------------
// Compiler
// ---------------------------------------------------------------------------

struct ScopeFrame {
    /// Function parameter name → temp slot, for the innermost inline
    /// expansion. Later frames shadow earlier ones, and earlier frames stay
    /// visible — the iterative evaluator walks all parameter frames on the
    /// op stack, which gives dynamic scoping across nested inlined calls.
    bindings: Vec<(HString, u16)>,
}

pub struct Compiler<'a, 'arena> {
    arena: &'arena Bump,
    ctx: &'a EvalContext,
    /// Expression parameter names, index-aligned with parameter indices.
    param_keys: &'a [HString],
    local_functions: Option<&'a ExpressionFunctionMap>,

    instrs: Vec<Instr>,
    slots: Vec<SlotBinding>,
    funcs: Vec<NativeImpl>,
    func_arities: Vec<u8>,
    func_names: Vec<FunctionName>,

    scopes: Vec<ScopeFrame>,
    expansion_chain: Vec<FunctionName>,

    cur_depth: usize,
    max_depth: usize,
}

/// Internal signal: this expression cannot be compiled faithfully; use the
/// iterative evaluator for it.
struct Abort;

impl<'a, 'arena> Compiler<'a, 'arena> {
    pub fn compile(
        arena: &'arena Bump,
        ctx: &'a EvalContext,
        param_keys: &'a [HString],
        local_functions: Option<&'a ExpressionFunctionMap>,
        ast: &AstExpr<'arena>,
    ) -> CompileOutcome<'arena> {
        if ast_depth_exceeds(ast, MAX_COMPILE_DEPTH) {
            return CompileOutcome::Fallback;
        }

        let mut c = Compiler {
            arena,
            ctx,
            param_keys,
            local_functions,
            instrs: Vec::new(),
            slots: Vec::new(),
            funcs: Vec::new(),
            func_arities: Vec::new(),
            func_names: Vec::new(),
            scopes: Vec::new(),
            expansion_chain: Vec::new(),
            cur_depth: 0,
            max_depth: 0,
        };

        if c.compile_node(ast).is_err() {
            return CompileOutcome::Fallback;
        }

        let instrs = arena.alloc_slice_copy(&c.instrs);
        let slots = bumpalo::collections::Vec::from_iter_in(c.slots.iter().cloned(), arena)
            .into_bump_slice();

        let slot_count = slots.len();
        let mut stack = Vec::new();
        stack.reserve_exact(c.max_depth.max(1));

        CompileOutcome::Compiled(CompiledProgram {
            instrs,
            slots,
            funcs: c.funcs,
            func_arities: c.func_arities,
            func_names: c.func_names,
            slot_values: alloc::vec![0.0; slot_count],
            slot_ok: alloc::vec![false; slot_count],
            stack,
        })
    }

    // -- emission helpers -------------------------------------------------

    fn emit(&mut self, i: Instr) {
        self.instrs.push(i);
    }

    fn note_push(&mut self) {
        self.cur_depth += 1;
        if self.cur_depth > self.max_depth {
            self.max_depth = self.cur_depth;
        }
    }

    fn note_pop(&mut self, n: usize) {
        self.cur_depth -= n;
    }

    fn here(&self) -> usize {
        self.instrs.len()
    }

    fn patch_jump(&mut self, at: usize, target: usize) -> Result<(), Abort> {
        let t = u16::try_from(target).map_err(|_| Abort)?;
        match &mut self.instrs[at] {
            Instr::Jump(x) | Instr::JumpIfZero(x) | Instr::JumpIfNotZero(x) => *x = t,
            _ => return Err(Abort),
        }
        Ok(())
    }

    fn new_slot(&mut self, binding: SlotBinding) -> Result<u16, Abort> {
        let idx = u16::try_from(self.slots.len()).map_err(|_| Abort)?;
        self.slots.push(binding);
        Ok(idx)
    }

    /// Find an existing slot for a binding that is refilled by name, so the
    /// per-eval fill cost is O(unique names), not O(occurrences).
    fn slot_for_param(&mut self, param_idx: u16) -> Result<u16, Abort> {
        for (i, s) in self.slots.iter().enumerate() {
            if let SlotBinding::Param(p) = s
                && *p == param_idx
            {
                return Ok(i as u16);
            }
        }
        self.new_slot(SlotBinding::Param(param_idx))
    }

    fn slot_for_ctx_name(&mut self, name: &HString) -> Result<u16, Abort> {
        for (i, s) in self.slots.iter().enumerate() {
            if let SlotBinding::CtxName(n) = s
                && n == name
            {
                return Ok(i as u16);
            }
        }
        self.new_slot(SlotBinding::CtxName(name.clone()))
    }

    // -- constant folding -------------------------------------------------

    /// Fold a subtree to a constant when that is provably identical to
    /// runtime evaluation. Only direct opcodes (builtin, unshadowed
    /// operators), logical ops, and conditionals participate; native calls
    /// never fold.
    fn try_fold(&self, e: &AstExpr) -> Option<Real> {
        match e {
            AstExpr::Constant(v) => Some(*v),
            AstExpr::Function { name, args } => {
                // Never fold through a name that an inline scope or a
                // parameter could rebind, or that isn't the builtin.
                let op = self.resolve_direct_op(name, args.len())?;
                match args.len() {
                    1 => {
                        let a = self.try_fold(&args[0])?;
                        match op {
                            Instr::Neg => Some(-a),
                            _ => None,
                        }
                    }
                    2 => {
                        let a = self.try_fold(&args[0])?;
                        let b = self.try_fold(&args[1])?;
                        apply_binary(op, a, b).ok()
                    }
                    _ => None,
                }
            }
            AstExpr::LogicalOp { op, left, right } => {
                let l = self.try_fold(left)?;
                match op {
                    LogicalOperator::And => {
                        if l == 0.0 {
                            Some(0.0)
                        } else {
                            let r = self.try_fold(right)?;
                            Some(if r != 0.0 { 1.0 } else { 0.0 })
                        }
                    }
                    LogicalOperator::Or => {
                        if l != 0.0 {
                            Some(1.0)
                        } else {
                            let r = self.try_fold(right)?;
                            Some(if r != 0.0 { 1.0 } else { 0.0 })
                        }
                    }
                }
            }
            AstExpr::Conditional {
                condition,
                true_branch,
                false_branch,
            } => {
                let c = self.try_fold(condition)?;
                if c != 0.0 {
                    self.try_fold(true_branch)
                } else {
                    self.try_fold(false_branch)
                }
            }
            _ => None,
        }
    }

    /// Resolve a function name to a direct opcode: it must be a builtin
    /// operator name, present in the registry, flagged builtin (not
    /// shadowed), with the matching arity — and not shadowed by a local
    /// expression function or an inline-scope binding either.
    fn resolve_direct_op(&self, name: &str, argc: usize) -> Option<Instr> {
        let op = direct_op_for(name, argc)?;
        if let Some(locals) = self.local_functions
            && let Ok(key) = name.try_into_function_name()
            && locals.contains_key(&key)
        {
            return None;
        }
        let f = self.ctx.get_native_function(name)?;
        if !f.builtin || f.arity != argc {
            return None;
        }
        Some(op)
    }

    // -- lowering ---------------------------------------------------------

    fn compile_node(&mut self, e: &AstExpr<'arena>) -> Result<(), Abort> {
        if let Some(v) = self.try_fold(e) {
            self.emit(Instr::PushConst(v));
            self.note_push();
            return Ok(());
        }

        match e {
            AstExpr::Constant(v) => {
                self.emit(Instr::PushConst(*v));
                self.note_push();
                Ok(())
            }

            AstExpr::Variable(name) => self.compile_variable(name),

            AstExpr::Function { name, args } => {
                // Short-circuit forms first; the iterative evaluator treats
                // ("&&", 2) and ("||", 2) specially regardless of the
                // registry, so the compiler must too.
                match (*name, args.len()) {
                    ("&&", 2) => {
                        return self.compile_logical(LogicalOperator::And, &args[0], &args[1]);
                    }
                    ("||", 2) => {
                        return self.compile_logical(LogicalOperator::Or, &args[0], &args[1]);
                    }
                    _ => {}
                }
                self.compile_call(name, args)
            }

            AstExpr::LogicalOp { op, left, right } => {
                self.compile_logical(op.clone(), left, right)
            }

            AstExpr::Conditional {
                condition,
                true_branch,
                false_branch,
            } => {
                // Constant condition: compile only the taken branch, exactly
                // like runtime short-circuiting.
                if let Some(c) = self.try_fold(condition) {
                    return if c != 0.0 {
                        self.compile_node(true_branch)
                    } else {
                        self.compile_node(false_branch)
                    };
                }

                self.compile_node(condition)?;
                let jz = self.here();
                self.emit(Instr::JumpIfZero(0));
                self.note_pop(1);

                let depth_at_branch = self.cur_depth;
                self.compile_node(true_branch)?;
                let jend = self.here();
                self.emit(Instr::Jump(0));

                let false_start = self.here();
                self.patch_jump(jz, false_start)?;
                self.cur_depth = depth_at_branch;
                self.compile_node(false_branch)?;

                let end = self.here();
                self.patch_jump(jend, end)?;
                Ok(())
            }

            // Arrays and attributes stay on the iterative path.
            AstExpr::Array { .. } | AstExpr::Attribute { .. } => Err(Abort),
        }
    }

    fn compile_variable(&mut self, name: &str) -> Result<(), Abort> {
        let hname = name.try_into_heapless().map_err(|_| Abort)?;

        // Innermost inline scope first (iterative: parameter frames on the
        // op stack, walked newest-first).
        for scope in self.scopes.iter().rev() {
            for (n, slot) in &scope.bindings {
                if n == &hname {
                    self.emit(Instr::LoadSlot(*slot));
                    self.note_push();
                    return Ok(());
                }
            }
        }

        // Expression parameters second (iterative: the override map).
        for (i, key) in self.param_keys.iter().enumerate() {
            if key == &hname {
                let slot = self.slot_for_param(i as u16)?;
                self.emit(Instr::LoadSlot(slot));
                self.note_push();
                return Ok(());
            }
        }

        // Context name, resolved per eval. Unresolved is not a compile
        // error: the iterative evaluator only errors when the lookup
        // actually executes.
        let slot = self.slot_for_ctx_name(&hname)?;
        self.emit(Instr::LoadSlot(slot));
        self.note_push();
        Ok(())
    }

    fn compile_logical(
        &mut self,
        op: LogicalOperator,
        left: &AstExpr<'arena>,
        right: &AstExpr<'arena>,
    ) -> Result<(), Abort> {
        // Foldable left operand: compile the reduced form.
        if let Some(l) = self.try_fold(left) {
            match op {
                LogicalOperator::And if l == 0.0 => {
                    self.emit(Instr::PushConst(0.0));
                    self.note_push();
                    return Ok(());
                }
                LogicalOperator::Or if l != 0.0 => {
                    self.emit(Instr::PushConst(1.0));
                    self.note_push();
                    return Ok(());
                }
                // The result is right != 0.0.
                _ => {
                    self.compile_node(right)?;
                    self.emit(Instr::PushConst(0.0));
                    self.note_push();
                    self.emit(Instr::Ne);
                    self.note_pop(1);
                    return Ok(());
                }
            }
        }

        let (short_jump, short_value, other_value): (fn(u16) -> Instr, Real, Real) = match op {
            LogicalOperator::And => (Instr::JumpIfZero, 0.0, 1.0),
            LogicalOperator::Or => (Instr::JumpIfNotZero, 1.0, 0.0),
        };

        self.compile_node(left)?;
        let j1 = self.here();
        self.emit(short_jump(0));
        self.note_pop(1);

        self.compile_node(right)?;
        let j2 = self.here();
        self.emit(short_jump(0));
        self.note_pop(1);

        self.emit(Instr::PushConst(other_value));
        self.note_push();
        let jend = self.here();
        self.emit(Instr::Jump(0));

        let short_target = self.here();
        self.patch_jump(j1, short_target)?;
        self.patch_jump(j2, short_target)?;
        self.cur_depth -= 1; // the two paths converge one value deep
        self.emit(Instr::PushConst(short_value));
        self.note_push();

        let end = self.here();
        self.patch_jump(jend, end)?;
        Ok(())
    }

    fn compile_call(&mut self, name: &str, args: &'arena [AstExpr<'arena>]) -> Result<(), Abort> {
        // Local expression functions take priority over everything.
        if let Some(locals) = self.local_functions {
            let key = name.try_into_function_name().map_err(|_| Abort)?;
            if let Some(func) = locals.get(&key) {
                return self.compile_inline_function(&key, &func.clone(), args);
            }
        }

        // Builtin operator → direct opcode.
        if let Some(op) = self.resolve_direct_op(name, args.len()) {
            for a in args {
                self.compile_node(a)?;
            }
            self.emit(op);
            match args.len() {
                1 => {} // Neg: pop one, push one
                2 => self.note_pop(1),
                _ => return Err(Abort),
            }
            return Ok(());
        }

        // Any other registered function → table call.
        let Some(f) = self.ctx.get_native_function(name) else {
            // Unknown function: must stay a lazy runtime error (it may sit
            // in a branch that never executes). The iterative path
            // reproduces that exactly.
            return Err(Abort);
        };

        let argc = u8::try_from(args.len()).map_err(|_| Abort)?;
        let table_idx = u16::try_from(self.funcs.len()).map_err(|_| Abort)?;
        self.funcs.push(f.implementation.clone());
        self.func_arities
            .push(u8::try_from(f.arity).map_err(|_| Abort)?);
        self.func_names.push(f.name.clone());

        for a in args {
            self.compile_node(a)?;
        }
        self.emit(Instr::CallNative { table_idx, argc });
        // Pops argc, pushes one.
        if args.is_empty() {
            self.note_push();
        } else {
            self.note_pop(args.len() - 1);
        }
        Ok(())
    }

    fn compile_inline_function(
        &mut self,
        key: &FunctionName,
        func: &crate::types::ExpressionFunction,
        args: &'arena [AstExpr<'arena>],
    ) -> Result<(), Abort> {
        // A repeated name in the expansion chain is (potential) recursion.
        // Inline expansion of a cycle would not terminate; the iterative
        // evaluator handles base-case recursion at runtime, so hand the
        // whole expression to it.
        if self.expansion_chain.contains(key) {
            return Err(Abort);
        }

        // Arity mismatches and body parse errors are lazy errors on the
        // iterative path (the body parses when the call executes), so they
        // are fallbacks here, not compile errors.
        if args.len() != func.params.len() {
            return Err(Abort);
        }
        let param_names: Vec<String> = func.params.clone();
        let Ok(body) =
            crate::engine::parse_expression_with_parameters(&func.expression, self.arena, &param_names)
        else {
            return Err(Abort);
        };
        let body = &*self.arena.alloc(body);
        if ast_depth_exceeds(body, MAX_COMPILE_DEPTH) {
            return Err(Abort);
        }

        // Evaluate arguments left to right into temp slots.
        let mut bindings = Vec::with_capacity(args.len());
        for (arg, pname) in args.iter().zip(func.params.iter()) {
            let slot = self.new_slot(SlotBinding::Temp)?;
            self.compile_node(arg)?;
            self.emit(Instr::StoreSlot(slot));
            self.note_pop(1);
            let hname = pname.as_str().try_into_heapless().map_err(|_| Abort)?;
            bindings.push((hname, slot));
        }

        self.expansion_chain.push(key.clone());
        self.scopes.push(ScopeFrame { bindings });
        let result = self.compile_node(body);
        self.scopes.pop();
        self.expansion_chain.pop();
        result
    }
}

/// Iterative (explicit-stack) depth check, so the compiler's own recursion
/// is bounded before it starts.
fn ast_depth_exceeds(ast: &AstExpr, limit: usize) -> bool {
    let mut stack: Vec<(&AstExpr, usize)> = alloc::vec![(ast, 1)];
    while let Some((e, d)) = stack.pop() {
        if d > limit {
            return true;
        }
        match e {
            AstExpr::Constant(_) | AstExpr::Variable(_) | AstExpr::Attribute { .. } => {}
            AstExpr::Function { args, .. } => {
                for a in *args {
                    stack.push((a, d + 1));
                }
            }
            AstExpr::Array { index, .. } => stack.push((index, d + 1)),
            AstExpr::LogicalOp { left, right, .. } => {
                stack.push((left, d + 1));
                stack.push((right, d + 1));
            }
            AstExpr::Conditional {
                condition,
                true_branch,
                false_branch,
            } => {
                stack.push((condition, d + 1));
                stack.push((true_branch, d + 1));
                stack.push((false_branch, d + 1));
            }
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Slot filling and execution
// ---------------------------------------------------------------------------

/// Context-name lookup, mirroring `ContextStack::lookup_variable` plus the
/// builtin constants from `process_variable_lookup`.
fn lookup_ctx_name(ctx: &EvalContext, name: &HString) -> Option<Real> {
    let mut current = Some(ctx);
    while let Some(c) = current {
        if let Some(&v) = c.variables.get(name) {
            return Some(v);
        }
        if let Some(&v) = c.constants.get(name) {
            return Some(v);
        }
        current = c.parent.as_deref();
    }
    match name.as_str() {
        "pi" | "PI" => Some(core::f64::consts::PI as Real),
        "e" | "E" => Some(core::f64::consts::E as Real),
        "tau" | "TAU" => Some(2.0 * core::f64::consts::PI as Real),
        _ => None,
    }
}

/// The iterative evaluator's unknown-variable classification, reproduced for
/// lazy load errors.
fn unknown_variable_error(name: &HString) -> ExprError {
    let is_potential_function_name = matches!(
        name.as_str(),
        "sin" | "cos"
            | "tan"
            | "asin"
            | "acos"
            | "atan"
            | "atan2"
            | "sinh"
            | "cosh"
            | "tanh"
            | "exp"
            | "log"
            | "log10"
            | "ln"
            | "sqrt"
            | "abs"
            | "ceil"
            | "floor"
            | "pow"
            | "neg"
            | ","
            | "comma"
            | "+"
            | "-"
            | "*"
            | "/"
            | "%"
            | "^"
            | "max"
            | "min"
            | "<"
            | ">"
            | "<="
            | ">="
            | "=="
            | "!="
    );
    if is_potential_function_name && name.len() > 1 {
        ExprError::Syntax(format!("Function '{}' used without arguments", name))
    } else {
        ExprError::UnknownVariable {
            name: name.to_string(),
        }
    }
}

impl<'arena> CompiledProgram<'arena> {
    /// Refill parameter and context slots. Called once per `eval`, before
    /// running; O(unique names), not O(occurrences).
    pub fn fill_slots(&mut self, params: &[crate::expression::Param], ctx: &EvalContext) {
        for (i, binding) in self.slots.iter().enumerate() {
            match binding {
                SlotBinding::Param(p) => {
                    self.slot_values[i] = params[*p as usize].value;
                    self.slot_ok[i] = true;
                }
                SlotBinding::CtxName(name) => match lookup_ctx_name(ctx, name) {
                    Some(v) => {
                        self.slot_values[i] = v;
                        self.slot_ok[i] = true;
                    }
                    None => {
                        self.slot_ok[i] = false;
                    }
                },
                SlotBinding::Temp => {
                    self.slot_ok[i] = true;
                }
            }
        }
    }

    /// Run the program and return the result.
    pub fn run(&mut self) -> Result<Real, ExprError> {
        let instrs = self.instrs;
        let stack = &mut self.stack;
        stack.clear();

        let mut pc = 0usize;
        while pc < instrs.len() {
            match instrs[pc] {
                Instr::PushConst(v) => stack.push(v),
                Instr::LoadSlot(s) => {
                    let s = s as usize;
                    if !self.slot_ok[s] {
                        if let SlotBinding::CtxName(name) = &self.slots[s] {
                            return Err(unknown_variable_error(name));
                        }
                        return Err(ExprError::Other("unset slot".to_string()));
                    }
                    stack.push(self.slot_values[s]);
                }
                Instr::StoreSlot(s) => {
                    let v = stack.pop().ok_or_else(stack_underflow)?;
                    let s = s as usize;
                    self.slot_values[s] = v;
                    self.slot_ok[s] = true;
                }
                Instr::Neg => {
                    let a = stack.last_mut().ok_or_else(stack_underflow)?;
                    *a = -*a;
                }
                Instr::Add
                | Instr::Sub
                | Instr::Mul
                | Instr::Div
                | Instr::Mod
                | Instr::Pow
                | Instr::Lt
                | Instr::Gt
                | Instr::Le
                | Instr::Ge
                | Instr::Eq
                | Instr::Ne => {
                    let b = stack.pop().ok_or_else(stack_underflow)?;
                    let a = stack.last_mut().ok_or_else(stack_underflow)?;
                    *a = apply_binary(instrs[pc], *a, b)?;
                }
                Instr::CallNative { table_idx, argc } => {
                    let idx = table_idx as usize;
                    let argc = argc as usize;
                    if argc != self.func_arities[idx] as usize {
                        return Err(ExprError::InvalidFunctionCall {
                            name: self.func_names[idx].to_string(),
                            expected: self.func_arities[idx] as usize,
                            found: argc,
                        });
                    }
                    let start = stack.len().checked_sub(argc).ok_or_else(stack_underflow)?;
                    let result = (self.funcs[idx])(&stack[start..]);
                    stack.truncate(start);
                    stack.push(result);
                }
                Instr::Jump(t) => {
                    pc = t as usize;
                    continue;
                }
                Instr::JumpIfZero(t) => {
                    let v = stack.pop().ok_or_else(stack_underflow)?;
                    if v == 0.0 {
                        pc = t as usize;
                        continue;
                    }
                }
                Instr::JumpIfNotZero(t) => {
                    let v = stack.pop().ok_or_else(stack_underflow)?;
                    if v != 0.0 {
                        pc = t as usize;
                        continue;
                    }
                }
            }
            pc += 1;
        }

        stack.pop().ok_or_else(stack_underflow)
    }
}

fn stack_underflow() -> ExprError {
    ExprError::Other("Value stack underflow".to_string())
}
