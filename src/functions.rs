//! Built-in mathematical functions for expression evaluation.
//!
//! This module provides the implementation of all built-in functions that can be used
//! in expressions. These include common mathematical operations like trigonometric functions,
//! logarithms, exponentials, and more. The functions handle special cases like division
//! by zero and out-of-range inputs gracefully by returning appropriate values like NaN
//! or infinity.
//!
//! All functions use the `libm` crate for their implementations, which ensures
//! compatibility with no_std environments. Depending on the selected floating-point
//! precision (f32 or f64, controlled by the "f32" feature), different versions of the
//! math functions are used.

#[cfg(all(feature = "libm", feature = "f32"))]
use libm::{
    acosf as libm_acos, asinf as libm_asin, atan2f as libm_atan2, atanf as libm_atan,
    ceilf as libm_ceil, cosf as libm_cos, coshf as libm_cosh, expf as libm_exp,
    floorf as libm_floor, log10f as libm_log10, logf as libm_ln, powf as libm_pow,
    sinf as libm_sin, sinhf as libm_sinh, sqrtf as libm_sqrt, tanf as libm_tan, tanhf as libm_tanh,
};

#[cfg(all(feature = "libm", not(feature = "f32")))]
use libm::{
    acos as libm_acos, asin as libm_asin, atan as libm_atan, atan2 as libm_atan2,
    ceil as libm_ceil, cos as libm_cos, cosh as libm_cosh, exp as libm_exp, floor as libm_floor,
    log as libm_ln, log10 as libm_log10, pow as libm_pow, sin as libm_sin, sinh as libm_sinh,
    sqrt as libm_sqrt, tan as libm_tan, tanh as libm_tanh,
};

use crate::Real;

// When libm feature is not enabled, provide our own implementations
#[cfg(all(not(feature = "libm"), test))]
mod internal_math {
    use crate::Real;
    
    // When not using libm, provide implementations only for tests
    // In real embedded environments, these functions would need to be 
    // registered explicitly by the user
    
    // Trigonometric functions
    pub fn libm_sin(x: Real) -> Real { x.sin() }
    pub fn libm_cos(x: Real) -> Real { x.cos() }
    pub fn libm_tan(x: Real) -> Real { x.tan() }
    pub fn libm_asin(x: Real) -> Real { x.asin() }
    pub fn libm_acos(x: Real) -> Real { x.acos() }
    pub fn libm_atan(x: Real) -> Real { x.atan() }
    pub fn libm_atan2(y: Real, x: Real) -> Real { y.atan2(x) }
    
    // Hyperbolic functions
    pub fn libm_sinh(x: Real) -> Real { x.sinh() }
    pub fn libm_cosh(x: Real) -> Real { x.cosh() }
    pub fn libm_tanh(x: Real) -> Real { x.tanh() }
    
    // Exponential and logarithmic functions
    pub fn libm_exp(x: Real) -> Real { x.exp() }
    pub fn libm_ln(x: Real) -> Real { x.ln() }
    pub fn libm_log10(x: Real) -> Real { x.log10() }
    
    // Power and root functions
    pub fn libm_pow(x: Real, y: Real) -> Real { x.powf(y) }
    pub fn libm_sqrt(x: Real) -> Real { x.sqrt() }
    
    // Rounding functions
    pub fn libm_ceil(x: Real) -> Real { x.ceil() }
    pub fn libm_floor(x: Real) -> Real { x.floor() }
}

// Import our math functions when libm is disabled
#[cfg(all(not(feature = "libm"), test))]
use internal_math::*;

/// Dummy function that panics when called.
///
/// This function is used internally as a placeholder for uninitialized functions.
/// It should never be called in normal operation.
pub fn dummy(_: Real, _: Real) -> Real {
    panic!("called dummy!")
}

/// Returns the maximum of two values.
///
/// # Parameters
///
/// * `a` - First value
/// * `b` - Second value
///
/// # Returns
///
/// The larger of `a` and `b`.
pub fn max(a: Real, b: Real) -> Real {
    if a > b {
        a
    } else {
        b
    }
}

/// Adds two values.
///
/// # Parameters
///
/// * `a` - First value
/// * `b` - Second value
///
/// # Returns
///
/// The sum of `a` and `b`.
pub fn add(a: Real, b: Real) -> Real {
    a + b
}

