#ifndef REGISTER_TEST_FUNCTIONS_H
#define REGISTER_TEST_FUNCTIONS_H

#include "exp_rs.h"
#include <math.h>

// Function prototypes for registering math functions
struct ExprContext* create_test_context(void);
void register_test_math_functions(struct ExprContext* ctx);

// Math function wrappers - use Real type from exp_rs.h
#if defined(DEF_USE_F32) || (defined(USE_F32) && !defined(USE_F64))
    #define SIN_FUNC sinf
    #define COS_FUNC cosf
    #define TAN_FUNC tanf
    #define ASIN_FUNC asinf
    #define ACOS_FUNC acosf
    #define ATAN_FUNC atanf
    #define ATAN2_FUNC atan2f
    #define SINH_FUNC sinhf
    #define COSH_FUNC coshf
    #define TANH_FUNC tanhf
    #define EXP_FUNC expf
    #define LOG_FUNC logf
    #define LOG10_FUNC log10f
    #define LOG2_FUNC log2f
    #define SQRT_FUNC sqrtf
    #define POW_FUNC powf
    #define FABS_FUNC fabsf
    #define FLOOR_FUNC floorf
    #define CEIL_FUNC ceilf
    #define ROUND_FUNC roundf
    #define FMOD_FUNC fmodf
#else
    #define SIN_FUNC sin
    #define COS_FUNC cos
    #define TAN_FUNC tan
    #define ASIN_FUNC asin
    #define ACOS_FUNC acos
    #define ATAN_FUNC atan
    #define ATAN2_FUNC atan2
    #define SINH_FUNC sinh
    #define COSH_FUNC cosh
    #define TANH_FUNC tanh
    #define EXP_FUNC exp
    #define LOG_FUNC log
    #define LOG10_FUNC log10
    #define LOG2_FUNC log2
    #define SQRT_FUNC sqrt
    #define POW_FUNC pow
    #define FABS_FUNC fabs
    #define FLOOR_FUNC floor
    #define CEIL_FUNC ceil
    #define ROUND_FUNC round
    #define FMOD_FUNC fmod
#endif

#endif // REGISTER_TEST_FUNCTIONS_H
