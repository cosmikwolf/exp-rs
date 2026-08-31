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
- [x] Port `qemu_test/batch_performance_test.c` from the removed FFI
      (`exp_rs_context_eval`, `exp_rs_batch_eval`) to the current
      `expr_batch_*` API. Re-enable its meson target. (Done 2026-08-31,
      `b06202b`. Measures setup / eval / param update / full cycle in CMSDK
      ticks; verifies against C reference math in both float modes.)
- [x] Fix `qemu_test/test_batch_memory.c`: `struct CAllocationInfo` no longer
      exists in the generated header for this configuration. It stops the whole
      QEMU ninja build. (Done 2026-08-31, `b06202b`. Tracking code guarded by
      `EXP_RS_ALLOC_TRACKING`; meson passes the define when the option is on.
      Full `--qemu` build and run works now, f32 and f64.)
- [x] Guard the allocation-stats calls in `tests_native_c/common_allocator.c`
      with `#ifdef`, so the native C suite links without `--track-allocs`.
      (Done 2026-08-31, `b06202b`. Native suite passes 17/17 in all four
      configurations: f32/f64 x tracking on/off.)
- [x] Add a criterion bench for `Expression::eval`
      (`benches/eval_benchmark.rs`: per-expression, native baseline, and the
      10-param/7-expression batch shape). (Done 2026-08-31, `b06202b`.)
- [x] Capture the baseline on all three targets. Saved to
      `bench_results/2026-08-31_phase0_baseline/` (raw outputs + README with
      summary tables), commit `b06202b`.

Found during Phase 0 (fixed in `b06202b`, no behavior change):
`cargo test --features f32` did not compile: a bad deref in `AstExpr::pow`
cfg branches (`src/types.rs`), f64-typed assertions/casts in tests and
examples, and missing f32 libm imports in `examples/eval_context.rs`.

Baseline highlights (host, commit `b06202b`, 2026-08-31; full tables in
`bench_results/2026-08-31_phase0_baseline/README.md`):

| Suite | Result |
|---|---|
| Rust `eval_benchmark` (f64) | `a+5`: 151 ns/eval; batch update10+eval7: 5.00 µs |
| Rust `eval_benchmark` (f32) | `a+5`: 144 ns/eval; batch update10+eval7: 4.81 µs |
| Native C `test_performance` (f64) | 7 exprs: 1.78 µs/batch, 0.254 µs/expr; param update 10x: 0.041 µs |
| Native C `test_performance` (f32) | 7 exprs: 1.75 µs/batch, 0.251 µs/expr |
| QEMU `batch_performance_test` (f64) | eval/batch of 6: 25 ticks; full cycle: 65 ticks |

Note: the Rust batch path (5.0 µs) is ~2.8x slower than the same shape through
the C FFI (1.78 µs) — that gap is the per-call `BatchParamMap` rebuild (root
cause 4, Phase A3 target).

Caveat: QEMU is not cycle-accurate. QEMU numbers compare before/after only.
Ground truth for cycle counts is the DWT measurement on real hardware in the
consumer project. Plan: one on-hardware check per phase from the consumer side.

### How to run the benchmarks

- Rust: `cargo bench --bench eval_benchmark` (add `--features f32` for f32).
  This is the in-repo replacement for the temporary microbenchmark behind the
  per-expression table in the Problem section.
  Also: `cargo bench --bench arena_consolidated_benchmark`.
- Native C: `./run_tests.sh --native [-m f32]`. Then run the binary directly
  for the timing output, because meson test hides it:
  `./target/meson/tests_native_c/test_performance`
- QEMU: `./run_tests.sh --qemu [-m f32]` (full suite works now). For benchmark
  output, run the kernel directly:
  `qemu-system-arm -M mps2-an500 -cpu cortex-m7 -semihosting
  -semihosting-config enable=on,target=native -nographic -monitor none
  -serial stdio -kernel target/meson/qemu_test/batch_performance_test_f64`

---

## Phase A — Remove batch-level overhead

Goal: remove the per-expression engine overhead. Targets the ~1,638 cycles the
consumer project measured.

