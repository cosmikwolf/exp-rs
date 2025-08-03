#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <math.h>
#include <stdint.h>
#include "exp_rs.h"

#ifdef __APPLE__
#include <mach/mach_time.h>
#endif

// Native function implementations
Real native_sin(const Real* args, uintptr_t nargs) { (void)nargs; return sin(args[0]); }
Real native_cos(const Real* args, uintptr_t nargs) { (void)nargs; return cos(args[0]); }
Real native_sqrt(const Real* args, uintptr_t nargs) { (void)nargs; return sqrt(args[0]); }
Real native_exp(const Real* args, uintptr_t nargs) { (void)nargs; return exp(args[0]); }
Real native_log(const Real* args, uintptr_t nargs) { (void)nargs; return log(args[0]); }
Real native_log10(const Real* args, uintptr_t nargs) { (void)nargs; return log10(args[0]); }
Real native_pow(const Real* args, uintptr_t nargs) { (void)nargs; return pow(args[0], args[1]); }
Real native_atan2(const Real* args, uintptr_t nargs) { (void)nargs; return atan2(args[0], args[1]); }
Real native_abs(const Real* args, uintptr_t nargs) { (void)nargs; return fabs(args[0]); }
Real native_sign(const Real* args, uintptr_t nargs) { 
    (void)nargs;
    return (args[0] > 0) ? 1.0 : (args[0] < 0) ? -1.0 : 0.0;
}
Real native_min(const Real* args, uintptr_t nargs) { (void)nargs; return args[0] < args[1] ? args[0] : args[1]; }
Real native_max(const Real* args, uintptr_t nargs) { (void)nargs; return args[0] > args[1] ? args[0] : args[1]; }
Real native_fmod(const Real* args, uintptr_t nargs) { (void)nargs; return fmod(args[0], args[1]); }

// High-resolution timer
typedef struct {
    uint64_t start;
    uint64_t end;
#ifdef __APPLE__
    mach_timebase_info_data_t timebase;
#endif
} Timer;

void timer_init(Timer* timer) {
#ifdef __APPLE__
    mach_timebase_info(&timer->timebase);
#endif
}

void timer_start(Timer* timer) {
#ifdef __APPLE__
    timer->start = mach_absolute_time();
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    timer->start = ts.tv_sec * 1000000000ULL + ts.tv_nsec;
#endif
}

void timer_stop(Timer* timer) {
#ifdef __APPLE__
    timer->end = mach_absolute_time();
#else
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    timer->end = ts.tv_sec * 1000000000ULL + ts.tv_nsec;
#endif
}

double timer_elapsed_ns(Timer* timer) {
#ifdef __APPLE__
    uint64_t elapsed = timer->end - timer->start;
    return (double)elapsed * timer->timebase.numer / timer->timebase.denom;
#else
    return (double)(timer->end - timer->start);
#endif
}

double timer_elapsed_us(Timer* timer) {
    return timer_elapsed_ns(timer) / 1000.0;
}

// Create and configure a context
EvalContextOpaque* create_test_context() {
    EvalContextOpaque* ctx = exp_rs_context_new();
    
    // Register functions
    exp_rs_context_register_native_function(ctx, "sin", 1, native_sin);
    exp_rs_context_register_native_function(ctx, "cos", 1, native_cos);
    exp_rs_context_register_native_function(ctx, "sqrt", 1, native_sqrt);
    exp_rs_context_register_native_function(ctx, "exp", 1, native_exp);
    exp_rs_context_register_native_function(ctx, "log", 1, native_log);
    exp_rs_context_register_native_function(ctx, "log10", 1, native_log10);
    exp_rs_context_register_native_function(ctx, "pow", 2, native_pow);
    exp_rs_context_register_native_function(ctx, "atan2", 2, native_atan2);
    exp_rs_context_register_native_function(ctx, "abs", 1, native_abs);
    exp_rs_context_register_native_function(ctx, "sign", 1, native_sign);
    exp_rs_context_register_native_function(ctx, "min", 2, native_min);
    exp_rs_context_register_native_function(ctx, "max", 2, native_max);
    exp_rs_context_register_native_function(ctx, "fmod", 2, native_fmod);
    
    return ctx;
}

// Measure a single operation multiple times and return average
double measure_operation(Timer* timer, void (*operation)(void* data), void* data, int iterations) {
    // Warm up
    for (int i = 0; i < 10; i++) {
        operation(data);
    }
    
    // Measure
    timer_start(timer);
    for (int i = 0; i < iterations; i++) {
        operation(data);
    }
    timer_stop(timer);
    
    return timer_elapsed_us(timer) / iterations;
}

