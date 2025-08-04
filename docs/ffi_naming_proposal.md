# FFI Function Naming Proposal

## Current vs Proposed Names

### Core Evaluation Functions
| Current | Proposed | Rationale |
|---------|----------|-----------|
| `exp_rs_eval` | `expr_eval` | Shorter, clearer |
| `exp_rs_free_error` | `expr_free_error` | Consistent prefix |

### Context Management
| Current | Proposed | Rationale |
|---------|----------|-----------|
| `exp_rs_context_new` | `expr_ctx_new` | Shorter "ctx" abbreviation |
| `exp_rs_context_free` | `expr_ctx_free` | Consistent |
| `exp_rs_context_eval` | `expr_ctx_eval` | Consistent |
| `exp_rs_context_set_parameter` | `expr_ctx_set_var` | "var" is shorter and clearer |

### Function Registration
| Current | Proposed | Rationale |
|---------|----------|-----------|
| `exp_rs_context_register_expression_function` | `expr_ctx_add_func` | Much shorter |
| `exp_rs_context_register_native_function` | `expr_ctx_add_native_func` | Clearer |
| `exp_rs_context_unregister_expression_function` | `expr_ctx_remove_func` | Shorter |

### Expression API (New Primary Interface)
| Current | Proposed | Rationale |
|---------|----------|-----------|
| N/A | `expr_new` | Create expression object |
| N/A | `expr_free` | Free expression object |
| N/A | `expr_parse` | Parse expression string |
| N/A | `expr_add_var` | Add variable/parameter |
| N/A | `expr_set_var` | Set variable value |
| N/A | `expr_eval_single` | Evaluate single expression |
| N/A | `expr_eval_multi` | Evaluate multiple expressions |
| N/A | `expr_get_result` | Get result by index |

### Batch Operations (Legacy)
| Current | Proposed | Rationale |
|---------|----------|-----------|
| `exp_rs_batch_builder_new` | `expr_batch_new` | Shorter |
| `exp_rs_batch_builder_free` | `expr_batch_free` | Consistent |
| `exp_rs_batch_builder_add_expression` | `expr_batch_add` | Shorter |
| `exp_rs_batch_builder_add_parameter` | `expr_batch_add_var` | Consistent |
| `exp_rs_batch_builder_set_param` | `expr_batch_set_var` | Consistent |
| `exp_rs_batch_builder_eval` | `expr_batch_eval` | Shorter |
| `exp_rs_batch_builder_get_result` | `expr_batch_result` | Shorter |

### Arena Management (Advanced Users)
| Current | Proposed | Rationale |
|---------|----------|-----------|
| `exp_rs_arena_new` | `expr_arena_new` | Consistent prefix |
| `exp_rs_arena_free` | `expr_arena_free` | Consistent |
| `exp_rs_arena_reset` | `expr_arena_reset` | Consistent |

## Naming Principles

1. **Short prefix**: Use `expr_` instead of `exp_rs_`
2. **Common abbreviations**: 
   - `ctx` for context
   - `var` for variable/parameter
   - `func` for function
3. **Action-first**: `add`, `set`, `get`, `eval`, `free`
4. **Avoid redundancy**: Don't repeat "expression" or "builder"

## Benefits

1. **Shorter names**: Average reduction of 30-50% in name length
2. **Clearer hierarchy**: Consistent use of underscore separators
3. **Better autocomplete**: Grouped by functionality (expr_ctx_*, expr_batch_*, etc.)
4. **Easier to remember**: Consistent patterns

## Migration Strategy

1. Add new names as aliases first
2. Deprecate old names with warnings
3. Update documentation and examples
4. Remove old names in major version bump

## Example Usage Comparison

### Old API:
```c
ExpContext* ctx = exp_rs_context_new();
exp_rs_context_set_parameter(ctx, "x", 10.0);
exp_rs_context_register_expression_function(ctx, "square", params, 1, "x*x");
EvalResult result = exp_rs_context_eval("square(x)", ctx);
exp_rs_context_free(ctx);
```

### New API:
```c
ExprContext* ctx = expr_ctx_new();
expr_ctx_set_var(ctx, "x", 10.0);
expr_ctx_add_func(ctx, "square", params, 1, "x*x");
ExprResult result = expr_ctx_eval("square(x)", ctx);
expr_ctx_free(ctx);
```

### New Expression API:
```c
Expr* expr = expr_new();
expr_parse(expr, "x^2 + y");
expr_add_var(expr, "x", 3.0);
expr_add_var(expr, "y", 4.0);
double result = expr_eval_single(expr, NULL);
expr_free(expr);
```