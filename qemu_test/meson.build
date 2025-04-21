subdir('qemu_harness')

# Get options from the main project or set defaults
enable_f64 = get_option('enable_f64')

# Use the real CMSIS-DSP library from a subproject using the wrap system
cmsis_dsp_proj = subproject('cmsis-dsp', default_options: ['enable_f64=' + enable_f64.to_string()])
cmsis_dsp_dep = cmsis_dsp_proj.get_variable('cmsis_dsp_dep')

common_link_args = [
  '-T', meson.current_source_dir() + '/qemu_harness/mps2_an500.ld',
  '-lc',
  '-lrdimon',
  '-lm',
]
common_test_args = [
  '-M', 'mps2-an500',
  '-cpu', 'cortex-m7',
  '-semihosting',
  '-semihosting-config', 'enable=on,target=native',
  '-nographic',
  '-monitor', 'none',
  '-serial', 'stdio',
]

# C test program for FFI - F32 version
test_ffi_f32_exe = executable(
  'test_ffi_f32',
  ['test_ffi.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: include_directories('.', '../include'),
  dependencies: [qemu_harness_dep, exp_rs_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)

# C test program for FFI - F64 version (built when F64 is enabled)
if enable_f64
  test_ffi_f64_exe = executable(
    'test_ffi_f64',
    ['test_ffi.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )
endif

# C test program for EvalContext - F32 version
test_eval_context_f32_exe = executable(
  'test_eval_context_f32',
  ['eval_context.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: include_directories('.', '../include'),
  dependencies: [qemu_harness_dep, exp_rs_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)

# C test program for EvalContext - F64 version (built when F64 is enabled)
if enable_f64
  test_eval_context_f64_exe = executable(
    'test_eval_context_f64',
    ['eval_context.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )
endif

# C test for benchmark - F32 version
test_benchmark_f32_exe = executable(
  'test_benchmark_f32',
  ['benchmark.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: include_directories('.', '../include'),
  dependencies: [qemu_harness_dep, exp_rs_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)

# C test for benchmark - F64 version (built when F64 is enabled)
if enable_f64
  test_benchmark_f64_exe = executable(
    'test_benchmark_f64',
    ['benchmark.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )
endif

# Consolidated CMSIS-DSP test (F32 mode)
test_cmsis_dsp_f32_exe = executable(
  'test_cmsis_dsp_f32',
  ['cmsis_dsp_test.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: [include_directories('.', '../include')],
  dependencies: [qemu_harness_dep, cmsis_dsp_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)

# Consolidated CMSIS-DSP test (F64 mode - only built when F64 is enabled)
if enable_f64
  test_cmsis_dsp_f64_exe = executable(
    'test_cmsis_dsp_f64',
    ['cmsis_dsp_test.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: [include_directories('.', '../include')],
    dependencies: [qemu_harness_dep, cmsis_dsp_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )
endif

# Add a test target for the FFI test (F32 version)
test(
  'test_ffi_f32',
  find_program('qemu-system-arm'),
  args: common_test_args + ['-kernel', test_ffi_f32_exe.full_path()],
  is_parallel: false,
  timeout: 5,
)

# Add a test target for the FFI test (F64 version, when enabled)
if enable_f64
  test(
    'test_ffi_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_ffi_f64_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Add a test target for the EvalContext test - F32 version
test(
  'test_eval_context_f32',
  find_program('qemu-system-arm'),
  args: common_test_args + ['-kernel', test_eval_context_f32_exe.full_path()],
  is_parallel: false,
  timeout: 5,
)

# Add a test target for the EvalContext test - F64 version (when enabled)
if enable_f64
  test(
    'test_eval_context_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_eval_context_f64_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Add a test target for the Benchmark test - F32 version
test(
  'test_benchmark_f32',
  find_program('qemu-system-arm'),
  args: common_test_args + ['-kernel', test_benchmark_f32_exe.full_path()],
  is_parallel: false,
  timeout: 50, # Longer timeout for benchmark
)

# Add a test target for the Benchmark test - F64 version (when enabled)
if enable_f64
  test(
    'test_benchmark_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_benchmark_f64_exe.full_path()],
    is_parallel: false,
    timeout: 15, # Longer timeout for benchmark
  )
endif

# Add a test target for the CMSIS-DSP F32 test
test(
  'test_cmsis_dsp_f32',
  find_program('qemu-system-arm'),
  args: common_test_args + ['-kernel', test_cmsis_dsp_f32_exe.full_path()],
  is_parallel: false,
  timeout: 15, # Longer timeout for benchmark
)

# Add a test target for the CMSIS-DSP F64 test
if enable_f64
  test(
    'test_cmsis_dsp_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_cmsis_dsp_f64_exe.full_path()],
    is_parallel: false,
    timeout: 15, # Longer timeout for benchmark
  )
endif

# CMSIS-DSP Benchmark (F32 mode)
test_cmsis_dsp_benchmark_f32_exe = executable(
  'test_cmsis_dsp_benchmark_f32',
  ['cmsis_dsp_benchmark.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: [include_directories('.', '../include')],
  dependencies: [qemu_harness_dep, cmsis_dsp_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)

# Add a test target for the CMSIS-DSP benchmark F32 test
test(
  'test_cmsis_dsp_benchmark_f32',
  find_program('qemu-system-arm'),
  args: common_test_args + ['-kernel', test_cmsis_dsp_benchmark_f32_exe.full_path()],
  is_parallel: false,
  timeout: 15, # Longer timeout for benchmark
)

# CMSIS-DSP Benchmark (F64 mode - only built when F64 is enabled)
if enable_f64
  test_cmsis_dsp_benchmark_f64_exe = executable(
    'test_cmsis_dsp_benchmark_f64',
    ['cmsis_dsp_benchmark.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: [include_directories('.', '../include')],
    dependencies: [qemu_harness_dep, cmsis_dsp_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )

  # Add a test target for the CMSIS-DSP benchmark F64 test
  test(
    'test_cmsis_dsp_benchmark_f64',
    find_program('qemu-system-arm'),
    args: common_test_args
    + ['-kernel', test_cmsis_dsp_benchmark_f64_exe.full_path()],
    is_parallel: false,
    timeout: 15, # Longer timeout for benchmark
  )
endif

# exp-rs Integration test - F32 mode
test_exp_rs_integration_f32_exe = executable(
  'test_exp_rs_integration_f32',
  ['exp_rs_integration_test.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: [include_directories('.', '../include')],
  dependencies: [qemu_harness_dep, exp_rs_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)

# Add a test target for the exp-rs integration F32 test
test(
  'test_exp_rs_integration_f32',
  find_program('qemu-system-arm'),
  args: common_test_args + ['-kernel', test_exp_rs_integration_f32_exe.full_path()],
  is_parallel: false,
  timeout: 5,
)

# exp-rs Integration test - F64 mode (only when F64 is enabled)
if enable_f64
  test_exp_rs_integration_f64_exe = executable(
    'test_exp_rs_integration_f64',
    ['exp_rs_integration_test.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: [include_directories('.', '../include')],
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )

  # Add a test target for the exp-rs integration F64 test
  test(
    'test_exp_rs_integration_f64',
    find_program('qemu-system-arm'),
    args: common_test_args
    + ['-kernel', test_exp_rs_integration_f64_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# exp-rs CMSIS Integration Benchmark (F32 mode)
test_exp_rs_cmsis_benchmark_f32_exe = executable(
  'test_exp_rs_cmsis_benchmark_f32',
  ['exp_rs_cmsis_benchmark.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: [include_directories('.', '../include')],
  dependencies: [qemu_harness_dep, exp_rs_dep, cmsis_dsp_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)

# Add a test target for the exp-rs CMSIS Integration Benchmark F32
test(
  'test_exp_rs_cmsis_benchmark_f32',
  find_program('qemu-system-arm'),
  args: common_test_args
  + ['-kernel', test_exp_rs_cmsis_benchmark_f32_exe.full_path()],
  is_parallel: false,
  timeout: 60, # Longer timeout for benchmark
)

# exp-rs CMSIS Integration Benchmark (F64 mode - only built when F64 is enabled)
if enable_f64
  test_exp_rs_cmsis_benchmark_f64_exe = executable(
    'test_exp_rs_cmsis_benchmark_f64',
    ['exp_rs_cmsis_benchmark.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: [include_directories('.', '../include')],
    dependencies: [qemu_harness_dep, exp_rs_dep, cmsis_dsp_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )

  # Add a test target for the exp-rs CMSIS Integration Benchmark F64
  test(
    'test_exp_rs_cmsis_benchmark_f64',
    find_program('qemu-system-arm'),
    args: common_test_args
    + ['-kernel', test_exp_rs_cmsis_benchmark_f64_exe.full_path()],
    is_parallel: false,
    timeout: 60, # Longer timeout for benchmark
  )
endif

# DWT timer test (F32 mode)
dwt_timer_test_exe = executable(
  'dwt_timer_test',
  ['dwt_timer_test.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: include_directories('.'),
  dependencies: [qemu_harness_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)

# Add a test target for the DWT timer test
test(
  'dwt_timer_test',
  find_program('qemu-system-arm'),
  args: common_test_args + ['-kernel', dwt_timer_test_exe.full_path()],
  is_parallel: false,
  timeout: 5,
)
