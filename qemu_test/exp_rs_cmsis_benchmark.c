#include "qemu_test_harness.h"
#include <float.h> // For float comparisons
#include <math.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

// Include the generated header
#include "../include/exp_rs.h"

// Access the timer state from the harness
extern int timer_initialized;

// We will use CMSDK Dual Timer
// Note: DWT and SysTick are not reliable in the mps2-an500 QEMU machine

// Define common types and utilities for our tests
typedef Real real_t;
#if defined(DEF_USE_F32) || (defined(USE_F32) && !defined(USE_F64))
// typedef float real_t;
#define SIN sinf
#define COS cosf
#define SQRT sqrtf
#define FABS fabsf
#define TEST_NAME "F32"
#define FORMAT_SPEC "%.6f"

// Custom CMSIS-DSP function implementations
static inline float custom_arm_sin_f32(float x) { return sinf(x); }
#define ARM_SIN custom_arm_sin_f32

static inline float custom_arm_cos_f32(float x) { return cosf(x); }
#define ARM_COS custom_arm_cos_f32

static inline void custom_arm_sqrt_f32(float in, float *out) {
  *out = sqrtf(in);
}
#define ARM_SQRT(x, result) custom_arm_sqrt_f32(x, result)

#elif defined(DEF_USE_F64) || defined(USE_F64)
// typedef double real_t;
#define SIN sin
#define COS cos
#define SQRT sqrt
#define FABS fabs
#define TEST_NAME "F64"
#define FORMAT_SPEC "%.12f"

// Custom CMSIS-DSP function implementations
static inline double custom_arm_sin_f64(double x) { return sin(x); }
#define ARM_SIN custom_arm_sin_f64

static inline double custom_arm_cos_f64(double x) { return cos(x); }
#define ARM_COS custom_arm_cos_f64

static inline void custom_arm_sqrt_f64(double in, double *out) {
  *out = sqrt(in);
}
#define ARM_SQRT(x, result) custom_arm_sqrt_f64(x, result)

#else
#error "Neither USE_F32 nor USE_F64 is defined."
#endif

// Using the EvalResult struct directly

// Number of iterations for the benchmark
#define BENCHMARK_ITERATIONS 10000

// Helper to check approximate equality
static int approx_eq(real_t a, real_t b, real_t eps) {
// Check if both values are NaN (special case)
#if defined(DEF_USE_F32) || (defined(USE_F32) && !defined(USE_F64))
  if (isnanf(a) && isnanf(b)) {
    return 1; // Both NaN means they're "equal" for our purposes
  }
#elif defined(DEF_USE_F64) || defined(USE_F64)
  if (isnan(a) && isnan(b)) {
    return 1; // Both NaN means they're "equal" for our purposes
  }
#endif

  // Standard approximate equality
  return FABS(a - b) < eps;
}

// Benchmark expressions and parameters
typedef struct {
  const char *expression;
  real_t (*direct_func)(real_t, real_t);
} benchmark_expr_t;

// Add volatile to prevent compiler from optimizing away loops
// 1. a * sin(b) + cos(a+b)
static real_t __attribute__((noinline)) eval_expr_1(real_t a, real_t b) {
  volatile real_t sin_b = ARM_SIN(b);
  volatile real_t cos_ab = ARM_COS(a + b);
  return a * sin_b + cos_ab;
}

// 2. a * cos(b) + sin(a*b)
static real_t __attribute__((noinline)) eval_expr_2(real_t a, real_t b) {
  volatile real_t cos_b = ARM_COS(b);
  volatile real_t sin_ab = ARM_SIN(a * b);
  return a * cos_b + sin_ab;
}

// 3. sqrt(a*a + b*b) * sin(a+b)
static real_t __attribute__((noinline)) eval_expr_3(real_t a, real_t b) {
  real_t sum_squares = a * a + b * b;
  real_t hypot_result;
  ARM_SQRT(sum_squares, &hypot_result);
  volatile real_t sin_ab = ARM_SIN(a + b);
  return hypot_result * sin_ab;
}

// 4. sin(a) * cos(b) + tan(a*b)
static real_t __attribute__((noinline)) eval_expr_4(real_t a, real_t b) {
  volatile real_t sin_a = ARM_SIN(a);
  volatile real_t cos_b = ARM_COS(b);
  volatile real_t tan_ab = ARM_SIN(a * b) / ARM_COS(a * b);
  return sin_a * cos_b + tan_ab;
}

// 5. a^2 + b^2 - 2*a*b*cos(pi/4)
static real_t __attribute__((noinline)) eval_expr_5(real_t a, real_t b) {
  volatile real_t a_squared = a * a;
  volatile real_t b_squared = b * b;
  volatile real_t cos_pi_4 = ARM_COS(0.785398163397448); // π/4
  return a_squared + b_squared - 2 * a * b * cos_pi_4;
}

