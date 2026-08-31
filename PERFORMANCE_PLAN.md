# Runtime Evaluation Performance Plan

Status document. Tracks the work to make runtime expression evaluation fast.
Update the checkboxes and the results tables as work completes.

- Started: 2026-08-31, at commit `77847d5`
- Owner: tenkai
- Consumer project: uses this crate through a meson wrap (`subprojects/exp-rs`),
  builds with the `f32` feature, evaluates 6-9 expressions per tick on a Cortex-M7.

## Problem

Evaluation resolves every name at run time, on every evaluation. Each AST node
costs string copies, hash lookups, and large memory moves. Measured on the batch
path (`Expression::eval`, the same path as `expr_batch_evaluate`):

| Expression | Batch path | Native | Ratio |
|---|---|---|---|
| `a+5` | 150 ns/eval | 1.1 ns | 133x |
| `(a+5)*2` | 190 ns/eval | 1.1 ns | 170x |
| `a*a + 2*a + 1` | 285 ns/eval | 1.3 ns | 221x |
| `sin(a)*cos(a) + sqrt(a+1)` | 390 ns/eval | 7.9 ns | 49x |
| `1/(a+1)+2/(a+2)+3/(a+3)` | 440 ns/eval | 1.1 ns | 391x |

(Apple Silicon host, f64, release profile. 2026-08-31.)

The consumer project measured ~1,638 cycles of per-expression engine overhead on
target, on top of per-node evaluation cost.

Root causes, verified in source and by CPU profile:

1. Every operator is an `AstExpr::Function` with a string name. Each evaluation
   does two string-keyed hash lookups, an `Rc` clone, and an indirect call per
   operator node (`src/eval/iterative.rs`, `process_function_call`).
2. `EvalOp` is ~96 bytes because it carries `HString`/`FunctionName` inline.
   `memmove` is the top entry in the CPU profile.
3. Per expression, the engine clears and rebuilds per-batch state: context
   stack, overrides, a 128-slot `parent_map` (its `clear` writes every slot).
4. `Expression::eval` rebuilds a ~3 KB `BatchParamMap` on every call.
5. No constant folds. No compile-time name resolution.

## Target

- Simple expressions: tens of nanoseconds per evaluation on the host.
- Remove the per-expression engine overhead (item 3) completely.
- No public API or `exp_rs.h` changes. No behavior changes.

---

## Phase 0 — Benchmark repair

Goal: honest before/after numbers on all three targets (Rust, native C, QEMU).

- [x] Fix `[profile.bench] opt-level = 0` in `Cargo.toml`. Benches now inherit
      the release profile. (Done 2026-08-31.)
- [ ] Port `qemu_test/batch_performance_test.c` from the removed FFI
      (`exp_rs_context_eval`, `exp_rs_batch_eval`) to the current
      `expr_batch_*` API. Re-enable its meson target.
- [ ] Fix `qemu_test/test_batch_memory.c`: `struct CAllocationInfo` no longer
      exists in the generated header for this configuration. It stops the whole
      QEMU ninja build.
- [ ] Guard the allocation-stats calls in `tests_native_c/common_allocator.c`
      with `#ifdef`, so the native C suite links without `--track-allocs`.
      (The symbols only exist with the `alloc_tracking` feature.)
- [ ] Add a criterion bench for `Expression::eval` (the current bench files
      import criterion but only do single-shot manual timing).
- [ ] Capture the baseline on all three targets. Save to `bench_results/`
      with date and commit hash.

Baseline captured so far (host, f64, commit `77847d5` + bench profile fix):

| Suite | Result |
|---|---|
| Rust `arena_consolidated_benchmark` | 7 expressions: 7.8 µs/iteration, 1.1 µs/expression |
| Native C `test_performance` | 7 expressions: 1.78 µs/batch, 0.254 µs/expression; param update 10x: 0.042 µs |
| QEMU | harness runs; no exp-rs evaluation benchmark exists yet |

Caveat: QEMU is not cycle-accurate. QEMU numbers compare before/after only.
Ground truth for cycle counts is the DWT measurement on real hardware in the
consumer project. Plan: one on-hardware check per phase from the consumer side.

### How to run the benchmarks

- Rust: `cargo bench --bench arena_consolidated_benchmark`
- The per-expression table in the Problem section came from a temporary
  microbenchmark (a loop over `set_param` + `eval` on one `Expression` with one
  parameter, 200k iterations, against a native closure). The criterion bench
  from Phase 0 replaces it in-repo.
- Native C: `./run_tests.sh --native --track-allocs -t test_performance`.
  The `--track-allocs` flag is REQUIRED until the `common_allocator.c` guard
  lands (the default configuration fails to link). Then run the binary directly
  for the timing output, because meson test hides it:
  `./target/meson/tests_native_c/test_performance`
- QEMU: `./run_tests.sh --qemu -t test_cmsis_dsp_benchmark_f64` (harness check).
  A full `--qemu` run fails until the `test_batch_memory.c` fix lands.
  The exp-rs evaluation benchmark for QEMU does not exist yet (Phase 0 task).

---

## Phase A — Remove batch-level overhead

Goal: remove the per-expression engine overhead. Targets the ~1,638 cycles the
consumer project measured.

- [ ] A1. Delete dead code:
      `func_cache` (`src/eval/iterative.rs:55`, cleared per eval, never read),
      `FunctionCacheEntry` and `OwnedNativeFunction` (`src/eval/types.rs`),
      the `visited_contexts` Vec in `ContextStack::lookup_variable`
      (`src/eval/context_stack.rs:142`, heap alloc per lookup, never read),
      unused imports in `src/ffi.rs:79`.
      Note: `src/eval/recursion.rs` is also dead but public API. It stays.
