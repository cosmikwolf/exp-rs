//! Differential tests: the compiled path must behave exactly like the
//! iterative evaluator — same values (bit-for-bit), same error/no-error
//! outcomes — on generated and hand-picked expressions.

use bumpalo::Bump;
use exp_rs::eval::iterative::EvalEngine;
use exp_rs::{EvalContext, Expression, Real};
use proptest::prelude::*;
use std::rc::Rc;

const A: Real = 1.75;
const B: Real = -0.5;
const X: Real = 3.25;
const Y: Real = 0.125;

/// Context with x/y variables (and the default functions).
fn make_ctx() -> Rc<EvalContext> {
    let mut ctx = EvalContext::new();
    ctx.set_parameter("x", X).unwrap();
    ctx.set_parameter("y", Y).unwrap();
    Rc::new(ctx)
}

/// Iterative reference: parameters a/b provided as context variables, which
/// resolves them to the same values the compiled path reads from its
/// parameter slots.
fn eval_iterative_ref(expr: &str) -> Result<Real, exp_rs::error::ExprError> {
    let mut ctx = EvalContext::new();
    ctx.set_parameter("x", X).unwrap();
    ctx.set_parameter("y", Y).unwrap();
    ctx.set_parameter("a", A).unwrap();
    ctx.set_parameter("b", B).unwrap();
    let arena = Bump::new();
    let ast = exp_rs::engine::parse_expression(expr, &arena)?;
    let mut engine = EvalEngine::new(&arena);
    engine.eval(&ast, Some(Rc::new(ctx)))
}

/// Compiled path: a/b as Expression parameters, x/y from the context.
fn eval_compiled(expr: &str) -> Result<Real, exp_rs::error::ExprError> {
    let ctx = make_ctx();
    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.add_parameter("a", A)?;
    e.add_parameter("b", B)?;
    e.add_expression(expr)?;
    e.eval(&ctx)?;
    Ok(e.get_result(0).unwrap())
}

fn assert_same(expr: &str) {
    let compiled = eval_compiled(expr);
    let iterative = eval_iterative_ref(expr);
    match (&compiled, &iterative) {
        (Ok(c), Ok(i)) => {
            assert!(
                c == i || (c.is_nan() && i.is_nan()),
                "value mismatch for {expr:?}: compiled={c}, iterative={i}"
            );
        }
        (Err(_), Err(_)) => {}
        _ => panic!("outcome mismatch for {expr:?}: compiled={compiled:?}, iterative={iterative:?}"),
    }
}

// ---------------------------------------------------------------------------
// Generated expressions
// ---------------------------------------------------------------------------

fn arb_expr() -> impl Strategy<Value = String> {
    let leaf = prop_oneof![
        (-4i32..10).prop_map(|n| n.to_string()),
        prop_oneof![
            Just("0.5".to_string()),
            Just("2.25".to_string()),
            Just("1e2".to_string()),
        ],
        prop_oneof![
            Just("a".to_string()),
            Just("b".to_string()),
            Just("x".to_string()),
            Just("y".to_string()),
            Just("pi".to_string()),
            Just("e".to_string()),
        ],
    ];
    leaf.prop_recursive(4, 48, 3, |inner| {
        prop_oneof![
            // binary operators
            (
                inner.clone(),
                prop_oneof![
                    Just("+"),
                    Just("-"),
                    Just("*"),
                    Just("/"),
                    Just("%"),
                    Just("^"),
                    Just("<"),
                    Just(">"),
                    Just("<="),
                    Just(">="),
                    Just("=="),
                    Just("!="),
                    Just("&&"),
                    Just("||"),
                ],
                inner.clone()
            )
                .prop_map(|(l, op, r)| format!("({l}) {op} ({r})")),
            // unary minus
            inner.clone().prop_map(|e| format!("-({e})")),
            // one-argument functions
            (
                prop_oneof![Just("sin"), Just("cos"), Just("abs"), Just("sqrt"), Just("exp")],
                inner.clone()
            )
                .prop_map(|(f, e)| format!("{f}({e})")),
            // two-argument functions
            (
                prop_oneof![Just("min"), Just("max"), Just("pow"), Just("atan2")],
                inner.clone(),
                inner.clone()
            )
                .prop_map(|(f, l, r)| format!("{f}({l}, {r})")),
            // ternary
            (inner.clone(), inner.clone(), inner.clone())
                .prop_map(|(c, t, f)| format!("({c}) ? ({t}) : ({f})")),
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]
    #[test]
    #[cfg(feature = "libm")]
    fn compiled_matches_iterative(expr in arb_expr()) {
        assert_same(&expr);
    }
}

// ---------------------------------------------------------------------------
// Targeted semantics
// ---------------------------------------------------------------------------

#[test]
fn logical_results_normalize() {
    for expr in ["5 && 3", "0 && 3", "5 && 0", "0 || 7", "0 || 0", "2 || 0"] {
        assert_same(expr);
    }
}

#[test]
fn builtin_constants_resolve() {
    for expr in ["pi", "e", "tau", "PI + E + TAU"] {
        assert_same(expr);
    }
}

#[test]
fn param_shadows_context_variable() {
    // "x" exists in the context (3.25); a parameter of the same name must
    // win, exactly like the iterative override map.
    let ctx = make_ctx();
    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.add_parameter("x", 100.0).unwrap();
    e.add_expression("x + 1").unwrap();
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(101.0));
}

#[test]
fn context_variable_changes_between_evals() {
    let mut ctx = EvalContext::new();
    ctx.set_parameter("v", 1.0).unwrap();
    let mut ctx = Rc::new(ctx);

    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.add_expression("v * 10").unwrap();
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(10.0));

    // Change the context variable; slots must refill, not snapshot.
    Rc::get_mut(&mut ctx)
        .unwrap()
        .set_parameter("v", 2.0)
        .unwrap();
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(20.0));
}

