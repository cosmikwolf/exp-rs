# To build for QEMU:
# meson setup build-qemu --cross-file qemu_test/qemu_harness/arm-cortex-m7-qemu.ini -Dbuild_target=exp_rs_qemu_tests
# meson compile -C build-qemu
# meson test -C build-qemu

project('exp-rs', 'c', version: '0.1.2', meson_version: '>=1.0.0')

# Set Rust features based on configuration
if get_option('use_f32')
  rust_features = ['f32'] # Use standard malloc/free
else
  rust_features = []
endif

if get_option('custom_cbindgen_alloc')
  rust_features += 'custom_cbindgen_alloc'
endif

# Determine build profile based on build type
# build_type = get_option('buildtype')
# if build_type == 'debug' or build_type == 'debugoptimized'
#   message('Building in debug mode')
#   cargo_profile = 'debug'
#   cargo_profile_flag = []
# else
#   message('Building in release mode')
#   cargo_profile = 'release'
#   cargo_profile_flag = ['--release']
# endif
cargo_profile = 'release'
cargo_profile_flag = ['--release']

# Build Rust static library using cargo build
if get_option('test_native')
  # Native build for host tests
  exp_rs_build = custom_target(
    'cargo_build',
    output: 'libexp_rs.a',
    command: [
      'cargo',
      'build',
      cargo_profile_flag,
      '--features', ','.join(rust_features),
      '--target-dir', meson.current_build_dir(),
      '--manifest-path', meson.current_source_dir() + '/Cargo.toml',
      '--lib', '&&',
      'cp',
      meson.current_source_dir() + '/include/exp_rs.h',
      meson.current_build_dir(),
      '&&',
      'cp',
      meson.current_source_dir()
      + '/target/'
      + cargo_profile
      + '/libexp_rs.a',
      meson.current_build_dir()
      + '/libexp_rs.a',
    ],
    build_always_stale: false,
    build_by_default: false,
    console: true,
  )
else
  # Cross-compilation build for QEMU tests
  exp_rs_build = custom_target(
    'cargo_build',
    output: 'libexp_rs.a',
    command: [
      'cargo',
      'build',
      '-Zbuild-std=core,alloc,compiler_builtins',
      '--target', meson.current_source_dir() + '/targets/cortex-m7-strict.json',
      cargo_profile_flag,
      '--no-default-features',
      '--features', ','.join(rust_features),
      '--target-dir', meson.current_build_dir(),
      '--manifest-path', meson.current_source_dir() + '/Cargo.toml',
      '--lib', '&&',
      'cp',
      meson.current_source_dir() + '/include/exp_rs.h',
      meson.current_build_dir(),
      '&&',
      'cp',
      meson.current_build_dir()
      + '/cortex-m7-strict/'
      + cargo_profile
      + '/libexp_rs.a',
      meson.current_build_dir()
      + '/libexp_rs.a',
    ],
    build_always_stale: false,
    build_by_default: false,
    console: true,
  )
endif

exp_rs_dep = declare_dependency(
  # sources: ['src/ffi.rs'],
  include_directories: include_directories('include'),
  link_with: exp_rs_build,
)

# Include the qemu_test subdir if building qemu tests
if get_option('enable_exprs_qemu_tests')
  subdir('qemu_test')
endif

# Include native C tests if building for host system
if get_option('test_native')
  subdir('tests_native_c')
endif
