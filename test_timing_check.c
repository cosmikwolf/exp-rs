#include <stdio.h>
#include <time.h>
#include <unistd.h>

static double get_time_us() {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return ts.tv_sec * 1e6 + ts.tv_nsec / 1e3;
}

int main() {
    printf("Testing timing accuracy...\n");
    
    double start = get_time_us();
    usleep(100000); // Sleep for 100ms
    double end = get_time_us();
    
    printf("Expected: ~100,000 µs\n");
    printf("Measured: %.0f µs\n", end - start);
    
    // Test calculation
    int iterations = 10000 / 100;  // 100
    int batch_size = 100;
    double total_time_us = 162000; // 162ms from the test
    double total_evals = iterations * batch_size * 7;
    double us_per_batch = total_time_us / (iterations * batch_size);
    
    printf("\nC test calculation check:\n");
    printf("Iterations: %d\n", iterations);
    printf("Batch size: %d\n", batch_size); 
    printf("Total batches: %d\n", iterations * batch_size);
    printf("Total time: %.0f µs\n", total_time_us);
    printf("Time per batch: %.3f µs\n", us_per_batch);
    printf("This means evaluating 7 expressions takes: %.3f µs\n", us_per_batch);
    
    return 0;
}
