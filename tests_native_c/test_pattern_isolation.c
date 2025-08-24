#include <stdio.h>
#include <math.h>
#include "../include/exp_rs.h"

// Test pattern-based architecture where different channels have different functions
int main() {
    printf("=== Pattern Isolation Test ===\n\n");
    
    // Create shared context with common functions
    ExprContext* ctx = expr_context_new();
    if (!ctx) {
        printf("Failed to create context\n");
        return 1;
    }
    
    // Add some shared functions to context
    printf("1. Setting up shared context functions:\n");
    expr_context_add_expression_function(ctx, "normalize", "x,min,max", "(x-min)/(max-min)");
    expr_context_add_expression_function(ctx, "clamp", "x,min,max", "min(max(x,min),max)");
    printf("   - Added shared functions: normalize, clamp\n");
    
    // Create arena for all patterns
    // Arena is now managed internally by batch
    
    // Simulate different pattern channels with their own functions
    printf("\n2. Creating pattern channels with unique functions:\n");
    
    // Pattern 1: Audio processing channel
    ExprBatch* audio_pattern = expr_batch_new(16384);
    expr_batch_add_expression_function(audio_pattern, "amplify", "signal,gain", "signal*gain");
    expr_batch_add_expression_function(audio_pattern, "fade", "signal,factor", "signal*factor");
    expr_batch_add_expression_function(audio_pattern, "mix", "a,b,ratio", "a*(1-ratio)+b*ratio");
    printf("   - Audio pattern: amplify, fade, mix\n");
    
    // Pattern 2: Video processing channel  
    ExprBatch* video_pattern = expr_batch_new(16384);
    expr_batch_add_expression_function(video_pattern, "brightness", "pixel,factor", "min(pixel*factor,1)");
    expr_batch_add_expression_function(video_pattern, "contrast", "pixel,factor", "((pixel-0.5)*factor)+0.5");
    expr_batch_add_expression_function(video_pattern, "gamma", "pixel,g", "pixel^(1/g)");
    printf("   - Video pattern: brightness, contrast, gamma\n");
    
    // Pattern 3: Sensor data processing
    ExprBatch* sensor_pattern = expr_batch_new(16384);
    expr_batch_add_expression_function(sensor_pattern, "smooth", "prev,curr,alpha", "prev*(1-alpha)+curr*alpha");
    expr_batch_add_expression_function(sensor_pattern, "threshold", "value,thresh", "value>thresh?1:0");
    expr_batch_add_expression_function(sensor_pattern, "scale", "value,factor,offset", "value*factor+offset");
    printf("   - Sensor pattern: smooth, threshold, scale\n");
    
    // Test 3: Each pattern uses its own functions
    printf("\n3. Testing pattern-specific functions:\n");
    
    // Audio pattern tests
    expr_batch_add_variable(audio_pattern, "input", 0.5);
    expr_batch_add_expression(audio_pattern, "amplify(input, 2.0)");
    expr_batch_add_expression(audio_pattern, "fade(input, 0.7)");
    expr_batch_add_expression(audio_pattern, "mix(0.3, 0.8, 0.4)");
    expr_batch_evaluate(audio_pattern, ctx);
    
    printf("   Audio results:\n");
    printf("     - amplify(0.5, 2.0) = %.2f (expected 1.00)\n", expr_batch_get_result(audio_pattern, 0));
    printf("     - fade(0.5, 0.7) = %.2f (expected 0.35)\n", expr_batch_get_result(audio_pattern, 1));
    printf("     - mix(0.3, 0.8, 0.4) = %.2f (expected 0.50)\n", expr_batch_get_result(audio_pattern, 2));
    
    // Video pattern tests
    expr_batch_add_variable(video_pattern, "pixel", 0.6);
    expr_batch_add_expression(video_pattern, "brightness(pixel, 1.5)");
    expr_batch_add_expression(video_pattern, "contrast(pixel, 2.0)");
    expr_batch_add_expression(video_pattern, "gamma(0.5, 2.2)");
    expr_batch_evaluate(video_pattern, ctx);
    
    printf("\n   Video results:\n");
    printf("     - brightness(0.6, 1.5) = %.2f (expected 0.90)\n", expr_batch_get_result(video_pattern, 0));
    printf("     - contrast(0.6, 2.0) = %.2f (expected 0.70)\n", expr_batch_get_result(video_pattern, 1));
    printf("     - gamma(0.5, 2.2) = %.2f (expected 0.73)\n", expr_batch_get_result(video_pattern, 2));
    
    // Sensor pattern tests
    expr_batch_add_variable(sensor_pattern, "prev", 10.0);
    expr_batch_add_variable(sensor_pattern, "curr", 12.0);
    expr_batch_add_expression(sensor_pattern, "smooth(prev, curr, 0.3)");
    expr_batch_add_expression(sensor_pattern, "threshold(curr, 11.0)");
    expr_batch_add_expression(sensor_pattern, "scale(curr, 0.1, -0.2)");
    expr_batch_evaluate(sensor_pattern, ctx);
    
    printf("\n   Sensor results:\n");
    printf("     - smooth(10, 12, 0.3) = %.2f (expected 10.60)\n", expr_batch_get_result(sensor_pattern, 0));
    printf("     - threshold(12, 11) = %.2f (expected 1.00)\n", expr_batch_get_result(sensor_pattern, 1));
    printf("     - scale(12, 0.1, -0.2) = %.2f (expected 1.00)\n", expr_batch_get_result(sensor_pattern, 2));
    
    // Test 4: Verify isolation - patterns can't access each other's functions
    printf("\n4. Testing function isolation between patterns:\n");
    
    // Try to use audio function in video pattern (should fail during eval)
    expr_batch_add_expression(video_pattern, "amplify(0.5, 2.0)");
    int result = expr_batch_evaluate(video_pattern, ctx);
    printf("   - Video pattern trying audio function: %s\n", 
           result != 0 ? "failed as expected" : "unexpectedly succeeded");
    
    // Try to use video function in sensor pattern (should fail during eval)
    expr_batch_add_expression(sensor_pattern, "brightness(0.5, 2.0)");
    result = expr_batch_evaluate(sensor_pattern, ctx);
    printf("   - Sensor pattern trying video function: %s\n",
           result != 0 ? "failed as expected" : "unexpectedly succeeded");
    
    // Test 5: Shared context functions are available to all
    printf("\n5. Testing shared context functions:\n");
    
    // All patterns can use normalize from context
    expr_batch_add_expression(audio_pattern, "normalize(5, 0, 10)");
    expr_batch_add_expression(video_pattern, "normalize(128, 0, 255)");
    expr_batch_add_expression(sensor_pattern, "normalize(25, 20, 30)");
    
    expr_batch_evaluate(audio_pattern, ctx);
    expr_batch_evaluate(video_pattern, ctx);
    expr_batch_evaluate(sensor_pattern, ctx);
    
    // We need to track the indices manually since there's no get_expression_count function
    // Audio has 3 initial expressions + 1 normalize = index 3
    // Video has 3 initial expressions + 1 failed + 1 normalize = index 4  
    // Sensor has 3 initial expressions + 1 failed + 1 normalize = index 4
    
    printf("   - Audio: normalize(5, 0, 10) = %.2f (expected 0.50)\n", 
           expr_batch_get_result(audio_pattern, 3));
    printf("   - Video: normalize(128, 0, 255) = %.2f (expected 0.50)\n",
           expr_batch_get_result(video_pattern, 4));
    printf("   - Sensor: normalize(25, 20, 30) = %.2f (expected 0.50)\n",
           expr_batch_get_result(sensor_pattern, 4));
    
    // Test 6: Override context function in one pattern
    printf("\n6. Testing function override in specific pattern:\n");
    
    // Override normalize in audio pattern to work differently
    expr_batch_add_expression_function(audio_pattern, "normalize", "x,min,max", 
                                      "2*((x-min)/(max-min))-1");  // Maps to [-1, 1] instead of [0, 1]
    
    expr_batch_add_expression(audio_pattern, "normalize(5, 0, 10)");
    expr_batch_add_expression(video_pattern, "normalize(5, 0, 10)");
    
    expr_batch_evaluate(audio_pattern, ctx);
    expr_batch_evaluate(video_pattern, ctx);
    
    // Audio has 4 expressions from before + 1 new normalize = index 4
    // Video has 5 expressions from before + 1 new normalize = index 5
    
    printf("   - Audio (overridden): normalize(5, 0, 10) = %.2f (expected 0.00)\n",
           expr_batch_get_result(audio_pattern, 4));
    printf("   - Video (original): normalize(5, 0, 10) = %.2f (expected 0.50)\n",
           expr_batch_get_result(video_pattern, 5));
    
    // Test 7: Dynamic pattern loading/unloading
    printf("\n7. Testing dynamic pattern loading:\n");
    
    // Create a new pattern dynamically
    ExprBatch* dynamic_pattern = expr_batch_new(16384);
    expr_batch_add_expression_function(dynamic_pattern, "process", "x", "x*x+1");
    expr_batch_add_expression(dynamic_pattern, "process(3)");
    expr_batch_evaluate(dynamic_pattern, ctx);
    
    printf("   - Dynamic pattern: process(3) = %.2f (expected 10.00)\n",
           expr_batch_get_result(dynamic_pattern, 0));
    
    // "Unload" (free) the dynamic pattern
    expr_batch_free(dynamic_pattern);
    printf("   - Dynamic pattern unloaded\n");
    
    // Clean up
    expr_batch_free(audio_pattern);
    expr_batch_free(video_pattern);
    expr_batch_free(sensor_pattern);
    expr_context_free(ctx);
    
    printf("\n=== Pattern Isolation Test Completed ===\n");
    
    return 0;
}