// Example: Efficient multi-channel device with pattern loading/unloading
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include "../include/exp_rs.h"

#define NUM_CHANNELS 4
#define MAX_PATTERNS_PER_CHANNEL 10

// Pattern definition
typedef struct {
    char* name;
    char** expressions;
    size_t num_expressions;
    char** param_names;
    Real* param_values;
    size_t num_params;
    // Custom functions specific to this pattern
    struct {
        char* name;
        size_t arity;
        NativeFunc func;
    }* functions;
    size_t num_functions;
} Pattern;

// Channel state
typedef struct {
    ExprArena* arena;
    ExprBatch* batch;
    Pattern* current_pattern;
    // Channel-specific parameters that persist across patterns
    Real channel_gain;
    Real channel_offset;
} Channel;

// Device state
typedef struct {
    Channel channels[NUM_CHANNELS];
    ExprContext* shared_context;  // Shared functions across all channels
    ExprContext* pattern_contexts[NUM_CHANNELS]; // Pattern-specific functions
} Device;

// Example custom functions
Real custom_envelope(const Real* args, uintptr_t nargs) {
    // args[0] = time, args[1] = attack, args[2] = decay, args[3] = sustain, args[4] = release
    Real t = args[0];
    Real attack = args[1];
    Real decay = args[2];
    Real sustain = args[3];
    
    if (t < attack) {
        return t / attack;
    } else if (t < attack + decay) {
        return 1.0 - (1.0 - sustain) * ((t - attack) / decay);
    } else {
        return sustain;
    }
}

Real custom_oscillator(const Real* args, uintptr_t nargs) {
    // args[0] = frequency, args[1] = phase
    return sin(2.0 * M_PI * args[0] * args[1]);
}

// Initialize device with shared functions
Device* device_create() {
    Device* dev = malloc(sizeof(Device));
    if (!dev) return NULL;
    
    // Create shared context with common functions
    dev->shared_context = expr_context_new();
    if (!dev->shared_context) {
        free(dev);
        return NULL;
    }
    
    // Add common functions all patterns can use
    expr_context_add_function(dev->shared_context, "sin", 1, (NativeFunc)sin);
    expr_context_add_function(dev->shared_context, "cos", 1, (NativeFunc)cos);
    expr_context_add_function(dev->shared_context, "sqrt", 1, (NativeFunc)sqrt);
    expr_context_add_function(dev->shared_context, "envelope", 5, custom_envelope);
    expr_context_add_function(dev->shared_context, "osc", 2, custom_oscillator);
    
    // Initialize channels
    for (int i = 0; i < NUM_CHANNELS; i++) {
        // Create a reasonably sized arena for each channel
        // Size depends on pattern complexity - 32KB is usually plenty
        dev->channels[i].arena = expr_arena_new(32 * 1024);
        dev->channels[i].batch = NULL;
        dev->channels[i].current_pattern = NULL;
        dev->channels[i].channel_gain = 1.0;
        dev->channels[i].channel_offset = 0.0;
        
        // Create pattern-specific context for each channel
        dev->pattern_contexts[i] = expr_context_new();
    }
    
    return dev;
}

// Load a pattern into a channel
int channel_load_pattern(Device* dev, int channel_idx, Pattern* pattern) {
    if (channel_idx < 0 || channel_idx >= NUM_CHANNELS) return -1;
    
    Channel* ch = &dev->channels[channel_idx];
    
    // Clean up previous pattern
    if (ch->batch) {
        expr_batch_free(ch->batch);
        ch->batch = NULL;
    }
    
    // Reset arena for new pattern
    expr_arena_reset(ch->arena);
    
    // Clear previous pattern-specific functions
    if (ch->current_pattern) {
        // In a real implementation, you'd remove the functions
        // For now, we'll recreate the context
        expr_context_free(dev->pattern_contexts[channel_idx]);
        dev->pattern_contexts[channel_idx] = expr_context_new();
    }
    
    // Add pattern-specific functions
    for (size_t i = 0; i < pattern->num_functions; i++) {
        expr_context_add_function(
            dev->pattern_contexts[channel_idx],
            pattern->functions[i].name,
            pattern->functions[i].arity,
            pattern->functions[i].func
        );
    }
    
    // Create new batch
    ch->batch = expr_batch_new(ch->arena);
    if (!ch->batch) return -2;
    
    // Add all expressions from the pattern
    for (size_t i = 0; i < pattern->num_expressions; i++) {
        if (expr_batch_add_expression(ch->batch, pattern->expressions[i]) < 0) {
            return -3;
        }
    }
    
    // Add pattern parameters
    for (size_t i = 0; i < pattern->num_params; i++) {
        if (expr_batch_add_variable(ch->batch, pattern->param_names[i], pattern->param_values[i]) < 0) {
            return -4;
        }
    }
    
    // Add channel-specific parameters
    expr_batch_add_variable(ch->batch, "ch_gain", ch->channel_gain);
    expr_batch_add_variable(ch->batch, "ch_offset", ch->channel_offset);
    expr_batch_add_variable(ch->batch, "ch_num", (Real)channel_idx);
    
    ch->current_pattern = pattern;
    return 0;
}

