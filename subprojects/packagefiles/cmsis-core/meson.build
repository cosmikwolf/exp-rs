project('cmsis-core', 'c',
  version : '6.0.0',
  default_options : ['warning_level=0']
)

# Define include directories
cmsis_core_inc = include_directories(
  'CMSIS/Core/Include'
)

# Provide a dependency for other subprojects to use
cmsis_core_dep = declare_dependency(
  include_directories : cmsis_core_inc
)