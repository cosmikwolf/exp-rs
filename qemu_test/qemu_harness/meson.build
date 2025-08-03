# Compile the QEMU test harness library
if meson.is_cross_build()
  # QEMU test harness sources
  qemu_harness_inc = include_directories('.')

  qemu_harness_sources = [
    'qemu_test_harness.c',
    'stubs.c',
    'timer_overflow.c',
  ]
  qemu_harness_lib = static_library(
    'qemu_test_harness',
    qemu_harness_sources,
    include_directories: qemu_harness_inc,
    install: false,
  )

  # Declare dependency for QEMU tests
  qemu_harness_dep = declare_dependency(
    include_directories: qemu_harness_inc,
    link_with: qemu_harness_lib,
  )

  # Make the dependency available globally
  meson.override_dependency('qemu_test_harness', qemu_harness_dep)
else
  # Create an empty dependency for non-QEMU builds
  qemu_harness_dep = declare_dependency()
  meson.override_dependency('qemu_test_harness', qemu_harness_dep)
endif

# Add executable for QEMU test with linker script
if meson.is_cross_build()
  # qemu_test_exe = executable(
  #   'qemu_test',
  #   'qemu_test.c',
  #   include_directories: include_directories('.'),
  #   link_with: [qemu_harness_lib],
  #   link_args: ['-T', 'qemu_harness/mps2_an500.ld'],
  #   install: false,
  # )
  #
  # test('qemu_test',
  #   find_program('qemu-system-arm'),
  #   args: [
  #     '-M', 'mps2-an500',
  #     '-cpu', 'cortex-m7',
  #     '-nographic',
  #     '-semihosting',
  #     '-kernel', qemu_test_exe.full_path(),
  #   ],
  #   is_parallel: false,
  #   timeout: 60,
  #   env: ['QEMU_AUDIO_DRV=none'],
  # )
endif
