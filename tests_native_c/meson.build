cc = meson.get_compiler('c')
m_dep = cc.find_library('m', required: true)
dl_dep = cc.find_library('dl', required: false)

inc = include_directories('.', '../include')

# Define test programs that are still active
test_programs = {
  # Basic tests
  'test_simple_batch': {
    'sources': ['test_simple_batch.c'],
    'deps': [exp_rs_dep, m_dep]
  },
  
  # Session API tests
  'test_session_api': {
    'sources': ['test_session_api.c'],
    'deps': [exp_rs_dep, m_dep]
  },
  
  # Arena tests
  'test_arena_integration': {
    'sources': ['test_arena_integration.c'],
    'deps': [exp_rs_dep, m_dep]
  },
  
  # Embedded tests
  'test_embedded_pool': {
    'sources': ['test_embedded_pool.c'],
    'deps': [exp_rs_dep, m_dep]
  },
  
  # Timing tests
  'test_detailed_timing': {
    'sources': ['test_detailed_timing.c'],
    'deps': [exp_rs_dep, m_dep]
  },
  'test_microbenchmark': {
    'sources': ['test_microbenchmark.c'],
    'deps': [exp_rs_dep, m_dep]
  },
  'test_realistic_timing': {
    'sources': ['test_realistic_timing.c'],
    'deps': [exp_rs_dep, m_dep]
  },
  'test_proper_timing': {
    'sources': ['test_proper_timing.c'],
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

timing_tests = [
  'test_detailed_timing',
  'test_microbenchmark',
  'test_realistic_timing',
  'test_proper_timing'
]

run_target('test-timing',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@'] + timing_tests
)  

memory_tests = [
  'test_embedded_pool',
  'test_arena_integration'
]

run_target('test-memory',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@'] + memory_tests
)

functional_tests = [
  'test_simple_batch',
  'test_session_api',
  'test_arena_integration',
  'test_embedded_pool'
]

run_target('test-functional',
  command: ['meson', 'test', '-C', '@BUILD_ROOT@'] + functional_tests
)