/// Returns the minimum of two values.
///
/// # Parameters
///
/// * `a` - First value
/// * `b` - Second value
///
/// # Returns
///
/// The smaller of `a` and `b`.
pub fn min(a: Real, b: Real) -> Real {
    if a < b {
        a
    } else {
        b
    }
}

/// Subtracts the second value from the first.
///
/// # Parameters
///
/// * `a` - Value to subtract from
/// * `b` - Value to subtract
///
/// # Returns
///
/// The difference `a - b`.
pub fn sub(a: Real, b: Real) -> Real {
    a - b
}

/// Multiplies two values.
///
/// # Parameters
///
/// * `a` - First value
/// * `b` - Second value
///
/// # Returns
///
/// The product of `a` and `b`.
pub fn mul(a: Real, b: Real) -> Real {
    a * b
}

/// Divides the first value by the second.
///
/// This function handles division by zero gracefully by returning:
/// - NaN for 0/0
/// - Positive infinity for positive/0
/// - Negative infinity for negative/0
///
/// # Parameters
///
/// * `a` - Numerator
/// * `b` - Denominator
///
/// # Returns
///
/// The quotient `a / b`, or appropriate value for division by zero.
pub fn div(a: Real, b: Real) -> Real {
    if b == 0.0 {
        if a == 0.0 {
            #[cfg(feature = "f32")]
            return f32::NAN; // 0/0 is NaN
            #[cfg(not(feature = "f32"))]
            return f64::NAN; // 0/0 is NaN
        } else if a > 0.0 {
            #[cfg(feature = "f32")]
            return f32::INFINITY; // Positive number divided by zero is positive infinity
            #[cfg(not(feature = "f32"))]
            return f64::INFINITY; // Positive number divided by zero is positive infinity
        } else {
            #[cfg(feature = "f32")]
            return f32::NEG_INFINITY; // Negative number divided by zero is negative infinity
            #[cfg(not(feature = "f32"))]
            return f64::NEG_INFINITY; // Negative number divided by zero is negative infinity
        }
    } else {
        a / b
    }
}
pub fn fmod(a: Real, b: Real) -> Real {
    a % b
}
pub fn neg(a: Real, _: Real) -> Real {
    -a
}
pub fn comma(_: Real, b: Real) -> Real {
    b
}
pub fn abs(a: Real, _: Real) -> Real {
    a.abs()
}
#[cfg(feature = "libm")]
pub fn acos(a: Real, _: Real) -> Real {
    if !(-1.0..=1.0).contains(&a) {
        #[cfg(feature = "f32")]
        return f32::NAN; // acos is only defined for inputs between -1 and 1
        #[cfg(not(feature = "f32"))]
        return f64::NAN; // acos is only defined for inputs between -1 and 1
    } else {
        libm_acos(a)
    }
}