// 6. (a+b)^3 - (a^3 + 3*a^2*b + 3*a*b^2 + b^3)
static real_t __attribute__((noinline)) eval_expr_6(real_t a, real_t b) {
  volatile real_t sum = a + b;
  volatile real_t sum_cubed = sum * sum * sum;
  volatile real_t a_squared = a * a;
  volatile real_t b_squared = b * b;
  volatile real_t a_cubed = a_squared * a;
  volatile real_t b_cubed = b_squared * b;
  volatile real_t expanded =
      a_cubed + 3 * a_squared * b + 3 * a * b_squared + b_cubed;
  return sum_cubed - expanded;
}

// 7. exp(a*b) / (1 + exp(a*b))
static real_t __attribute__((noinline)) eval_expr_7(real_t a, real_t b) {
  volatile real_t exp_ab = exp(a * b);
  return exp_ab / (1 + exp_ab);
}

// 8. log(a+1) * sqrt(b+1)
static real_t __attribute__((noinline)) eval_expr_8(real_t a, real_t b) {
  volatile real_t log_a_plus_1 = log(a + 1);
  volatile real_t sqrt_b_plus_1_result;
  ARM_SQRT(b + 1, &sqrt_b_plus_1_result);
  return log_a_plus_1 * sqrt_b_plus_1_result;
}

// 9. pow(a, b) + pow(b, a)
static real_t __attribute__((noinline)) eval_expr_9(real_t a, real_t b) {
  volatile real_t pow_a_b = pow(a, b);
  volatile real_t pow_b_a = pow(b, a);
  return pow_a_b + pow_b_a;
}

// 10. sin(a)^2 + cos(a)^2
static real_t __attribute__((noinline)) eval_expr_10(real_t a, real_t b) {
  volatile real_t sin_a = ARM_SIN(a);
  volatile real_t cos_a = ARM_COS(a);
  return sin_a * sin_a + cos_a * cos_a;
}

// 11. floor(a+0.5) * ceil(b-0.3)
static real_t __attribute__((noinline)) eval_expr_11(real_t a, real_t b) {
  volatile real_t floor_a = floor(a + 0.5);
  volatile real_t ceil_b = ceil(b - 0.3);
  return floor_a * ceil_b;
}

// 12. max(a, b) + min(a*2, b/2)
static real_t __attribute__((noinline)) eval_expr_12(real_t a, real_t b) {
  volatile real_t max_val = a > b ? a : b;
  volatile real_t min_val = a * 2 < b / 2 ? a * 2 : b / 2;
  return max_val + min_val;
}

// 13. abs(a-b) * sin(a*b)
static real_t __attribute__((noinline)) eval_expr_13(real_t a, real_t b) {
  volatile real_t abs_diff = FABS(a - b);
  volatile real_t sin_prod = ARM_SIN(a * b);
  return abs_diff * sin_prod;
}

// 14. (a+b) * (a-b) / (a*a + b*b)
static real_t __attribute__((noinline)) eval_expr_14(real_t a, real_t b) {
  volatile real_t sum = a + b;
  volatile real_t diff = a - b;
  volatile real_t denom = a * a + b * b;
  if (denom == 0)
    return 0;
  return sum * diff / denom;
}

// 15. sin(pi*a) * cos(pi*b)
static real_t __attribute__((noinline)) eval_expr_15(real_t a, real_t b) {
#define PI 3.14159265358979323846
  volatile real_t sin_pi_a = ARM_SIN(PI * a);
  volatile real_t cos_pi_b = ARM_COS(PI * b);
  return sin_pi_a * cos_pi_b;
}

// 16. sqrt(1 - (a*a + b*b))
static real_t __attribute__((noinline)) eval_expr_16(real_t a, real_t b) {
  volatile real_t sum_squares = a * a + b * b;
  // Removed the check to match Rust behavior
  // The Rust sqrt function will return NaN for negative inputs
  volatile real_t sqrt_result;
  ARM_SQRT(1 - sum_squares, &sqrt_result);
  return sqrt_result;
}

// 17. a * exp(-b*b/2) / sqrt(2*pi)
static real_t __attribute__((noinline)) eval_expr_17(real_t a, real_t b) {
  // PI already defined above
  volatile real_t exp_term = exp(-b * b / 2);
  volatile real_t sqrt_2pi;
  ARM_SQRT(2 * PI, &sqrt_2pi);
  return a * exp_term / sqrt_2pi;
}