#[test]
fn function_registered_after_first_eval_is_picked_up() {
    let mut ctx = Rc::new(EvalContext::new());

    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.add_expression("mystery(2)").unwrap();

    // Unknown function: both paths error.
    assert!(e.eval(&ctx).is_err());

    // Register it; fn_generation changes, programs recompile.
    Rc::get_mut(&mut ctx)
        .unwrap()
        .register_native_function("mystery", 1, |args| args[0] * 7.0)
        .unwrap();
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(14.0));
}

#[test]
fn shadowed_operator_uses_user_function() {
    let mut ctx = EvalContext::new();
    // Shadow "+" with something observable.
    ctx.register_native_function("+", 2, |args| args[0] + args[1] + 100.0)
        .unwrap();
    let ctx = Rc::new(ctx);

    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.add_expression("1 + 2").unwrap();
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(103.0));

    // And the shadow disables folding too: same result for constants.
    let arena2 = Bump::new();
    let mut e2 = Expression::new(&arena2);
    e2.add_expression("(1 + 2) * 1").unwrap();
    // "*" is still builtin; "+" is shadowed.
    e2.eval(&ctx).unwrap();
    assert_eq!(e2.get_result(0), Some(103.0));
}

#[test]
fn lazy_errors_in_untaken_branches() {
    // Unknown variable in the untaken branch: fine.
    assert_eq!(eval_compiled("1 ? 5 : zzz_unknown").unwrap(), 5.0);
    // Taken branch: error, same as iterative.
    assert_same("0 ? 5 : zzz_unknown");
    assert_same("1 ? zzz_unknown : 5");

    // Wrong arity in the untaken branch: fine (abs is builtin, arity 1).
    assert_eq!(eval_compiled("1 ? 5 : abs(1, 2)").unwrap(), 5.0);
    assert_same("0 ? 5 : abs(1, 2)");
}

#[test]
fn expression_functions_inline_and_recursion_falls_back() {
    let ctx = Rc::new(EvalContext::new());
    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.register_expression_function("dbl", &["u"], "u * 2").unwrap();
    e.register_expression_function("addm", &["p", "q"], "dbl(p) + q")
        .unwrap();
    e.add_parameter("a", 5.0).unwrap();
    e.add_expression("addm(a, 3)").unwrap(); // 5*2 + 3
    e.add_expression("dbl(dbl(a))").unwrap(); // 20
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(13.0));
    assert_eq!(e.get_result(1), Some(20.0));

    // Base-case recursion must still work (iterative fallback).
    let arena2 = Bump::new();
    let mut e2 = Expression::new(&arena2);
    e2.register_expression_function("count", &["n"], "n <= 0 ? 0 : count(n - 1) + 1")
        .unwrap();
    e2.add_expression("count(5)").unwrap();
    e2.eval(&ctx).unwrap();
    assert_eq!(e2.get_result(0), Some(5.0));
}

#[test]
fn function_param_shadows_everything_in_body() {
    let mut ctx = EvalContext::new();
    ctx.set_parameter("u", 1000.0).unwrap();
    let ctx = Rc::new(ctx);

    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.add_parameter("u", 500.0).unwrap();
    e.register_expression_function("probe", &["u"], "u + 1").unwrap();
    e.add_expression("probe(7)").unwrap();
    e.add_expression("u").unwrap(); // outside the function: the parameter
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(8.0));
    assert_eq!(e.get_result(1), Some(500.0));
}

#[test]
fn parent_chain_lookup() {
    let mut parent = EvalContext::new();
    parent.set_parameter("inherited", 42.0).unwrap();
    let mut child = EvalContext::new();
    child.parent = Some(Rc::new(parent));
    let ctx = Rc::new(child);

    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.add_expression("inherited + 1").unwrap();
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(43.0));
}

#[test]
fn set_param_after_fallback_reaches_iterative_path() {
    // A recursive function forces the whole expression onto the iterative
    // path; parameter updates must still reach it (override-map sync).
    let ctx = Rc::new(EvalContext::new());
    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.register_expression_function("count", &["n"], "n <= 0 ? 0 : count(n - 1) + 1")
        .unwrap();
    let p = e.add_parameter("k", 3.0).unwrap();
    e.add_expression("count(k)").unwrap();
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(3.0));

    e.set_param(p, 6.0).unwrap();
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(6.0));
}

#[test]
#[cfg(feature = "libm")]
fn constant_folding_matches_runtime() {
    for expr in [
        "2 + 3 * 4",
        "2 ^ 10",
        "1 / 0",
        "-(3) % 2",
        "(1 < 2) && (3 != 3)",
        "0 ? sin(1) : cos(0)",
        "sqrt(2) * sqrt(2)", // not folded (native call), still equal
    ] {
        assert_same(expr);
    }
}

#[test]
fn mixed_compiled_and_fallback_in_one_batch() {
    let ctx = Rc::new(EvalContext::new());
    let arena = Bump::new();
    let mut e = Expression::new(&arena);
    e.register_expression_function("count", &["n"], "n <= 0 ? 0 : count(n - 1) + 1")
        .unwrap();
    e.add_parameter("a", 4.0).unwrap();
    e.add_expression("a * 2").unwrap(); // compiled
    e.add_expression("count(a)").unwrap(); // fallback (recursive)
    e.add_expression("a + 100").unwrap(); // compiled
    e.eval(&ctx).unwrap();
    assert_eq!(e.get_result(0), Some(8.0));
    assert_eq!(e.get_result(1), Some(4.0));
    assert_eq!(e.get_result(2), Some(104.0));
}
