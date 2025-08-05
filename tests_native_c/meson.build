cc = meson.get_compiler('c')
m_dep = cc.find_library('m', required: true)
dl_dep = cc.find_library('dl', required: false)

inc = include_directories('.', '../include')

# Define test programs that are still active
test_programs = {
  # Batch API tests
  'test_batch_api': {
    'sources': ['test_batch_api.c'],
    'deps': [exp_rs_dep, m_dep]
  },

  # Session API tests
  'test_session_api': {
    'sources': ['test_session_api.c'],
    'deps': [exp_rs_dep, m_dep]
  },

  # Memory management tests
  'test_memory_management': {
    'sources': ['test_memory_management.c'],
    'deps': [exp_rs_dep, m_dep]
  },

  # Embedded tests
  'test_embedded_pool': {
    'sources': ['test_embedded_pool.c'],
    'deps': [exp_rs_dep, m_dep]
  },

  # Performance tests (consolidated)
  'test_performance': {
    'sources': ['test_performance.c'],
    'deps': [exp_rs_dep, m_dep]
  },

  # Error handling tests
  'test_error_handling': {
    'sources': ['test_error_handling.c'],
    'deps': [exp_rs_dep, m_dep]
  },

  # Expression function tests
  'test_expression_functions': {
    'sources': ['test_expression_functions.c'],
    'deps': [exp_rs_dep, m_dep]
  },

  # Pattern isolation tests
  'test_pattern_isolation': {
    'sources': ['test_pattern_isolation.c'],
    'deps': [exp_rs_dep, m_dep]
  },

  # Panic handler tests
  'test_panic_handler': {
    'sources': ['test_panic_handler.c'],
    'deps': [exp_rs_dep, m_dep]
  },

  # Empty context tests
  'test_empty_context': {
    'sources': ['test_empty_context.c'],
    'deps': [exp_rs_dep, m_dep]
  }
}

# Build all test executables
foreach test_name, test_info : test_programs
  exe = executable(test_name,
    test_info['sources'],
    include_directories: inc,
    dependencies: test_info['deps'],
    install: false
  )

  test(test_name, exe,
    timeout: 30,
    is_parallel: true
  )
endforeach

# Test groups for targeted testing
run_target('test-all',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@']
)

performance_tests = [
  'test_performance'
]

run_target('test-performance',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@'] + performance_tests
)

memory_tests = [
  'test_embedded_pool',
  'test_memory_management'
]

run_target('test-memory',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@'] + memory_tests
)

functional_tests = [
  'test_batch_api',
  'test_session_api',
  'test_memory_management',
  'test_embedded_pool'
]

run_target('test-functional',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@'] + functional_tests
)
