#include "register_test_functions.h"
#include "qemu_test_harness.h"
#include <string.h>

// Native function wrappers - these match the signature expected by exp-rs
static Real native_sin(const Real *args, size_t nargs) {
    (void)nargs; // Unused
    return SIN_FUNC(args[0]);
}

static Real native_cos(const Real *args, size_t nargs) {
    (void)nargs;
    return COS_FUNC(args[0]);
}

static Real native_tan(const Real *args, size_t nargs) {
    (void)nargs;
    return TAN_FUNC(args[0]);
}

static Real native_asin(const Real *args, size_t nargs) {
    (void)nargs;
    return ASIN_FUNC(args[0]);
}

static Real native_acos(const Real *args, size_t nargs) {
    (void)nargs;
    return ACOS_FUNC(args[0]);
}

static Real native_atan(const Real *args, size_t nargs) {
    (void)nargs;
    return ATAN_FUNC(args[0]);
}

static Real native_atan2(const Real *args, size_t nargs) {
    (void)nargs;
    return ATAN2_FUNC(args[0], args[1]);
}

static Real native_sinh(const Real *args, size_t nargs) {
    (void)nargs;
    return SINH_FUNC(args[0]);
}

static Real native_cosh(const Real *args, size_t nargs) {
    (void)nargs;
    return COSH_FUNC(args[0]);
}

static Real native_tanh(const Real *args, size_t nargs) {
    (void)nargs;
    return TANH_FUNC(args[0]);
}

static Real native_exp(const Real *args, size_t nargs) {
    (void)nargs;
    return EXP_FUNC(args[0]);
}

static Real native_ln(const Real *args, size_t nargs) {
    (void)nargs;
    return LOG_FUNC(args[0]);
}

static Real native_log(const Real *args, size_t nargs) {
    (void)nargs;
    return LOG_FUNC(args[0]);
}

static Real native_log10(const Real *args, size_t nargs) {
    (void)nargs;
    return LOG10_FUNC(args[0]);
}

static Real native_log2(const Real *args, size_t nargs) {
    (void)nargs;
    return LOG2_FUNC(args[0]);
}

static Real native_sqrt(const Real *args, size_t nargs) {
    (void)nargs;
    return SQRT_FUNC(args[0]);
}

static Real native_pow(const Real *args, size_t nargs) {
    (void)nargs;
    return POW_FUNC(args[0], args[1]);
}

static Real native_abs(const Real *args, size_t nargs) {
    (void)nargs;
    return FABS_FUNC(args[0]);
}

static Real native_floor(const Real *args, size_t nargs) {
    (void)nargs;
    return FLOOR_FUNC(args[0]);
}

static Real native_ceil(const Real *args, size_t nargs) {
    (void)nargs;
    return CEIL_FUNC(args[0]);
}

static Real native_round(const Real *args, size_t nargs) {
    (void)nargs;
    return ROUND_FUNC(args[0]);
}

static Real native_min(const Real *args, size_t nargs) {
    (void)nargs;
    return args[0] < args[1] ? args[0] : args[1];
}

static Real native_max(const Real *args, size_t nargs) {
    (void)nargs;
    return args[0] > args[1] ? args[0] : args[1];
}

static Real native_hypot(const Real *args, size_t nargs) {
    (void)nargs;
    return SQRT_FUNC(args[0] * args[0] + args[1] * args[1]);
}

static Real native_fmod(const Real *args, size_t nargs) {
    (void)nargs;
    return FMOD_FUNC(args[0], args[1]);
}

// Create a new test context with all math functions registered
struct EvalContextOpaque* create_test_context(void) {
    struct EvalContextOpaque* ctx = exp_rs_context_new();
    if (!ctx) {
        qemu_printf("Failed to create context\n");
        return NULL;
    }
    
    register_test_math_functions(ctx);
    return ctx;
}

// Register all math functions with the given context
void register_test_math_functions(struct EvalContextOpaque* ctx) {
    qemu_printf("Registering math functions for testing...\n");
    
    if (!ctx) {
        qemu_printf("Error: NULL context provided\n");
        return;
    }
    
    // Trigonometric functions
    exp_rs_context_register_native_function(ctx, "sin", 1, (void*)native_sin);
    exp_rs_context_register_native_function(ctx, "cos", 1, (void*)native_cos);
    exp_rs_context_register_native_function(ctx, "tan", 1, (void*)native_tan);
    exp_rs_context_register_native_function(ctx, "asin", 1, (void*)native_asin);
    exp_rs_context_register_native_function(ctx, "acos", 1, (void*)native_acos);
    exp_rs_context_register_native_function(ctx, "atan", 1, (void*)native_atan);
    exp_rs_context_register_native_function(ctx, "atan2", 2, (void*)native_atan2);
    
    // Hyperbolic functions
    exp_rs_context_register_native_function(ctx, "sinh", 1, (void*)native_sinh);
    exp_rs_context_register_native_function(ctx, "cosh", 1, (void*)native_cosh);
    exp_rs_context_register_native_function(ctx, "tanh", 1, (void*)native_tanh);
    
    // Exponential and logarithmic functions
    exp_rs_context_register_native_function(ctx, "exp", 1, (void*)native_exp);
    exp_rs_context_register_native_function(ctx, "ln", 1, (void*)native_ln);
    exp_rs_context_register_native_function(ctx, "log", 1, (void*)native_log);
    exp_rs_context_register_native_function(ctx, "log10", 1, (void*)native_log10);
    exp_rs_context_register_native_function(ctx, "log2", 1, (void*)native_log2);
    
    // Power and root functions
    exp_rs_context_register_native_function(ctx, "sqrt", 1, (void*)native_sqrt);
    exp_rs_context_register_native_function(ctx, "pow", 2, (void*)native_pow);
    exp_rs_context_register_native_function(ctx, "^", 2, (void*)native_pow);  // Alias for pow
    
    // Rounding and absolute value functions
    exp_rs_context_register_native_function(ctx, "abs", 1, (void*)native_abs);
    exp_rs_context_register_native_function(ctx, "floor", 1, (void*)native_floor);
    exp_rs_context_register_native_function(ctx, "ceil", 1, (void*)native_ceil);
    exp_rs_context_register_native_function(ctx, "round", 1, (void*)native_round);
    
    // Min/max functions
    exp_rs_context_register_native_function(ctx, "min", 2, (void*)native_min);
    exp_rs_context_register_native_function(ctx, "max", 2, (void*)native_max);
    
    // Other functions
    exp_rs_context_register_native_function(ctx, "hypot", 2, (void*)native_hypot);
    exp_rs_context_register_native_function(ctx, "fmod", 2, (void*)native_fmod);
    
    qemu_printf("Math functions registered successfully\n");
}