// 18. 1 / (1 + exp(-a*b))
static real_t __attribute__((noinline)) eval_expr_18(real_t a, real_t b) {
  volatile real_t exp_neg_ab = exp(-a * b);
  return 1 / (1 + exp_neg_ab);
}

// 19. a*a*a + b*b*b + 3*a*b*(a+b)
static real_t __attribute__((noinline)) eval_expr_19(real_t a, real_t b) {
  volatile real_t a_cubed = a * a * a;
  volatile real_t b_cubed = b * b * b;
  volatile real_t sum = a + b;
  volatile real_t product_term = 3 * a * b * sum;
  return a_cubed + b_cubed + product_term;
}

// 20. a * sin(b) + b * sin(a)
static real_t __attribute__((noinline)) eval_expr_20(real_t a, real_t b) {
  volatile real_t sin_b = ARM_SIN(b);
  volatile real_t sin_a = ARM_SIN(a);
  return a * sin_b + b * sin_a;
}

// 21. log10(a+10) * log10(b+10)
static real_t __attribute__((noinline)) eval_expr_21(real_t a, real_t b) {
  volatile real_t log10_a = log10(a + 10);
  volatile real_t log10_b = log10(b + 10);
  return log10_a * log10_b;
}

// 22. atan2(a, b) + atan2(b, a)
static real_t __attribute__((noinline)) eval_expr_22(real_t a, real_t b) {
  volatile real_t atan2_ab = atan2(a, b);
  volatile real_t atan2_ba = atan2(b, a);
  return atan2_ab + atan2_ba;
}

// 23. a*exp(b) + b*exp(a)
static real_t __attribute__((noinline)) eval_expr_23(real_t a, real_t b) {
  volatile real_t exp_b = exp(b);
  volatile real_t exp_a = exp(a);
  return a * exp_b + b * exp_a;
}

// 24. a/(1+a) + b/(1+b)
static real_t __attribute__((noinline)) eval_expr_24(real_t a, real_t b) {
  volatile real_t a_term = a / (1 + a);
  volatile real_t b_term = b / (1 + b);
  return a_term + b_term;
}

// 25. sqrt(a)*log(b) + sqrt(b)*log(a)
static real_t __attribute__((noinline)) eval_expr_25(real_t a, real_t b) {
  if (a <= 0 || b <= 0)
    return 0;
  volatile real_t sqrt_a;
  volatile real_t sqrt_b;
  ARM_SQRT(a, &sqrt_a);
  ARM_SQRT(b, &sqrt_b);
  volatile real_t log_a = log(a);
  volatile real_t log_b = log(b);
  return sqrt_a * log_b + sqrt_b * log_a;
}

// 26. sin(a+b) * cos(a-b)
static real_t __attribute__((noinline)) eval_expr_26(real_t a, real_t b) {
  volatile real_t sin_sum = ARM_SIN(a + b);
  volatile real_t cos_diff = ARM_COS(a - b);
  return sin_sum * cos_diff;
}

// 27. (a*a + b*b)^1.5
static real_t __attribute__((noinline)) eval_expr_27(real_t a, real_t b) {
  volatile real_t sum_squares = a * a + b * b;
  return pow(sum_squares, 1.5);
}

// 28. exp(-(a*a + b*b))
static real_t __attribute__((noinline)) eval_expr_28(real_t a, real_t b) {
  volatile real_t sum_squares = a * a + b * b;
  return exp(-sum_squares);
}

// 29. a*a / (a*a + b*b) * sin(a*b)
static real_t __attribute__((noinline)) eval_expr_29(real_t a, real_t b) {
  volatile real_t a_squared = a * a;
  volatile real_t sum_squares = a_squared + b * b;
  if (sum_squares == 0)
    return 0;
  volatile real_t sin_ab = ARM_SIN(a * b);
  return a_squared / sum_squares * sin_ab;
}

// 30. tanh(a*b) * sinh(a+b)
static real_t __attribute__((noinline)) eval_expr_30(real_t a, real_t b) {
  volatile real_t ab = a * b;
  volatile real_t tanh_ab = (exp(ab) - exp(-ab)) / (exp(ab) + exp(-ab));
  volatile real_t ab_sum = a + b;
  volatile real_t sinh_sum = (exp(ab_sum) - exp(-ab_sum)) / 2;
  return tanh_ab * sinh_sum;
}

// 31. a * asin(b/2) + b * acos(a/2)
static real_t __attribute__((noinline)) eval_expr_31(real_t a, real_t b) {
  if (FABS(b / 2) > 1 || FABS(a / 2) > 1)
    return 0;
  volatile real_t asin_term = asin(b / 2);
  volatile real_t acos_term = acos(a / 2);
  return a * asin_term + b * acos_term;
}

