cc = meson.get_compiler('c')
m_dep = cc.find_library('m', required: true)
dl_dep = cc.find_library('dl', required: false)

inc = include_directories('.')

# Common allocator object for all tests (required when custom_cbindgen_alloc is enabled)
common_allocator_lib = static_library(
  'common_allocator',
  'common_allocator.c',
  include_directories: inc,
  install: false,
)

# Create dependency that includes the library
common_allocator_dep = declare_dependency(link_with: common_allocator_lib, include_directories: inc)

# Define test programs that are still active
test_programs = {
  # Batch API tests
  'test_batch_api': {
    'sources': ['test_batch_api.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Memory management tests
  'test_memory_management': {
    'sources': ['test_memory_management.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Embedded tests
  'test_embedded_pool': {
    'sources': ['test_embedded_pool.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Performance tests (consolidated)
  'test_performance': {
    'sources': ['test_performance.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Error handling tests
  'test_error_handling': {
    'sources': ['test_error_handling.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Expression function tests
  'test_expression_functions': {
    'sources': ['test_expression_functions.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Pattern isolation tests
  'test_pattern_isolation': {
    'sources': ['test_pattern_isolation.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Panic handler tests
  'test_panic_handler': {
    'sources': ['test_panic_handler.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Empty context tests
  'test_empty_context': {
    'sources': ['test_empty_context.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Memory and arena analysis
  'test_memory_analysis': {
    'sources': ['test_memory_analysis.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Batch memory tracking tests
  'test_batch_memory_tracking': {
    'sources': ['test_batch_memory_tracking.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Batch clear functionality tests
  'test_batch_clear': {
    'sources': ['test_batch_clear.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Batch clear iteration tests
  'test_batch_clear_iterations': {
    'sources': ['test_batch_clear_iterations.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Double-free protection tests
  'test_double_free': {
    'sources': ['test_double_free.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Clear and free interaction tests
  'test_clear_then_free': {
    'sources': ['test_clear_then_free.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },

  # Memory deallocation verification tests
  'test_actual_memory_free': {
    'sources': ['test_actual_memory_free.c'],
    'deps': [exp_rs_dep, m_dep, common_allocator_dep],
  },
}

# Build all test executables
foreach test_name, test_info : test_programs
  exe = executable(
    test_name,
    test_info['sources'],
    include_directories: inc,
    dependencies: test_info['deps'],
    install: false,
  )

  test(test_name, exe, timeout: 30, is_parallel: true)
endforeach

# Test groups for targeted testing
run_target('test-all', command: ['meson', 'test', '-C', '@BUILD_ROOT@'])

performance_tests = ['test_performance']

run_target(
  'test-performance',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@'] + performance_tests,
)

memory_tests = [
  'test_embedded_pool',
  'test_memory_management',
  'test_memory_analysis',
  'test_batch_memory_tracking',
  'test_batch_clear_iterations',
]

run_target(
  'test-memory',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@'] + memory_tests,
)

functional_tests = [
  'test_batch_api',
  'test_memory_management',
  'test_embedded_pool',
  'test_batch_clear',
]

run_target(
  'test-functional',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@'] + functional_tests,
)
