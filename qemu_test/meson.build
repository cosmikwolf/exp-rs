subdir('qemu_harness')

# Get options from the main project or set defaults
use_f32 = get_option('use_f32')
enable_f64 = not use_f32
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

if use_f32
# C test program for FFI - F32 version
test_ffi_f32_exe = executable(
  'test_ffi_f32',
  ['test_ffi.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: include_directories('.', '../include'),
  dependencies: [qemu_harness_dep, exp_rs_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)
# C test program for EvalContext - F32 version
test_eval_context_f32_exe = executable(
  'test_eval_context_f32',
  ['eval_context.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: include_directories('.', '../include'),
  dependencies: [qemu_harness_dep, exp_rs_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
)
else
# C test program for FFI - F64 version (built when F64 is enabled)
  test_ffi_f64_exe = executable(
    'test_ffi_f64',
    ['test_ffi.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )
endif


# C test program for EvalContext - F64 version (built when F64 is enabled)
if not use_f32
  test_eval_context_f64_exe = executable(
    'test_eval_context_f64',
    ['eval_context.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )
endif

# C test program for native function registration - F32 version
if use_f32
  test_native_function_f32_exe = executable(
  'test_native_function_f32',
  ['test_native_function.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: include_directories('.', '../include'),
  dependencies: [qemu_harness_dep, exp_rs_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
  )
endif

# C test program for native function registration - F64 version (built when F64 is enabled)
if not use_f32
  test_native_function_f64_exe = executable(
    'test_native_function_f64',
    ['test_native_function.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )
endif

# C test for benchmark - F32 version
if use_f32
  test_benchmark_f32_exe = executable(
  'test_benchmark_f32',
  ['benchmark.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: include_directories('.', '../include'),
  dependencies: [qemu_harness_dep, exp_rs_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
  )
endif

# C test for benchmark - F64 version (built when F64 is enabled)
if not use_f32
  test_benchmark_f64_exe = executable(
    'test_benchmark_f64',
    ['benchmark.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )
endif

# Consolidated CMSIS-DSP test (F32 mode)
if use_f32
  test_cmsis_dsp_f32_exe = executable(
  'test_cmsis_dsp_f32',
  ['cmsis_dsp_test.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: [include_directories('.', '../include')],
  dependencies: [qemu_harness_dep, cmsis_dsp_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
  )
endif

# Consolidated CMSIS-DSP test (F64 mode - only built when F64 is enabled)
if not use_f32
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
if use_f32
  test(
    'test_ffi_f32',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_ffi_f32_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Add a test target for the FFI test (F64 version, when enabled)
if not use_f32
  test(
    'test_ffi_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_ffi_f64_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Add a test target for the EvalContext test - F32 version
if use_f32
  test(
    'test_eval_context_f32',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_eval_context_f32_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Add a test target for the EvalContext test - F64 version (when enabled)
if not use_f32
  test(
    'test_eval_context_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_eval_context_f64_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Add a test target for the native function registration test - F32 version
if use_f32
  test(
    'test_native_function_f32',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_native_function_f32_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Add a test target for the native function registration test - F64 version (when enabled)
if not use_f32
  test(
    'test_native_function_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_native_function_f64_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Add a test target for the Benchmark test - F32 version
if use_f32
  test(
    'test_benchmark_f32',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_benchmark_f32_exe.full_path()],
    is_parallel: false,
    timeout: 50, # Longer timeout for benchmark
  )
endif

# Add a test target for the Benchmark test - F64 version (when enabled)
if not use_f32
  test(
    'test_benchmark_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_benchmark_f64_exe.full_path()],
    is_parallel: false,
    timeout: 15, # Longer timeout for benchmark
  )
endif

# Add a test target for the CMSIS-DSP F32 test
if use_f32
  test(
    'test_cmsis_dsp_f32',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_cmsis_dsp_f32_exe.full_path()],
    is_parallel: false,
    timeout: 15, # Longer timeout for benchmark
  )
endif

# Add a test target for the CMSIS-DSP F64 test
if not use_f32
  test(
    'test_cmsis_dsp_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_cmsis_dsp_f64_exe.full_path()],
    is_parallel: false,
    timeout: 15, # Longer timeout for benchmark
  )
endif

# CMSIS-DSP Benchmark (F32 mode)
if use_f32
  test_cmsis_dsp_benchmark_f32_exe = executable(
  'test_cmsis_dsp_benchmark_f32',
  ['cmsis_dsp_benchmark.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: [include_directories('.', '../include')],
  dependencies: [qemu_harness_dep, cmsis_dsp_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
  )
endif

# Add a test target for the CMSIS-DSP benchmark F32 test
if use_f32
  test(
    'test_cmsis_dsp_benchmark_f32',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_cmsis_dsp_benchmark_f32_exe.full_path()],
    is_parallel: false,
    timeout: 15, # Longer timeout for benchmark
  )
endif

# CMSIS-DSP Benchmark (F64 mode - only built when F64 is enabled)
if not use_f32
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
if use_f32
  test_exp_rs_integration_f32_exe = executable(
  'test_exp_rs_integration_f32',
  ['exp_rs_integration_test.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: [include_directories('.', '../include')],
  dependencies: [qemu_harness_dep, exp_rs_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
  )
endif

# Add a test target for the exp-rs integration F32 test
if use_f32
  test(
    'test_exp_rs_integration_f32',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_exp_rs_integration_f32_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# exp-rs Integration test - F64 mode (only when F64 is enabled)
if not use_f32
  test_exp_rs_integration_f64_exe = executable(
    'test_exp_rs_integration_f64',
    ['exp_rs_integration_test.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
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
if use_f32
  test_exp_rs_cmsis_benchmark_f32_exe = executable(
  'test_exp_rs_cmsis_benchmark_f32',
  ['exp_rs_cmsis_benchmark.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: [include_directories('.', '../include')],
  dependencies: [qemu_harness_dep, exp_rs_dep, cmsis_dsp_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
  )
endif

# Add a test target for the exp-rs CMSIS Integration Benchmark F32
if use_f32
  test(
    'test_exp_rs_cmsis_benchmark_f32',
    find_program('qemu-system-arm'),
    args: common_test_args
    + ['-kernel', test_exp_rs_cmsis_benchmark_f32_exe.full_path()],
    is_parallel: false,
    timeout: 60, # Longer timeout for benchmark
  )
endif

# exp-rs CMSIS Integration Benchmark (F64 mode - only built when F64 is enabled)
if not use_f32
  test_exp_rs_cmsis_benchmark_f64_exe = executable(
    'test_exp_rs_cmsis_benchmark_f64',
    ['exp_rs_cmsis_benchmark.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
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
if use_f32
  dwt_timer_test_exe = executable(
  'dwt_timer_test',
  ['dwt_timer_test.c', 'qemu_harness/vector_table_m7.c'],
  include_directories: include_directories('.'),
  dependencies: [qemu_harness_dep],
  link_args: common_link_args,
  c_args: ['-DDEF_USE_F32'],
  install: false,
  )
endif

# Add a test target for the DWT timer test
if use_f32
  test(
    'dwt_timer_test',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', dwt_timer_test_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Batch Performance test (F32 mode)
if use_f32
  batch_performance_test_f32_exe = executable(
    'batch_performance_test_f32',
    ['batch_performance_test.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F32'],
    install: false,
  )
endif

# Add a test target for the batch performance test F32
if use_f32
  test(
    'batch_performance_test_f32',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', batch_performance_test_f32_exe.full_path()],
    is_parallel: false,
    timeout: 30,  # Longer timeout for performance test
  )
endif

# Batch Performance test (F64 mode - only built when F64 is enabled)
if not use_f32
  batch_performance_test_f64_exe = executable(
    'batch_performance_test_f64',
    ['batch_performance_test.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64', '-DARM_MATH_DOUBLE'],
    install: false,
  )

  # Add a test target for the batch performance test F64
  test(
    'batch_performance_test_f64',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', batch_performance_test_f64_exe.full_path()],
    is_parallel: false,
    timeout: 30,  # Longer timeout for performance test
  )
endif

# Test for batch parameter order debugging
if not use_f32
  test_batch_param_order_exe = executable(
    'test_batch_param_order',
    ['test_batch_param_order.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64'],
    install: false,
  )

  test(
    'test_batch_param_order',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_batch_param_order_exe.full_path()],
    is_parallel: false,
    timeout: 5,
  )
endif

# Test for BatchBuilder API
if not use_f32
  test_batch_builder_exe = executable(
    'test_batch_builder',
    ['test_batch_builder.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64'],
    install: false,
  )

  test(
    'test_batch_builder',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_batch_builder_exe.full_path()],
    is_parallel: false,
    timeout: 10,
  )
endif

# Test for setup timing
if not use_f32
  test_setup_timing_exe = executable(
    'test_setup_timing',
    ['test_setup_timing.c', 'register_test_functions.c', 'qemu_harness/vector_table_m7.c'],
    include_directories: include_directories('.', '../include'),
    dependencies: [qemu_harness_dep, exp_rs_dep],
    link_args: common_link_args,
    c_args: ['-DDEF_USE_F64'],
    install: false,
  )

  test(
    'test_setup_timing',
    find_program('qemu-system-arm'),
    args: common_test_args + ['-kernel', test_setup_timing_exe.full_path()],
    is_parallel: false,
    timeout: 30,
  )
endif