// 32. log(a*b) / (log(a) + log(b))
static real_t __attribute__((noinline)) eval_expr_32(real_t a, real_t b) {
  // Similar behavior as Rust - return NaN for negative or zero inputs
  if (a <= 0 || b <= 0) {
// Return NaN to match Rust's behavior
#if defined(DEF_USE_F32) || (defined(USE_F32) && !defined(USE_F64))
    return NAN; // f32 NaN
#elif defined(DEF_USE_F64) || defined(USE_F64)
    return NAN; // f64 NaN
#endif
  }

  volatile real_t log_a = log(a);
  volatile real_t log_b = log(b);
  volatile real_t denom = log_a + log_b;

  if (denom == 0) {
// Return NaN for zero denominator to match Rust
#if defined(DEF_USE_F32) || (defined(USE_F32) && !defined(USE_F64))
    return NAN; // f32 NaN
#elif defined(DEF_USE_F64) || defined(USE_F64)
    return NAN; // f64 NaN
#endif
  }

  volatile real_t log_product = log(a * b);
  return log_product / denom;
}

// 33. (a+b) / (1 + a*b)
static real_t __attribute__((noinline)) eval_expr_33(real_t a, real_t b) {
  volatile real_t sum = a + b;
  volatile real_t denom = 1 + a * b;
  if (denom == 0)
    return 0;
  return sum / denom;
}

// We don't need this function anymore since we call the benchmark functions
// directly