- [x] A1. Delete dead code:
      `func_cache` (`src/eval/iterative.rs:55`, cleared per eval, never read),
      `FunctionCacheEntry` and `OwnedNativeFunction` (`src/eval/types.rs`),
      the `visited_contexts` Vec in `ContextStack::lookup_variable`
      (`src/eval/context_stack.rs:142`, heap alloc per lookup, never read),
      unused imports in `src/ffi.rs:79`.
      Note: `src/eval/recursion.rs` is also dead but public API. It stays.
      (Done 2026-08-31, `356a070`. `src/eval/types.rs` removed entirely; the
      ffi imports are feature-gated, not deleted — they are live under
      `alloc_tracking`.)
- [x] A2. Split the engine: `begin_batch(ctx)` / `eval_one(ast)` / `end_batch()`.
      `begin_batch` pushes the context and sets overrides one time per batch.
      `eval_one` clears only the op and value stacks.
      This removes the per-expression `ctx_stack.clear()` (writes all 128
      `parent_map` slots), the context push, and the `Rc` churn.
      `EvalEngine::eval` and `eval_with_engine` stay as wrappers; no API change.
      (Done 2026-08-31, `356a070`.)
- [x] A3. The engine owns the parameter override map permanently.
      `add_parameter` / `set_param` write through to it. This removes the per-eval
      rebuild and the ~3 KB map move in `Expression::eval` (`src/expression.rs:125`).
      (A borrowed map is not possible: `Expression` owns the engine; that would
      self-borrow.)
      (Done 2026-08-31, `356a070`. Cost shift: `set_param` now does one hash
      lookup per update — native C param update 10x went 0.041 → 0.13 µs —
      but the update+eval cycle still nets −19%. Phase B slots make
      `set_param` a plain array write again. A full override map now errors
      at `add_parameter` time instead of at eval time.)
- [x] A4. FFI: `expr_batch_evaluate` with a NULL context builds a full
      `EvalContext::new()` per call, which registers ~30 functions
      (`src/ffi.rs:1450`, `src/context.rs:126`). Fix: one lazy default context
      per batch, built on first use.
      (Done 2026-08-31, `356a070`. Applied to `expr_batch_evaluate_ex` too.)
- [x] A5. Verify and measure: `cargo test` + `cargo clippy`, default and `f32`
      features; run all Phase 0 benchmarks; record results below.
      (Done 2026-08-31. cargo test 14/14 suites, native C 17/17, QEMU 4/4,
      both float modes. clippy warnings 33 → 21, all pre-existing.
      `exp_rs.h` byte-identical.)

Found during Phase A (fixed in `356a070`):
`expr_context_add_function` never validated a NULL function pointer. It
"rejected" NULL only because the engine held a clone of the last-used context,
which made `Rc::get_mut` return -4 for ANY registration after an evaluate.
A2's `end_batch` releases the context, which exposed both problems. Now:
registration after an evaluate works, NULL is rejected explicitly
(`NativeFunc` is `Option<extern fn>`; cbindgen output unchanged).

Results after Phase A (full tables and raw outputs in
`bench_results/2026-08-31_phaseA/`):

| Metric | Phase 0 | Phase A | Change |
|---|---|---|---|
| Rust `a+5` per eval (f64) | 150.7 ns | 90.7 ns | −40% |
| Rust `(a+5)*2` | 217.2 ns | 127.1 ns | −41% |
| Rust `1/(a+1)+2/(a+2)+3/(a+3)` | 447.4 ns | ~388 ns | −13% |
| Rust batch update10+eval7 (controlled A/B) | 4.94 µs | 4.76 µs | −4% |
| Native C batch eval 7 exprs (f64) | 1.776 µs | ~1.41 µs | −20% |
| Native C full update+eval cycle (f64) | 1.894 µs | ~1.53 µs | −19% |
| Native C full cycle (f32) | 1.784 µs | ~1.60 µs | −11% |

Reading: the fixed per-eval overhead dropped by ~60 ns (context clear, push,
Rc churn, map rebuild). Small expressions gain 40%; heavy expressions gain
little because per-node cost dominates — exactly the Phase B/C targets
(string-keyed function lookups per operator node, 96-byte `EvalOp` moves).

Measurement notes for later phases:
- End-of-suite criterion runs read up to 15% high on this machine
  (thermal/scheduling). For the batch bench, compare isolated runs, ideally
  A/B against a worktree at the reference commit.
- The first `test_performance` run after a rebuild reads high; warm up once
  and use the second run.
- meson does not track Rust sources: after editing `src/`, delete
  `target/meson/libexp_rs.a` (and `exp_rs.h`) or the C suites link the stale
  library.

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
