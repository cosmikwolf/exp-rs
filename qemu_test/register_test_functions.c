#include "register_test_functions.h"
#include "qemu_test_harness.h"
#include <string.h>

// Native function wrappers - these match the signature expected by exp-rs
static Real native_sin(const Real *args, uintptr_t nargs) {
    (void)nargs; // Unused
    return SIN_FUNC(args[0]);
}

static Real native_cos(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return COS_FUNC(args[0]);
}

static Real native_tan(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return TAN_FUNC(args[0]);
}

static Real native_asin(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return ASIN_FUNC(args[0]);
}

static Real native_acos(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return ACOS_FUNC(args[0]);
}

static Real native_atan(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return ATAN_FUNC(args[0]);
}

static Real native_atan2(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return ATAN2_FUNC(args[0], args[1]);
}

static Real native_sinh(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return SINH_FUNC(args[0]);
}

static Real native_cosh(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return COSH_FUNC(args[0]);
}

static Real native_tanh(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return TANH_FUNC(args[0]);
}

static Real native_exp(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return EXP_FUNC(args[0]);
}

static Real native_ln(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return LOG_FUNC(args[0]);
}

static Real native_log(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return LOG_FUNC(args[0]);
}

static Real native_log10(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return LOG10_FUNC(args[0]);
}

static Real native_log2(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return LOG2_FUNC(args[0]);
}

static Real native_sqrt(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return SQRT_FUNC(args[0]);
}

static Real native_pow(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return POW_FUNC(args[0], args[1]);
}

static Real native_abs(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return FABS_FUNC(args[0]);
}

static Real native_floor(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return FLOOR_FUNC(args[0]);
}

static Real native_ceil(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return CEIL_FUNC(args[0]);
}

static Real native_round(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return ROUND_FUNC(args[0]);
}

static Real native_min(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return args[0] < args[1] ? args[0] : args[1];
}

static Real native_max(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return args[0] > args[1] ? args[0] : args[1];
}

static Real native_hypot(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return SQRT_FUNC(args[0] * args[0] + args[1] * args[1]);
}

static Real native_fmod(const Real *args, uintptr_t nargs) {
    (void)nargs;
    return FMOD_FUNC(args[0], args[1]);
}

// Create a new test context with all math functions registered
struct ExprContext* create_test_context(void) {
    struct ExprContext* ctx = expr_context_new();
    if (!ctx) {
        qemu_printf("Failed to create context\n");
        return NULL;
    }
    
    register_test_math_functions(ctx);
    return ctx;
}

// Register all math functions with the given context
void register_test_math_functions(struct ExprContext* ctx) {
    if (!ctx) {
        qemu_printf("Error: NULL context provided\n");
        return;
    }
    
    // Trigonometric functions
    expr_context_add_function(ctx, "sin", 1, native_sin);
    expr_context_add_function(ctx, "cos", 1, native_cos);
    expr_context_add_function(ctx, "tan", 1, native_tan);
    expr_context_add_function(ctx, "asin", 1, native_asin);
    expr_context_add_function(ctx, "acos", 1, native_acos);
    expr_context_add_function(ctx, "atan", 1, native_atan);
    expr_context_add_function(ctx, "atan2", 2, native_atan2);
    
    // Hyperbolic functions
    expr_context_add_function(ctx, "sinh", 1, native_sinh);
    expr_context_add_function(ctx, "cosh", 1, native_cosh);
    expr_context_add_function(ctx, "tanh", 1, native_tanh);
    
    // Exponential and logarithmic functions
    expr_context_add_function(ctx, "exp", 1, native_exp);
    expr_context_add_function(ctx, "ln", 1, native_ln);
    expr_context_add_function(ctx, "log", 1, native_log);
    expr_context_add_function(ctx, "log10", 1, native_log10);
    expr_context_add_function(ctx, "log2", 1, native_log2);
    
    // Power and root functions
    expr_context_add_function(ctx, "sqrt", 1, native_sqrt);
    expr_context_add_function(ctx, "pow", 2, native_pow);
    expr_context_add_function(ctx, "^", 2, native_pow);  // Alias for pow
    
    // Rounding and absolute value functions
    expr_context_add_function(ctx, "abs", 1, native_abs);
    expr_context_add_function(ctx, "floor", 1, native_floor);
    expr_context_add_function(ctx, "ceil", 1, native_ceil);
    expr_context_add_function(ctx, "round", 1, native_round);
    
    // Min/max functions
    expr_context_add_function(ctx, "min", 2, native_min);
    expr_context_add_function(ctx, "max", 2, native_max);
    
    // Other functions
    expr_context_add_function(ctx, "hypot", 2, native_hypot);
    expr_context_add_function(ctx, "fmod", 2, native_fmod);
}