// Utility function to read expressions from a file
int read_benchmark_expressions(const char *filename,
                               benchmark_expr_t *expressions,
                               int max_expressions) {
  char buffer[256];
  int fd, count = 0;

  qemu_printf("Reading benchmark expressions from file: %s\n", filename);

  fd = qemu_file_open(filename, "r");
  if (fd < 0) {
    qemu_printf("Error: Could not open file %s (error code %d)\n", filename,
                fd);
    return -1;
  }

  while (count < max_expressions) {
    // Read a line from the file
    int i = 0;
    while (i < sizeof(buffer) - 1) {
      char c;
      if (qemu_file_read(fd, &c, 1) != 0)
        break; // EOF or error
      if (c == '\n')
        break;
      buffer[i++] = c;
    }
    buffer[i] = '\0';

    // If empty line, we're done
    if (i == 0)
      break;

    // Map expression to function
    if (strcmp(buffer, "a * sin(b) + cos(a+b)") == 0) {
      expressions[count].expression = "a * sin(b) + cos(a+b)";
      expressions[count].direct_func = eval_expr_1;
    } else if (strcmp(buffer, "a * cos(b) + sin(a*b)") == 0) {
      expressions[count].expression = "a * cos(b) + sin(a*b)";
      expressions[count].direct_func = eval_expr_2;
    } else if (strcmp(buffer, "sqrt(a*a + b*b) * sin(a+b)") == 0) {
      expressions[count].expression = "sqrt(a*a + b*b) * sin(a+b)";
      expressions[count].direct_func = eval_expr_3;
    } else if (strcmp(buffer, "sin(a) * cos(b) + tan(a*b)") == 0) {
      expressions[count].expression = "sin(a) * cos(b) + tan(a*b)";
      expressions[count].direct_func = eval_expr_4;
    } else if (strcmp(buffer, "a^2 + b^2 - 2*a*b*cos(pi/4)") == 0) {
      expressions[count].expression = "a^2 + b^2 - 2*a*b*cos(pi/4)";
      expressions[count].direct_func = eval_expr_5;
    } else if (strcmp(buffer, "(a+b)^3 - (a^3 + 3*a^2*b + 3*a*b^2 + b^3)") ==
               0) {
      expressions[count].expression =
          "(a+b)^3 - (a^3 + 3*a^2*b + 3*a*b^2 + b^3)";
      expressions[count].direct_func = eval_expr_6;
    } else if (strcmp(buffer, "exp(a*b) / (1 + exp(a*b))") == 0) {
      expressions[count].expression = "exp(a*b) / (1 + exp(a*b))";
      expressions[count].direct_func = eval_expr_7;
    } else if (strcmp(buffer, "log(a+1) * sqrt(b+1)") == 0) {
      expressions[count].expression = "log(a+1) * sqrt(b+1)";
      expressions[count].direct_func = eval_expr_8;
    } else if (strcmp(buffer, "pow(a, b) + pow(b, a)") == 0) {
      expressions[count].expression = "pow(a, b) + pow(b, a)";
      expressions[count].direct_func = eval_expr_9;
    } else if (strcmp(buffer, "sin(a)^2 + cos(a)^2") == 0) {
      expressions[count].expression = "sin(a)^2 + cos(a)^2";
      expressions[count].direct_func = eval_expr_10;
    } else if (strcmp(buffer, "floor(a+0.5) * ceil(b-0.3)") == 0) {
      expressions[count].expression = "floor(a+0.5) * ceil(b-0.3)";
      expressions[count].direct_func = eval_expr_11;
    } else if (strcmp(buffer, "max(a, b) + min(a*2, b/2)") == 0) {
      expressions[count].expression = "max(a, b) + min(a*2, b/2)";
      expressions[count].direct_func = eval_expr_12;
    } else if (strcmp(buffer, "abs(a-b) * sign(a*b)") == 0) {
      expressions[count].expression = "abs(a-b) * sign(a*b)";
      expressions[count].direct_func = eval_expr_13;
    } else if (strcmp(buffer, "(a+b) * (a-b) / (a*a + b*b)") == 0) {
      expressions[count].expression = "(a+b) * (a-b) / (a*a + b*b)";
      expressions[count].direct_func = eval_expr_14;
    } else if (strcmp(buffer, "sin(pi*a) * cos(pi*b)") == 0) {
      expressions[count].expression = "sin(pi*a) * cos(pi*b)";
      expressions[count].direct_func = eval_expr_15;
    } else if (strcmp(buffer, "sqrt(1 - (a*a + b*b))") == 0) {
      expressions[count].expression = "sqrt(1 - (a*a + b*b))";
      expressions[count].direct_func = eval_expr_16;
    } else if (strcmp(buffer, "a * exp(-b*b/2) / sqrt(2*pi)") == 0) {
      expressions[count].expression = "a * exp(-b*b/2) / sqrt(2*pi)";
      expressions[count].direct_func = eval_expr_17;
    } else if (strcmp(buffer, "1 / (1 + exp(-a*b))") == 0) {
      expressions[count].expression = "1 / (1 + exp(-a*b))";
      expressions[count].direct_func = eval_expr_18;
    } else if (strcmp(buffer, "a*a*a + b*b*b + 3*a*b*(a+b)") == 0) {
      expressions[count].expression = "a*a*a + b*b*b + 3*a*b*(a+b)";
      expressions[count].direct_func = eval_expr_19;
    } else if (strcmp(buffer, "a * sin(b) + b * sin(a)") == 0) {
      expressions[count].expression = "a * sin(b) + b * sin(a)";
      expressions[count].direct_func = eval_expr_20;
    } else if (strcmp(buffer, "log10(a+10) * log10(b+10)") == 0) {
      expressions[count].expression = "log10(a+10) * log10(b+10)";
      expressions[count].direct_func = eval_expr_21;
    } else if (strcmp(buffer, "atan2(a, b) + atan2(b, a)") == 0) {
      expressions[count].expression = "atan2(a, b) + atan2(b, a)";
      expressions[count].direct_func = eval_expr_22;
    } else if (strcmp(buffer, "a*exp(b) + b*exp(a)") == 0) {
      expressions[count].expression = "a*exp(b) + b*exp(a)";
      expressions[count].direct_func = eval_expr_23;
    } else if (strcmp(buffer, "a/(1+a) + b/(1+b)") == 0) {
      expressions[count].expression = "a/(1+a) + b/(1+b)";
      expressions[count].direct_func = eval_expr_24;
    } else if (strcmp(buffer, "sqrt(a)*log(b) + sqrt(b)*log(a)") == 0) {
      expressions[count].expression = "sqrt(a)*log(b) + sqrt(b)*log(a)";
      expressions[count].direct_func = eval_expr_25;
    } else if (strcmp(buffer, "sin(a+b) * cos(a-b)") == 0) {
      expressions[count].expression = "sin(a+b) * cos(a-b)";
      expressions[count].direct_func = eval_expr_26;
    } else if (strcmp(buffer, "(a*a + b*b)^1.5") == 0) {
      expressions[count].expression = "(a*a + b*b)^1.5";
      expressions[count].direct_func = eval_expr_27;
    } else if (strcmp(buffer, "exp(-(a*a + b*b))") == 0) {
      expressions[count].expression = "exp(-(a*a + b*b))";
      expressions[count].direct_func = eval_expr_28;
    } else if (strcmp(buffer, "a*a / (a*a + b*b) * sin(a*b)") == 0) {
      expressions[count].expression = "a*a / (a*a + b*b) * sin(a*b)";
      expressions[count].direct_func = eval_expr_29;
    } else if (strcmp(buffer, "tanh(a*b) * sinh(a+b)") == 0) {
      expressions[count].expression = "tanh(a*b) * sinh(a+b)";
      expressions[count].direct_func = eval_expr_30;
    } else if (strcmp(buffer, "a * asin(b/2) + b * acos(a/2)") == 0) {
      expressions[count].expression = "a * asin(b/2) + b * acos(a/2)";
      expressions[count].direct_func = eval_expr_31;
    } else if (strcmp(buffer, "log(a*b) / (log(a) + log(b))") == 0) {
      expressions[count].expression = "log(a*b) / (log(a) + log(b))";
      expressions[count].direct_func = eval_expr_32;
    } else if (strcmp(buffer, "(a+b) / (1 + a*b)") == 0) {
      expressions[count].expression = "(a+b) / (1 + a*b)";
      expressions[count].direct_func = eval_expr_33;
    } else {
      qemu_printf("Warning: Unknown expression '%s', skipping\n", buffer);
      continue;
    }

    qemu_printf("Added expression: %s\n", expressions[count].expression);
    count++;
  }

  qemu_file_close(fd);
  return count;
}