#[cfg(all(not(feature = "libm"), test))]
pub fn acos(a: Real, _: Real) -> Real {
    if !(-1.0..=1.0).contains(&a) {
        #[cfg(feature = "f32")]
        return f32::NAN;
        #[cfg(not(feature = "f32"))]
        return f64::NAN;
    } else {
        libm_acos(a)
    }
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn acos(_: Real, _: Real) -> Real {
    // In no_std without libm, this function would need to be registered by the user
    panic!("acos requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn asin(a: Real, _: Real) -> Real {
    if !(-1.0..=1.0).contains(&a) {
        #[cfg(feature = "f32")]
        return f32::NAN; // asin is only defined for inputs between -1 and 1
        #[cfg(not(feature = "f32"))]
        return f64::NAN; // asin is only defined for inputs between -1 and 1
    } else {
        libm_asin(a)
    }
}

#[cfg(all(not(feature = "libm"), test))]
pub fn asin(a: Real, _: Real) -> Real {
    if !(-1.0..=1.0).contains(&a) {
        #[cfg(feature = "f32")]
        return f32::NAN;
        #[cfg(not(feature = "f32"))]
        return f64::NAN;
    } else {
        libm_asin(a)
    }
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn asin(_: Real, _: Real) -> Real {
    panic!("asin requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn atan(a: Real, _: Real) -> Real {
    libm_atan(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn atan(a: Real, _: Real) -> Real {
    libm_atan(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn atan(_: Real, _: Real) -> Real {
    panic!("atan requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn atan2(a: Real, b: Real) -> Real {
    // atan2 takes y,x order (not x,y)
    #[cfg(test)]
    println!("atan2 called with a={}, b={}", a, b);

    let result = libm_atan2(a, b); // Don't swap the arguments

    #[cfg(test)]
    println!("atan2 result: {}", result);

    result
}

#[cfg(all(not(feature = "libm"), test))]
pub fn atan2(a: Real, b: Real) -> Real {
    libm_atan2(a, b)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn atan2(_: Real, _: Real) -> Real {
    panic!("atan2 requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn ceil(a: Real, _: Real) -> Real {
    libm_ceil(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn ceil(a: Real, _: Real) -> Real {
    libm_ceil(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn ceil(a: Real, _: Real) -> Real {
    // Simple implementation that works without libm
    let i = a as i64 as Real;
    if a > 0.0 && a > i {
        i + 1.0
    } else {
        i
    }
}

#[cfg(feature = "libm")]
pub fn cos(a: Real, _: Real) -> Real {
    libm_cos(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn cos(a: Real, _: Real) -> Real {
    libm_cos(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn cos(_: Real, _: Real) -> Real {
    panic!("cos requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn cosh(a: Real, _: Real) -> Real {
    libm_cosh(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn cosh(a: Real, _: Real) -> Real {
    libm_cosh(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn cosh(_: Real, _: Real) -> Real {
    panic!("cosh requires libm or custom implementation")
}

pub fn e(_: Real, _: Real) -> Real {
    crate::constants::E
}

#[cfg(feature = "libm")]
pub fn exp(a: Real, _: Real) -> Real {
    libm_exp(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn exp(a: Real, _: Real) -> Real {
    libm_exp(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn exp(_: Real, _: Real) -> Real {
    panic!("exp requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn floor(a: Real, _: Real) -> Real {
    libm_floor(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn floor(a: Real, _: Real) -> Real {
    libm_floor(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn floor(a: Real, _: Real) -> Real {
    // Simple implementation that works without libm
    let i = a as i64 as Real;
    if a < 0.0 && a < i {
        i - 1.0
    } else {
        i
    }
}

#[cfg(feature = "libm")]
pub fn ln(a: Real, _: Real) -> Real {
    if a <= 0.0 {
        #[cfg(feature = "f32")]
        return f32::NAN; // Natural log of zero or negative number is undefined
        #[cfg(not(feature = "f32"))]
        return f64::NAN; // Natural log of zero or negative number is undefined
    } else {
        libm_ln(a)
    }
}

#[cfg(all(not(feature = "libm"), test))]
pub fn ln(a: Real, _: Real) -> Real {
    if a <= 0.0 {
        #[cfg(feature = "f32")]
        return f32::NAN;
        #[cfg(not(feature = "f32"))]
        return f64::NAN;
    } else {
        libm_ln(a)
    }
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn ln(_: Real, _: Real) -> Real {
    panic!("ln requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn log(a: Real, _: Real) -> Real {
    if a <= 0.0 {
        #[cfg(feature = "f32")]
        return f32::NAN; // Log of zero or negative number is undefined
        #[cfg(not(feature = "f32"))]
        return f64::NAN; // Log of zero or negative number is undefined
    } else {
        libm_log10(a)
    }
}

#[cfg(all(not(feature = "libm"), test))]
pub fn log(a: Real, _: Real) -> Real {
    if a <= 0.0 {
        #[cfg(feature = "f32")]
        return f32::NAN;
        #[cfg(not(feature = "f32"))]
        return f64::NAN;
    } else {
        libm_log10(a)
    }
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn log(_: Real, _: Real) -> Real {
    panic!("log requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn log10(a: Real, _: Real) -> Real {
    if a <= 0.0 {
        #[cfg(feature = "f32")]
        return f32::NAN; // Log10 of zero or negative number is undefined
        #[cfg(not(feature = "f32"))]
        return f64::NAN; // Log10 of zero or negative number is undefined
    } else {
        libm_log10(a)
    }
}

#[cfg(all(not(feature = "libm"), test))]
pub fn log10(a: Real, _: Real) -> Real {
    if a <= 0.0 {
        #[cfg(feature = "f32")]
        return f32::NAN;
        #[cfg(not(feature = "f32"))]
        return f64::NAN;
    } else {
        libm_log10(a)
    }
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn log10(_: Real, _: Real) -> Real {
    panic!("log10 requires libm or custom implementation")
}

pub fn pi(_: Real, _: Real) -> Real {
    crate::constants::PI
}

/// Raises a value to a power.
///
/// This function computes `a` raised to the power of `b` (a^b).
/// It handles various special cases and edge conditions:
///
/// - 0^0 = 1 (by mathematical convention)
/// - Negative base with non-integer exponent returns NaN
/// - Very large exponents that would cause overflow return infinity
/// - Very small values that would cause underflow return 0
///
/// # Parameters
///
/// * `a` - Base value
/// * `b` - Exponent
///
/// # Returns
///
/// The value of `a` raised to the power of `b`.
#[cfg(feature = "libm")]
pub fn pow(a: Real, b: Real) -> Real {
    #[cfg(test)]
    println!("pow function called with a={}, b={}", a, b);

    // Handle special cases
    if a == 0.0 && b == 0.0 {
        return 1.0; // 0^0 = 1 by convention
    }

    if a < 0.0 && b != libm_floor(b) {
        // Negative base with non-integer exponent is not a real number
        #[cfg(feature = "f32")]
        return f32::NAN;
        #[cfg(not(feature = "f32"))]
        return f64::NAN;
    }

    // Check for potential overflow
    #[cfg(feature = "f32")]
    if a.abs() > 1.0 && b > 88.0 {
        return if a > 0.0 {
            f32::INFINITY
        } else {
            f32::NEG_INFINITY
        };
    }

    #[cfg(not(feature = "f32"))]
    if a.abs() > 1.0 && b > 709.0 {
        return if a > 0.0 {
            f64::INFINITY
        } else {
            f64::NEG_INFINITY
        };
    }

    #[cfg(feature = "f32")]
    if a.abs() < 1.0 && b < -88.0 {
        return 0.0; // Underflow to zero
    }

    #[cfg(not(feature = "f32"))]
    if a.abs() < 1.0 && b < -709.0 {
        return 0.0; // Underflow to zero
    }

    libm_pow(a, b)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn pow(a: Real, b: Real) -> Real {
    // Simplified version for tests
    if a == 0.0 && b == 0.0 {
        return 1.0;
    }
    
    libm_pow(a, b)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn pow(a: Real, b: Real) -> Real {
    // Basic implementation for non-libm, non-test builds
    // Handles only a few special cases
    if b == 0.0 { return 1.0; }
    if b == 1.0 { return a; }
    if b == 2.0 { return a * a; }
    if b == 0.5 && a >= 0.0 { return sqrt(a, 0.0); }
    if b == -1.0 { return 1.0 / a; }
    
    panic!("pow requires libm or custom implementation for general cases")
}

#[cfg(feature = "libm")]
pub fn sin(a: Real, _: Real) -> Real {
    libm_sin(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn sin(a: Real, _: Real) -> Real {
    libm_sin(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn sin(_: Real, _: Real) -> Real {
    panic!("sin requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn sinh(a: Real, _: Real) -> Real {
    libm_sinh(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn sinh(a: Real, _: Real) -> Real {
    libm_sinh(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn sinh(_: Real, _: Real) -> Real {
    panic!("sinh requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn sqrt(a: Real, _: Real) -> Real {
    if a < 0.0 {
        #[cfg(feature = "f32")]
        return f32::NAN; // Square root of negative number is NaN
        #[cfg(not(feature = "f32"))]
        return f64::NAN; // Square root of negative number is NaN
    } else {
        libm_sqrt(a)
    }
}

#[cfg(all(not(feature = "libm"), test))]
pub fn sqrt(a: Real, _: Real) -> Real {
    if a < 0.0 {
        #[cfg(feature = "f32")]
        return f32::NAN;
        #[cfg(not(feature = "f32"))]
        return f64::NAN;
    } else {
        libm_sqrt(a)
    }
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn sqrt(a: Real, _: Real) -> Real {
    if a < 0.0 {
        #[cfg(feature = "f32")]
        return f32::NAN;
        #[cfg(not(feature = "f32"))]
        return f64::NAN;
    }
    
    // Very simplified Newton-Raphson for sqrt approximation
    if a == 0.0 { return 0.0; }
    let mut x = a;
    let mut y = 1.0;
    
    // Just a couple of iterations
    for _ in 0..3 {
        y = 0.5 * (y + x / y);
    }
    
    y
}

#[cfg(feature = "libm")]
pub fn tan(a: Real, _: Real) -> Real {
    libm_tan(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn tan(a: Real, _: Real) -> Real {
    libm_tan(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn tan(_: Real, _: Real) -> Real {
    panic!("tan requires libm or custom implementation")
}

#[cfg(feature = "libm")]
pub fn tanh(a: Real, _: Real) -> Real {
    libm_tanh(a)
}

#[cfg(all(not(feature = "libm"), test))]
pub fn tanh(a: Real, _: Real) -> Real {
    libm_tanh(a)
}

#[cfg(all(not(feature = "libm"), not(test)))]
pub fn tanh(_: Real, _: Real) -> Real {
    panic!("tanh requires libm or custom implementation")
}

pub fn sign(a: Real, _: Real) -> Real {
    if a > 0.0 {
        1.0
    } else if a < 0.0 {
        -1.0
    } else {
        0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic(expected = "called dummy!")]
    fn test_dummy_panics() {
        dummy(1.0, 2.0);
    }

    #[test]
    fn test_sub() {
        assert_eq!(sub(5.0, 3.0), 2.0);
    }

    #[test]
    fn test_mul() {
        assert_eq!(mul(2.0, 3.0), 6.0);
    }

    #[test]
    fn test_div() {
        assert_eq!(div(6.0, 3.0), 2.0);
    }

    #[test]
    fn test_fmod() {
        assert_eq!(fmod(7.0, 4.0), 3.0);
    }

    #[test]
    fn test_neg() {
        assert_eq!(neg(5.0, 0.0), -5.0);
    }

    #[test]
    fn test_comma() {
        assert_eq!(comma(1.0, 2.0), 2.0);
    }

    #[test]
    fn test_abs() {
        assert_eq!(abs(-5.0, 0.0), 5.0);
    }

    #[test]
    fn test_acos() {
        assert!((acos(1.0, 0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_asin() {
        assert!((asin(0.0, 0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_atan() {
        assert!((atan(0.0, 0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_atan2() {
        // Test basic cases with direct function calls
        #[cfg(feature = "f32")]
        assert!((atan2(1.0, 1.0) - core::f32::consts::FRAC_PI_4).abs() < 1e-6);
        #[cfg(not(feature = "f32"))]
        assert!((atan2(1.0, 1.0) - core::f64::consts::FRAC_PI_4).abs() < 1e-10);

        // Test more cases to verify the function works correctly
        // atan2(y, x) where:

        // Quadrant 1: y > 0, x > 0
        assert!(
            (atan2(1.0, 2.0) - 0.4636476090008061).abs() < 1e-10,
            "atan2(1.0, 2.0) should be approximately 0.4636"
        );

        // Quadrant 2: y > 0, x < 0
        #[cfg(feature = "f32")]
        assert!(
            (atan2(1.0, -1.0) - (3.0 * core::f32::consts::FRAC_PI_4)).abs() < 1e-6,
            "atan2(1.0, -1.0) should be approximately 3π/4"
        );
        #[cfg(not(feature = "f32"))]
        assert!(
            (atan2(1.0, -1.0) - (3.0 * core::f64::consts::FRAC_PI_4)).abs() < 1e-10,
            "atan2(1.0, -1.0) should be approximately 3π/4"
        );

        // Quadrant 3: y < 0, x < 0
        #[cfg(feature = "f32")]
        assert!(
            (atan2(-1.0, -1.0) + (3.0 * core::f32::consts::FRAC_PI_4)).abs() < 1e-6,
            "atan2(-1.0, -1.0) should be approximately -3π/4"
        );
        #[cfg(not(feature = "f32"))]
        assert!(
            (atan2(-1.0, -1.0) + (3.0 * core::f64::consts::FRAC_PI_4)).abs() < 1e-10,
            "atan2(-1.0, -1.0) should be approximately -3π/4"
        );

        // Quadrant 4: y < 0, x > 0
        #[cfg(feature = "f32")]
        assert!(
            (atan2(-1.0, 1.0) + core::f32::consts::FRAC_PI_4).abs() < 1e-6,
            "atan2(-1.0, 1.0) should be approximately -π/4"
        );
        #[cfg(not(feature = "f32"))]
        assert!(
            (atan2(-1.0, 1.0) + core::f64::consts::FRAC_PI_4).abs() < 1e-10,
            "atan2(-1.0, 1.0) should be approximately -π/4"
        );

        // Special cases
        assert!(
            (atan2(0.0, 1.0) - 0.0).abs() < 1e-10,
            "atan2(0.0, 1.0) should be 0"
        );

        #[cfg(feature = "f32")]
        assert!(
            (atan2(1.0, 0.0) - core::f32::consts::FRAC_PI_2).abs() < 1e-6,
            "atan2(1.0, 0.0) should be π/2"
        );
        #[cfg(not(feature = "f32"))]
        assert!(
            (atan2(1.0, 0.0) - core::f64::consts::FRAC_PI_2).abs() < 1e-10,
            "atan2(1.0, 0.0) should be π/2"
        );

        #[cfg(feature = "f32")]
        assert!(
            (atan2(0.0, -1.0) - core::f32::consts::PI).abs() < 1e-6,
            "atan2(0.0, -1.0) should be π"
        );
        #[cfg(not(feature = "f32"))]
        assert!(
            (atan2(0.0, -1.0) - core::f64::consts::PI).abs() < 1e-10,
            "atan2(0.0, -1.0) should be π"
        );

        #[cfg(feature = "f32")]
        assert!(
            (atan2(-1.0, 0.0) + core::f32::consts::FRAC_PI_2).abs() < 1e-6,
            "atan2(-1.0, 0.0) should be -π/2"
        );
        #[cfg(not(feature = "f32"))]
        assert!(
            (atan2(-1.0, 0.0) + core::f64::consts::FRAC_PI_2).abs() < 1e-10,
            "atan2(-1.0, 0.0) should be -π/2"
        );

        // Print debug information
        println!("atan2(1.0, 2.0) = {}", atan2(1.0, 2.0));
        println!("atan2(2.0, 1.0) = {}", atan2(2.0, 1.0));
    }

    #[test]
    fn test_ceil() {
        assert_eq!(ceil(2.3, 0.0), 3.0);
    }

    #[test]
    fn test_cos() {
        assert!((cos(0.0, 0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_cosh() {
        assert!((cosh(0.0, 0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_e() {
        #[cfg(feature = "f32")]
        assert!((e(0.0, 0.0) - core::f32::consts::E).abs() < 1e-6);
        #[cfg(not(feature = "f32"))]
        assert!((e(0.0, 0.0) - core::f64::consts::E).abs() < 1e-10);
    }

    #[test]
    fn test_exp() {
        #[cfg(feature = "f32")]
        assert!((exp(1.0, 0.0) - core::f32::consts::E).abs() < 1e-6);
        #[cfg(not(feature = "f32"))]
        assert!((exp(1.0, 0.0) - core::f64::consts::E).abs() < 1e-10);
    }

    #[test]
    fn test_floor() {
        assert_eq!(floor(2.7, 0.0), 2.0);
    }

    #[test]
    fn test_ln() {
        #[cfg(feature = "f32")]
        assert!((ln(core::f32::consts::E, 0.0) - 1.0).abs() < 1e-6);
        #[cfg(not(feature = "f32"))]
        assert!((ln(core::f64::consts::E, 0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_log() {
        assert!((log(1000.0, 0.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_log10() {
        assert!((log10(1000.0, 0.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_pi() {
        #[cfg(feature = "f32")]
        assert!((pi(0.0, 0.0) - core::f32::consts::PI).abs() < 1e-6);
        #[cfg(not(feature = "f32"))]
        assert!((pi(0.0, 0.0) - core::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn test_pow() {
        assert_eq!(pow(2.0, 3.0), 8.0);
    }

    #[test]
    fn test_sin() {
        assert!((sin(0.0, 0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_sinh() {
        assert!((sinh(0.0, 0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(sqrt(4.0, 0.0), 2.0);
    }

    #[test]
    fn test_tan() {
        assert!((tan(0.0, 0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_tanh() {
        assert!((tanh(0.0, 0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_sign() {
        assert_eq!(sign(5.0, 0.0), 1.0);
        assert_eq!(sign(-3.0, 0.0), -1.0);
        assert_eq!(sign(0.0, 0.0), 0.0);
    }
}
