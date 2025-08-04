#!/bin/bash

# Script to update all C test files to use new FFI API

echo "Updating C test files to use new FFI API..."

# Function to update a single file
update_file() {
    local file="$1"
    echo "Updating $file..."
    
    # Update type names
    sed -i '' 's/EvalContextOpaque/ExprContext/g' "$file"
    sed -i '' 's/ArenaOpaque/ExprArena/g' "$file"
    sed -i '' 's/BatchBuilderOpaque/ExprBatch/g' "$file"
    
    # Update function names
    sed -i '' 's/expr_ctx_new/expr_context_new/g' "$file"
    sed -i '' 's/expr_ctx_free/expr_context_free/g' "$file"
    sed -i '' 's/expr_ctx_add_native_func/expr_context_add_function/g' "$file"
    sed -i '' 's/expr_batch_add(/expr_batch_add_expression(/g' "$file"
    sed -i '' 's/expr_batch_add_var/expr_batch_add_variable/g' "$file"
    sed -i '' 's/expr_batch_set_var/expr_batch_set_variable/g' "$file"
    sed -i '' 's/expr_batch_eval_builder/expr_batch_evaluate/g' "$file"
    sed -i '' 's/expr_batch_result/expr_batch_get_result/g' "$file"
}

# Update all C test files
for file in tests_native_c/*.c; do
    if [ -f "$file" ]; then
        update_file "$file"
    fi
done

echo "All files updated!"