// Operation functions
void op_create_context(void* data) {
    (void)data;
    EvalContextOpaque* ctx = create_test_context();
    exp_rs_context_free(ctx);
}

void op_create_builder(void* data) {
    (void)data;
    BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
    exp_rs_batch_builder_free(builder);
}

typedef struct {
    BatchBuilderOpaque* builder;
    const char** param_names;
    double* param_values;
} ParamData;

void op_add_parameters(void* data) {
    ParamData* pd = (ParamData*)data;
    for (int i = 0; i < 10; i++) {
        exp_rs_batch_builder_add_parameter(pd->builder, pd->param_names[i], pd->param_values[i]);
    }
}

typedef struct {
    BatchBuilderOpaque* builder;
    const char** expressions;
} ExprData;

void op_add_expressions(void* data) {
    ExprData* ed = (ExprData*)data;
    for (int i = 0; i < 7; i++) {
        exp_rs_batch_builder_add_expression(ed->builder, ed->expressions[i]);
    }
}

int main() {
    printf("=== C FFI Setup Time Analysis (High Precision) ===\n\n");
    
    Timer timer;
    timer_init(&timer);
    
    const char* expressions[] = {
        "a*sin(b*3.14159/180) + c*cos(d*3.14159/180) + sqrt(e*e + f*f)",
        "exp(g/10) * log(h+1) + pow(i, 0.5) * j",
        "((a > 5) && (b < 10)) * c + ((d >= e) || (f != g)) * h + min(i, j)",
        "sqrt(pow(a-e, 2) + pow(b-f, 2)) + atan2(c-g, d-h) * (i+j)/2",
        "abs(a-b) * sign(c-d) + max(e, f) * min(g, h) + fmod(i*j, 10)",
        "(a+b+c)/3 * sin((d+e+f)*3.14159/6) + log10(g*h+1) - exp(-i*j/100)",
        "a + b * c - d / (e + 0.001) + pow(f, g) * h - i + j"
    };
    
    const char* param_names[] = {"a", "b", "c", "d", "e", "f", "g", "h", "i", "j"};
    double param_values[] = {1.5, 3.0, 4.5, 6.0, 7.5, 9.0, 10.5, 12.0, 13.5, 15.0};
    
    // 1. Context Creation Time
    printf("1. Context Creation Time\n");
    double ctx_us = measure_operation(&timer, op_create_context, NULL, 1000);
    printf("   Average time: %.3f µs\n", ctx_us);
    
    // Create one context for subsequent tests
    EvalContextOpaque* ctx = create_test_context();
    
    // 2. BatchBuilder Creation Time
    printf("\n2. BatchBuilder Creation Time\n");
    double builder_us = measure_operation(&timer, op_create_builder, NULL, 10000);
    printf("   Average time: %.3f µs\n", builder_us);
    
    // 3. Adding Parameters (measure in isolation)
    printf("\n3. Adding 10 Parameters\n");
    double total_param_time = 0.0;
    
    for (int iter = 0; iter < 1000; iter++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        ParamData pd = {builder, param_names, param_values};
        timer_start(&timer);
        op_add_parameters(&pd);
        timer_stop(&timer);
        
        total_param_time += timer_elapsed_us(&timer);
        exp_rs_batch_builder_free(builder);
    }
    
    double param_us = total_param_time / 1000.0;
    printf("   Average time: %.3f µs\n", param_us);
    printf("   Per parameter: %.3f µs\n", param_us / 10.0);
    
    // 4. Parsing and Adding Expressions (measure in isolation)
    printf("\n4. Parsing and Adding 7 Expressions\n");
    double total_expr_time = 0.0;
    
    for (int iter = 0; iter < 1000; iter++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters first
        for (int i = 0; i < 10; i++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
        }
        
        ExprData ed = {builder, expressions};
        timer_start(&timer);
        op_add_expressions(&ed);
        timer_stop(&timer);
        
        total_expr_time += timer_elapsed_us(&timer);
        exp_rs_batch_builder_free(builder);
    }
    
    double expr_us = total_expr_time / 1000.0;
    printf("   Average time: %.3f µs\n", expr_us);
    printf("   Per expression: %.3f µs\n", expr_us / 7.0);
    
    // 5. First evaluation time (measure separately)
    printf("\n5. First Evaluation Time\n");
    double total_first_eval_time = 0.0;
    
    for (int iter = 0; iter < 1000; iter++) {
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Setup
        for (int i = 0; i < 10; i++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
        }
        for (int i = 0; i < 7; i++) {
            exp_rs_batch_builder_add_expression(builder, expressions[i]);
        }
        
        // Measure first eval
        timer_start(&timer);
        exp_rs_batch_builder_eval(builder, ctx);
        timer_stop(&timer);
        
        total_first_eval_time += timer_elapsed_us(&timer);
        exp_rs_batch_builder_free(builder);
    }
    
    double first_eval_us = total_first_eval_time / 1000.0;
    printf("   Average time: %.3f µs\n", first_eval_us);
    
    // 6. Complete Setup Time
    printf("\n6. Complete Setup Time\n");
    printf("   (Context + Builder + 10 params + 7 expressions + first eval)\n");
    
    double total_setup_time = 0.0;
    
    for (int iter = 0; iter < 100; iter++) {
        timer_start(&timer);
        
        // Complete setup
        EvalContextOpaque* test_ctx = create_test_context();
        BatchBuilderOpaque* builder = exp_rs_batch_builder_new();
        
        // Add parameters
        for (int i = 0; i < 10; i++) {
            exp_rs_batch_builder_add_parameter(builder, param_names[i], param_values[i]);
        }
        
        // Add expressions
        for (int i = 0; i < 7; i++) {
            exp_rs_batch_builder_add_expression(builder, expressions[i]);
        }
        
        // First evaluation
        exp_rs_batch_builder_eval(builder, test_ctx);
        
        timer_stop(&timer);
        total_setup_time += timer_elapsed_us(&timer);
        
        exp_rs_batch_builder_free(builder);
        exp_rs_context_free(test_ctx);
    }
    
    double total_us = total_setup_time / 100.0;
    printf("   Average time: %.3f µs\n", total_us);
    
    // 7. Breakdown
    printf("\n7. Setup Time Breakdown\n");
    printf("   Context creation:    %7.3f µs (%5.1f%%)\n", ctx_us, (ctx_us / total_us) * 100.0);
    printf("   Builder creation:    %7.3f µs (%5.1f%%)\n", builder_us, (builder_us / total_us) * 100.0);
    printf("   Add parameters:      %7.3f µs (%5.1f%%)\n", param_us, (param_us / total_us) * 100.0);
    printf("   Parse expressions:   %7.3f µs (%5.1f%%)\n", expr_us, (expr_us / total_us) * 100.0);
    printf("   First evaluation:    %7.3f µs (%5.1f%%)\n", first_eval_us, (first_eval_us / total_us) * 100.0);
    double overhead = total_us - (ctx_us + builder_us + param_us + expr_us + first_eval_us);
    printf("   Measurement overhead:%7.3f µs (%5.1f%%)\n", overhead, (overhead / total_us) * 100.0);
    printf("   ──────────────────────────────────────────\n");
    printf("   Total:               %7.3f µs\n", total_us);
    
    // 8. Subsequent evaluation time for comparison
    printf("\n8. Subsequent Evaluation Time (for comparison)\n");
    BatchBuilderOpaque* test_builder = exp_rs_batch_builder_new();
    
    // Setup
    for (int i = 0; i < 10; i++) {
        exp_rs_batch_builder_add_parameter(test_builder, param_names[i], param_values[i]);
    }
    for (int i = 0; i < 7; i++) {
        exp_rs_batch_builder_add_expression(test_builder, expressions[i]);
    }
    
    // First eval
    exp_rs_batch_builder_eval(test_builder, ctx);
    
    // Measure subsequent evals
    timer_start(&timer);
    for (int i = 0; i < 10000; i++) {
        // Update params
        for (int p = 0; p < 10; p++) {
            exp_rs_batch_builder_set_param(test_builder, p, param_values[p] + i * 0.001);
        }
        // Eval
        exp_rs_batch_builder_eval(test_builder, ctx);
    }
    timer_stop(&timer);
    
    double eval_us = timer_elapsed_us(&timer) / 10000.0;
    printf("   Average time: %.3f µs\n", eval_us);
    
    exp_rs_batch_builder_free(test_builder);
    
    // 9. Amortization Analysis
    printf("\n9. Amortization Analysis\n");
    printf("   Setup cost: %.3f µs\n", total_us);
    printf("   Per-evaluation cost: %.3f µs\n", eval_us);
    printf("\n");
    int breakeven = (int)(total_us / eval_us) + 1;
    printf("   Break-even point: %d evaluations\n", breakeven);
    printf("   At 1000 Hz for 1 second:\n");
    printf("     Setup overhead: %.3f%%\n", (total_us / 1e6) * 100.0);
    printf("     Amortized setup cost: %.3f µs per evaluation\n", total_us / 1000.0);
    
    // Cleanup
    exp_rs_context_free(ctx);
    
    return 0;
}