- [ ] A2. Split the engine: `begin_batch(ctx)` / `eval_one(ast)` / `end_batch()`.
      `begin_batch` pushes the context and sets overrides one time per batch.
      `eval_one` clears only the op and value stacks.
      This removes the per-expression `ctx_stack.clear()` (writes all 128
      `parent_map` slots), the context push, and the `Rc` churn.
      `EvalEngine::eval` and `eval_with_engine` stay as wrappers; no API change.
- [ ] A3. The engine owns the parameter override map permanently.
      `add_parameter` / `set_param` write through to it. This removes the per-eval
      rebuild and the ~3 KB map move in `Expression::eval` (`src/expression.rs:125`).
      (A borrowed map is not possible: `Expression` owns the engine; that would
      self-borrow.)
- [ ] A4. FFI: `expr_batch_evaluate` with a NULL context builds a full
      `EvalContext::new()` per call, which registers ~30 functions
      (`src/ffi.rs:1450`, `src/context.rs:126`). Fix: one lazy default context
      per batch, built on first use.
- [ ] A5. Verify and measure: `cargo test` + `cargo clippy`, default and `f32`
      features; run all Phase 0 benchmarks; record results below.

Results after Phase A: _(fill in)_

---

## Phase B — Compile to slots

Goal: resolve names one time per expression, not one time per evaluation.
This is the parse-once vs resolve-once distinction: parse-once exists today;
resolve-once does not.

- [ ] B1. New module `src/compile.rs`:
      - `Instr` enum: `PushConst`, `LoadSlot(u16)`, direct opcodes
        (`Add`, `Sub`, `Mul`, `Div`, `Mod`, `Pow`, comparisons, `Neg`),
        `CallNative { table_idx, argc }`, jump ops for `?:`, `&&`, `||`.
      - Lower the AST to a flat postfix `Program`, allocated in the arena.
        Constraint: `expr_batch_clear` resets the arena (commit `1c7768f`),
        so programs must die together with their ASTs. Arena allocation
        guarantees that.
      - Slot bindings per program: `Param(i)` | `CtxName` | `BuiltinConst`
        (`pi`, `e`, `tau`). `begin_batch` fills ctx-name slots one time per
        batch — O(unique names), not O(occurrences). Context variables cannot
        become fixed snapshots; the caller can change them between calls.
      - Fold constant subtrees at compile time.
- [ ] B2. Compile-time function resolution:
      - Add `builtin: bool` to `NativeFunction`, set only by
        `register_default_math_functions`. Operators resolve to direct opcodes
        when builtin and not shadowed; otherwise to `CallNative`.
        (An override is not detectable today; defaults and user registrations
        look identical in the registry. The flag makes them distinct.)
      - Invalidation: each context gets a unique id from a monotonic counter,
        plus a generation number that function registration increments.
        A program re-resolves when either changes. Rc pointer identity is not
        a safe key (ABA). A stale program would be wrong, not undefined: the
        program table holds cloned `Rc`s, so nothing dangles.
      - Note: external code that builds `NativeFunction` as a struct literal
        needs a one-line update. `register_native_function` does not change.
- [ ] B3. Expression functions: inline bodies at compile time. Track the
      expansion chain; a repeated name = cycle → do not compile, fall back to
      the iterative evaluator for that expression. This preserves current
      behavior exactly: base-case recursion works (ternary evaluates one
      branch), runaway recursion errors at `MAX_STACK_DEPTH = 1000`
      (`src/eval/iterative.rs:23`). The runtime cap cannot protect the
      compiler; inline expansion of a cycle would not terminate.
      Recorded follow-up, only if recursion becomes hot: a `Call` instruction
      with per-function programs and a runtime depth cap.
      (`src/eval/recursion.rs` — the global atomic counter — is dead code; the
      op-stack cap is the live mechanism.)
- [ ] B4. Wire in: `Expression::eval` runs programs. `interp()` and `eval_ast`
      keep the iterative path. FFI surface and `exp_rs.h` unchanged.
- [ ] B5. Verify: differential proptest — compiled result must equal iterative
      result on generated expressions. Full test suite, both float features.
      Run all benchmarks; record results below.

Results after Phase B: _(fill in)_

---

## Phase C — Decide with data

Re-measure `interp()` and the fallback path after Phase B. Apply the deferred
iterative-evaluator fixes only if that path still matters:

- [ ] C1. Shrink `EvalOp` from ~96 to ~32 bytes: store `&'arena str` instead of
      inline `HString`/`FunctionName`. Constraint: heapless 0.8 `String` has no
      `Borrow<str>` impl, so map lookups need a linear scan over entries
      (maps hold ≤16 dense entries) or one `HString` build at the boundary.
      Measure both, pick one.
- [ ] C2. Immediate variable and attribute lookup in `process_eval`, without an
      op round-trip. Verified safe: the pushed op is always the next op popped;
      nothing runs between. `Array` is not eligible (its index evaluates first).
- [ ] C3. Operator fast path in the iterative evaluator through the existing
      `CompleteBinary` / `ApplyUnary` ops (dead today; nothing pushes them).
- [ ] C4. Optional cleanup: mark `src/eval/recursion.rs` `#[deprecated]`.

Results after Phase C: _(fill in)_

---

## Ship

- [ ] Update `bench_results/` with the final comparison.
- [ ] Remove this file (`PERFORMANCE_PLAN.md`) before the final push. It is a
      local working document, not repository documentation.
- [ ] Consumer project pins the meson wrap to the new commit and confirms the
      per-expression overhead on real hardware with DWT.
