project('cmsis-dsp', 'c')

# Get CMSIS-Core dependency for headers like cmsis_compiler.h
cmsis_core_proj = subproject('cmsis-core')
cmsis_core_dep = cmsis_core_proj.get_variable('cmsis_core_dep')

# Link the CMSIS-Core Include directory into the CMSIS-DSP Include directory
cmsis_link = run_command('ln', '-sf', '../cmsis-core/CMSIS/Core/Include', meson.current_source_dir() + '/Include/cmsis', check: true)

# Get options
enable_f64 = get_option('enable_f64')
enable_mvef = get_option('enable_mvef')
enable_fp16 = get_option('enable_fp16')

# Include directories for CMSIS-DSP
cmsis_dsp_inc = include_directories(
  'Include',
  'PrivateInclude',
  '.',
  '../cmsis-core/CMSIS/Core/Include'
)

# Compiler flags for CMSIS-DSP
cmsis_dsp_args = [
  '-DARM_MATH_CM7',         # For Cortex-M7
  '-D__FPU_PRESENT=1',      # Enable FPU
  '-DARM_MATH_AUTOVECTORIZE' # Enable auto-vectorization
]

# Add MVE support if enabled
if enable_mvef
  cmsis_dsp_args += '-DARM_MATH_MVEF'  # Enable MVE (64-bit vector extensions)
endif

# Add F16 support if enabled
if enable_fp16
  cmsis_dsp_args += '-DFLOAT16_SUPPORTED'  # Enable half-precision float support
endif

# Add F64 support if enabled
if enable_f64
  cmsis_dsp_args += '-DARM_MATH_DOUBLE'  # Enable double-precision support
endif

# Define CMSIS-DSP source files needed for our test
cmsis_dsp_sources = [
  # Basic Math Functions
  'Source/BasicMathFunctions/BasicMathFunctions.c',
  
  # Fast Math Functions (trig functions, etc.)
  'Source/FastMathFunctions/FastMathFunctions.c',
  'Source/FastMathFunctions/arm_sin_f32.c',
  'Source/FastMathFunctions/arm_cos_f32.c',
  'Source/FastMathFunctions/arm_sqrt_q15.c',
  'Source/FastMathFunctions/arm_sqrt_q31.c',
  
  # Support Functions
  'Source/SupportFunctions/SupportFunctions.c',
  
  # Common Tables
  'Source/CommonTables/CommonTables.c'
]

# Add F16 sources if enabled
if enable_fp16
  cmsis_dsp_sources += [
    'Source/BasicMathFunctions/BasicMathFunctionsF16.c',
    'Source/FastMathFunctions/FastMathFunctionsF16.c',
    'Source/SupportFunctions/SupportFunctionsF16.c',
    'Source/CommonTables/CommonTablesF16.c'
  ]
endif

# Add F64 sources if enabled
if enable_f64
  cmsis_dsp_sources += [
    'Source/FastMathFunctions/arm_vlog_f64.c',
    'Source/FastMathFunctions/arm_vexp_f64.c'
    # Note: sin_f64 and cos_f64 are not available in CMSIS-DSP
    # We'll implement custom versions in our wrapper
  ]
endif

# Create a static library with the CMSIS-DSP sources
cmsis_dsp_lib = static_library('cmsis-dsp', cmsis_dsp_sources,
  c_args: cmsis_dsp_args,
  include_directories: cmsis_dsp_inc,
  dependencies: [cmsis_core_dep]
)

# Create a dependency object that other targets can depend on
cmsis_dsp_dep = declare_dependency(
  include_directories: cmsis_dsp_inc,
  link_with: cmsis_dsp_lib,
  compile_args: cmsis_dsp_args,
  dependencies: [cmsis_core_dep]
)