// Benchmark exp-rs expression evaluation vs direct C implementation
test_result_t benchmark_exp_rs_vs_direct() {
  // Initialize the timer for benchmarking
  init_hardware_timer();

  // Print which timer is being used
  const char *timer_type = "CMSDK Hardware Timer (32-bit)";

  qemu_printf(
      "Benchmarking exp-rs vs direct C implementation with %s mode using %s\n",
      TEST_NAME, timer_type);

  // Define expressions to benchmark - use all 33 hardcoded expressions
  benchmark_expr_t expressions[33];
  int num_exprs = 33;

  // Use hardcoded expressions for all 33 tests
  qemu_print("Using all 33 hardcoded benchmark expressions\n");

  expressions[0].expression = "a * sin(b) + cos(a+b)";
  expressions[0].direct_func = eval_expr_1;

  expressions[1].expression = "a * cos(b) + sin(a*b)";
  expressions[1].direct_func = eval_expr_2;

  expressions[2].expression = "sqrt(a*a + b*b) * sin(a+b)";
  expressions[2].direct_func = eval_expr_3;

  expressions[3].expression = "sin(a) * cos(b) + tan(a*b)";
  expressions[3].direct_func = eval_expr_4;

  expressions[4].expression = "a^2 + b^2 - 2*a*b*cos(pi/4)";
  expressions[4].direct_func = eval_expr_5;

  expressions[5].expression = "(a+b)^3 - (a^3 + 3*a^2*b + 3*a*b^2 + b^3)";
  expressions[5].direct_func = eval_expr_6;

  expressions[6].expression = "exp(a*b) / (1 + exp(a*b))";
  expressions[6].direct_func = eval_expr_7;

  expressions[7].expression = "log(a+1) * sqrt(b+1)";
  expressions[7].direct_func = eval_expr_8;

  expressions[8].expression = "pow(a, b) + pow(b, a)";
  expressions[8].direct_func = eval_expr_9;

  expressions[9].expression = "sin(a)^2 + cos(a)^2";
  expressions[9].direct_func = eval_expr_10;

  expressions[10].expression = "floor(a+0.5) * ceil(b-0.3)";
  expressions[10].direct_func = eval_expr_11;

  expressions[11].expression = "max(a, b) + min(a*2, b/2)";
  expressions[11].direct_func = eval_expr_12;

  expressions[12].expression = "abs(a-b) * sin(a*b)";
  expressions[12].direct_func = eval_expr_13;

  expressions[13].expression = "(a+b) * (a-b) / (a*a + b*b)";
  expressions[13].direct_func = eval_expr_14;

  expressions[14].expression = "sin(pi*a) * cos(pi*b)";
  expressions[14].direct_func = eval_expr_15;

  expressions[15].expression = "sqrt(1 - (a*a + b*b))";
  expressions[15].direct_func = eval_expr_16;

  expressions[16].expression = "a * exp(-b*b/2) / sqrt(2*pi)";
  expressions[16].direct_func = eval_expr_17;

  expressions[17].expression = "1 / (1 + exp(-a*b))";
  expressions[17].direct_func = eval_expr_18;

  expressions[18].expression = "a*a*a + b*b*b + 3*a*b*(a+b)";
  expressions[18].direct_func = eval_expr_19;

  expressions[19].expression = "a * sin(b) + b * sin(a)";
  expressions[19].direct_func = eval_expr_20;

  expressions[20].expression = "log10(a+10) * log10(b+10)";
  expressions[20].direct_func = eval_expr_21;

  expressions[21].expression = "atan2(a, b) + atan2(b, a)";
  expressions[21].direct_func = eval_expr_22;

  expressions[22].expression = "a*exp(b) + b*exp(a)";
  expressions[22].direct_func = eval_expr_23;

  expressions[23].expression = "a/(1+a) + b/(1+b)";
  expressions[23].direct_func = eval_expr_24;

  expressions[24].expression = "sqrt(a)*log(b) + sqrt(b)*log(a)";
  expressions[24].direct_func = eval_expr_25;

  expressions[25].expression = "sin(a+b) * cos(a-b)";
  expressions[25].direct_func = eval_expr_26;

  expressions[26].expression = "(a*a + b*b)^1.5";
  expressions[26].direct_func = eval_expr_27;

  expressions[27].expression = "exp(-(a*a + b*b))";
  expressions[27].direct_func = eval_expr_28;

  expressions[28].expression = "a*a / (a*a + b*b) * sin(a*b)";
  expressions[28].direct_func = eval_expr_29;

  expressions[29].expression = "tanh(a*b) * sinh(a+b)";
  expressions[29].direct_func = eval_expr_30;

  expressions[30].expression = "a * asin(b/2) + b * acos(a/2)";
  expressions[30].direct_func = eval_expr_31;

  expressions[31].expression = "log(a*b) / (log(a) + log(b))";
  expressions[31].direct_func = eval_expr_32;

  expressions[32].expression = "(a+b) / (1 + a*b)";
  expressions[32].direct_func = eval_expr_33;

  // Expressions are now loaded dynamically

  // Balance between accuracy and runtime with 60s timeout
  // With 33 expressions, we need to reduce iterations to stay within timeout
  int iterations = 1000;
  real_t param_values[] = {2.0, 0.5};

  // Create evaluation context
  struct EvalContextOpaque *ctx = exp_rs_context_new();
  if (!ctx) {
    qemu_print("Failed to create context\n");
    return TEST_FAIL;
  }

  // Set parameters
  exp_rs_context_set_parameter(ctx, "a", param_values[0]);
  exp_rs_context_set_parameter(ctx, "b", param_values[1]);

  // Simple warm-up run to ensure consistent timing
  qemu_printf("Running warm-up phase...\n");

  // Do a thorough warm-up to get consistent timing
  for (int warmup_round = 0; warmup_round < 3; warmup_round++) {
    qemu_printf("Warm-up round %d of 3...\n", warmup_round + 1);
    
    for (int j = 0; j < num_exprs; j++) {
      const benchmark_expr_t *cur_expr = &expressions[j];
      
      // Do more iterations for thorough warm-up
      for (int i = 0; i < 50; i++) {
        // Evaluate the Rust expression
        struct EvalResult result = exp_rs_context_eval(cur_expr->expression, ctx);
        if (result.status != 0) {
          qemu_printf("Error evaluating expression '%s'\n", cur_expr->expression);
          if (result.error) {
            qemu_printf("Error: %s\n", result.error);
            exp_rs_free_error((char *)result.error);
          }
          exp_rs_context_free(ctx);
          return TEST_FAIL;
        }
        
        // Also warm up the direct C path
        cur_expr->direct_func(param_values[0], param_values[1]);
      }
      
      // Add a short pause between expressions during warm-up
      for (volatile int i = 0; i < 1000; i++) { }
    }
    
    // Add a short pause between warm-up rounds
    for (volatile int i = 0; i < 10000; i++) { }
  }
  
  qemu_printf("Warm-up phase complete. Starting benchmarks...\n");

  // Run benchmarks for each expression
  for (int expr_idx = 0; expr_idx < num_exprs; expr_idx++) {
    const benchmark_expr_t *expr = &expressions[expr_idx];
    qemu_printf("\nBenchmarking expression: %s\n", expr->expression);

    // Alternate benchmarking to prevent systematic biases
    // First, we'll do additional per-expression warm-up for more consistency
    qemu_printf("Additional per-expression warm-up...\n");
    
    // Run several iterations of each to ensure cache is hot
    for (int i = 0; i < 5000; i++) {
      expr->direct_func(param_values[0], param_values[1]);
      exp_rs_context_eval(expr->expression, ctx);
      
      // Every 1000 iterations, check if the timer is working correctly
      if (i % 1000 == 0) {
        check_counter_rollover();
      }
    }
    
    // Brief pause before starting actual benchmarks
    for (volatile int i = 0; i < 5000; i++) { }
    
    qemu_printf("Starting benchmark measurements...\n");

    // Benchmark both implementations multiple times and take the best time
    uint32_t exp_rs_best_time = UINT32_MAX;
    uint32_t direct_best_time = UINT32_MAX;
    volatile real_t exprs_sum = 0.0;
    volatile real_t c_sum = 0.0;

    // We no longer need to manually check the timer as this is handled by the
    // timer interface

    // Do a single run for each (to keep within timeout)
    for (int run = 0; run < 1; run++) {
      // Minimal clearing to avoid skewing timings
      for (volatile int i = 0; i < 100; i++) {
      }

      // 1. Benchmark exp_rs
      volatile real_t run_sum = 0.0;

      // Force a compile barrier to make sure things are initialized
      __asm__ volatile("" ::: "memory");

      // Start timing
      benchmark_start();

      // Run the benchmark
      for (int i = 0; i < iterations; i++) {
        // Check for counter rollovers periodically (every 20 iterations)
        // More frequent checks ensure we catch all rollovers and keep the
        // counter running
        if (i % 20 == 0) {
          check_counter_rollover();
        }

        struct EvalResult result = exp_rs_context_eval(expr->expression, ctx);
        if (result.status != 0) {
          qemu_printf("Error evaluating expression '%s'\n", expr->expression);
          if (result.error) {
            qemu_printf("Error: %s\n", result.error);
            exp_rs_free_error((char *)result.error);
          }
          exp_rs_context_free(ctx);
          return TEST_FAIL;
        }
        run_sum += result.value;
      }

      // Stop timing and get elapsed cycles
      uint32_t duration = benchmark_stop();

      // Verify the timing makes sense
      if (duration < 100) {
        qemu_printf("WARNING: Suspiciously low duration (%u ticks) for exp-rs "
                    "benchmark\n",
                    duration);
      }

      // Keep track of best time and sum
      // Use result but don't print each run timing
      if (duration < exp_rs_best_time) {
        exp_rs_best_time = duration;
        exprs_sum = run_sum;
      }

      // Force a small delay between tests
      for (volatile int i = 0; i < 1000; i++) {
      }

      // 2. Benchmark direct C implementation
      run_sum = 0.0;

      // Force a compile barrier to make sure things are initialized
      __asm__ volatile("" ::: "memory");

      // Start timing
      benchmark_start();

      // Run the benchmark
      for (int i = 0; i < iterations; i++) {
        // Check for counter rollovers periodically (every 20 iterations)
        // More frequent checks ensure we catch all rollovers and keep the
        // counter running
        if (i % 20 == 0) {
          check_counter_rollover();
        }

        run_sum += expr->direct_func(param_values[0], param_values[1]);
      }

      // Stop timing and get elapsed cycles
      duration = benchmark_stop();

      // Verify the timing makes sense
      if (duration < 100) {
        qemu_printf("WARNING: Suspiciously low duration (%u ticks) for direct "
                    "C benchmark\n",
                    duration);
      }

      // Keep track of best time and sum
      if (duration < direct_best_time) {
        direct_best_time = duration;
        c_sum = run_sum;
      }
    }

    // Calculate performance ratio (avoid division by zero)
    float ratio = 0.0;
    if (direct_best_time > 0) {
      ratio = (float)exp_rs_best_time / (float)direct_best_time;
    }

    qemu_printf("Performance: ");
    if (ratio > 1.0) {
      qemu_printf("exp-rs %.2fx slower (exp-rs: %u ticks, C: %u ticks)\n",
                  ratio, exp_rs_best_time, direct_best_time);
    } else if (ratio > 0.0) {
      qemu_printf("exp-rs %.2fx faster (exp-rs: %u ticks, C: %u ticks)\n",
                  1.0 / ratio, exp_rs_best_time, direct_best_time);
    } else {
      qemu_printf("Timing data unreliable\n");
    }

    // Verify results match
    struct EvalResult single_result =
        exp_rs_context_eval(expr->expression, ctx);
    real_t direct_result = expr->direct_func(param_values[0], param_values[1]);

    if (!approx_eq(single_result.value, direct_result, TEST_PRECISION)) {
      qemu_printf("FAIL: Result mismatch for '%s'\n", expr->expression);
      qemu_printf("  exp-rs = " FORMAT_SPEC ", direct = " FORMAT_SPEC "\n",
                  single_result.value, direct_result);
      exp_rs_context_free(ctx);
      return TEST_FAIL;
    }
  }

  // Clean up
  exp_rs_context_free(ctx);

  qemu_print("\nexp-rs benchmark completed successfully\n");
  return TEST_PASS;
}

// Test case definition
static const test_case_t tests[] = {
    {"exp_rs_benchmark", benchmark_exp_rs_vs_direct},
};

int main(void) {
  int failed = run_tests(tests, sizeof(tests) / sizeof(tests[0]));
  qemu_exit(failed ? EXIT_FAILURE : EXIT_SUCCESS);
  return failed ? 1 : 0;
}
