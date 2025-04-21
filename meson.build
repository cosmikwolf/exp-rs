# To build for QEMU:
# meson setup build-qemu --cross-file qemu_test/qemu_harness/arm-cortex-m7-qemu.ini -Dbuild_target=qemu_tests
# meson compile -C build-qemu
# meson test -C build-qemu

project('exp-rs', 'c', version: '0.1.2', meson_version: '>=1.0.0')

# Get configuration options
enable_f64 = get_option('enable_f64')

# Set Rust features based on configuration
if enable_f64
  rust_features = ['f64']
else
  rust_features = ['f32']
endif

# Build Rust static library using cargo build
exp_rs_build = custom_target(
  'cargo_build',
  output: 'libexp_rs.a',
  command: [
    'pwd',
    '&&',
    'cargo',
    'build',
    '--target', 'thumbv7em-none-eabihf',
    '--release',
    '--no-default-features',
    '--features', ','.join(rust_features),
    '--lib', '&&',
    'cp',
    meson.source_root() + '/target/thumbv7em-none-eabihf/release/libexp_rs.a',
    '@OUTPUT@',
  ],
  build_always_stale: true,
  install: false,
  console: true,
)

exp_rs_dep = declare_dependency(
  # sources: ['src/ffi.rs'],
  include_directories: include_directories('.'),
  link_with: exp_rs_build,
)

# Include the qemu_test subdir if building qemu tests
if get_option('build_target') == 'qemu_tests'
  subdir('qemu_test')
endif
