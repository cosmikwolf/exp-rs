# To build for QEMU:
# meson setup build-qemu --cross-file qemu_test/qemu_harness/arm-cortex-m7-qemu.ini -Dbuild_target=exp_rs_qemu_tests
# meson compile -C build-qemu
# meson test -C build-qemu

project('exp-rs', 'c', version: '0.1.2', meson_version: '>=1.0.0')

# Get configuration options
enable_f64 = get_option('enable_f64')

# Set Rust features based on configuration
if enable_f64
  rust_features = ['f64']  # Use standard malloc/free
else
  rust_features = ['f32']  # Use standard malloc/free
endif

# Build Rust static library using cargo build
exp_rs_build = custom_target(
  'cargo_build',
  output: 'libexp_rs.a',
  # input: meson.current_build_dir()
  # + '/exp_rs/thumbv7em-none-eabihf/release/libexp_rs.a',
  # install_dir: meson.current_build_dir(),
  command: [
    'cargo',
    'build',
    '--target', 'thumbv7em-none-eabihf',
    '--release',
    '--no-default-features',
    '--features', ','.join(rust_features),
    '--target-dir', meson.current_build_dir(),
    '--manifest-path', meson.current_source_dir() + '/Cargo.toml',
    # '--target-dir', meson.current_build_dir() + '/exp-rs',
    '--lib', '&&',
    'cp',
    meson.current_source_dir() + '/include/exp_rs.h',
    meson.current_build_dir(),
    '&&',
    'cp',
    meson.current_build_dir() + '/thumbv7em-none-eabihf/release/libexp_rs.a',
    # meson.current_source_dir()
    # + '/target/thumbv7em-none-eabihf/release/libexp_rs.a',
    meson.current_build_dir()
    + '/libexp_rs.a',
  ],
  build_always_stale: false,
  build_by_default: false,
  # install: true,
  console: true,
)

exp_rs_dep = declare_dependency(
  # sources: ['src/ffi.rs'],
  include_directories: include_directories('include'),
  link_with: exp_rs_build,
)

# Include the qemu_test subdir if building qemu tests
if get_option('enable_exprs_qemu_tests')
  subdir('qemu_test')
endif