// Evaluate all channels
void device_evaluate(Device* dev, Real* outputs) {
    for (int i = 0; i < NUM_CHANNELS; i++) {
        Channel* ch = &dev->channels[i];
        if (!ch->batch || !ch->current_pattern) {
            outputs[i] = 0.0;
            continue;
        }
        
        // Evaluate using both shared and pattern-specific contexts
        // In practice, you might merge contexts or evaluate twice
        expr_batch_evaluate(ch->batch, dev->shared_context);
        
        // Get the first expression result as channel output
        outputs[i] = expr_batch_get_result(ch->batch, 0);
    }
}

// Update parameters efficiently
void device_update_params(Device* dev, int channel_idx, Real time_param) {
    if (channel_idx < 0 || channel_idx >= NUM_CHANNELS) return;
    
    Channel* ch = &dev->channels[channel_idx];
    if (!ch->batch) return;
    
    // Update time parameter (assuming it's always index 0)
    expr_batch_set_variable(ch->batch, 0, time_param);
}

// Clean up
void device_destroy(Device* dev) {
    if (!dev) return;
    
    for (int i = 0; i < NUM_CHANNELS; i++) {
        if (dev->channels[i].batch) {
            expr_batch_free(dev->channels[i].batch);
        }
        if (dev->channels[i].arena) {
            expr_arena_free(dev->channels[i].arena);
        }
        if (dev->pattern_contexts[i]) {
            expr_context_free(dev->pattern_contexts[i]);
        }
    }
    
    if (dev->shared_context) {
        expr_context_free(dev->shared_context);
    }
    
    free(dev);
}

// Example usage
int main() {
    // Create device
    Device* dev = device_create();
    if (!dev) {
        printf("Failed to create device\n");
        return 1;
    }
    
    // Create example patterns
    Pattern sine_pattern = {
        .name = "Simple Sine",
        .expressions = (char*[]){"sin(2 * pi * freq * time) * amp * ch_gain + ch_offset"},
        .num_expressions = 1,
        .param_names = (char*[]){"time", "freq", "amp", "pi"},
        .param_values = (Real[]){0.0, 440.0, 1.0, M_PI},
        .num_params = 4,
        .functions = NULL,
        .num_functions = 0
    };
    
    Pattern envelope_pattern = {
        .name = "Envelope Modulated",
        .expressions = (char*[]){
            "osc(freq, time) * envelope(time, attack, decay, sustain, release) * amp * ch_gain"
        },
        .num_expressions = 1,
        .param_names = (char*[]){"time", "freq", "amp", "attack", "decay", "sustain", "release"},
        .param_values = (Real[]){0.0, 440.0, 1.0, 0.1, 0.2, 0.7, 0.5},
        .num_params = 7,
        .functions = NULL,  // Using shared functions
        .num_functions = 0
    };
    
    // Load patterns into channels
    printf("Loading patterns...\n");
    channel_load_pattern(dev, 0, &sine_pattern);
    channel_load_pattern(dev, 1, &envelope_pattern);
    channel_load_pattern(dev, 2, &sine_pattern);  // Channel 2 uses sine
    channel_load_pattern(dev, 3, &envelope_pattern); // Channel 3 uses envelope
    
    // Simulate real-time evaluation
    Real outputs[NUM_CHANNELS];
    Real sample_rate = 48000.0;
    Real time_step = 1.0 / sample_rate;
    
    printf("\nSimulating 100ms of audio...\n");
    for (int sample = 0; sample < 4800; sample++) {
        Real time = sample * time_step;
        
        // Update time parameter for all channels
        for (int ch = 0; ch < NUM_CHANNELS; ch++) {
            device_update_params(dev, ch, time);
        }
        
        // Evaluate all channels
        device_evaluate(dev, outputs);
        
        // Print some samples
        if (sample % 480 == 0) {
            printf("t=%.3f: ", time);
            for (int ch = 0; ch < NUM_CHANNELS; ch++) {
                printf("Ch%d=%.3f ", ch, outputs[ch]);
            }
            printf("\n");
        }
    }
    
    // Change pattern on channel 0
    printf("\nChanging channel 0 to envelope pattern...\n");
    channel_load_pattern(dev, 0, &envelope_pattern);
    
    // Evaluate again
    device_update_params(dev, 0, 0.0);
    device_evaluate(dev, outputs);
    printf("After pattern change - Ch0: %.3f\n", outputs[0]);
    
    // Clean up
    device_destroy(dev);
    printf("\nDevice destroyed successfully\n");
    
    return 0;
}