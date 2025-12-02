use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // 1) Explicit FFI lib dir + name
    if let (Ok(dir), Ok(name)) = (
        env::var("VELOX_FFI_LIB_DIR"),
        env::var("VELOX_FFI_LIB_NAME"),
    ) {
        println!("cargo:rustc-link-search=native={}", dir);
        println!("cargo:rustc-link-lib=dylib={}", name);
        println!("cargo:rustc-cfg=velox_native");
        return;
    }

    // 2) Full path to FFI lib
    if let Ok(full) = env::var("VELOX_FFI_LIB") {
        let p = PathBuf::from(&full);
        if let (Some(dir), Some(file)) = (p.parent(), p.file_name().and_then(|f| f.to_str())) {
            let stem = file
                .trim_start_matches("lib")
                .trim_end_matches(".so")
                .trim_end_matches(".dylib")
                .trim_end_matches(".dll");
            println!("cargo:rustc-link-search=native={}", dir.display());
            println!("cargo:rustc-link-lib=dylib={}", stem);
            println!("cargo:rustc-cfg=velox_native");
            return;
        }
    }

    // 3) Prebuilt shim location
    if let Ok(dir) = env::var("VELOX_SHIM_LIB_DIR") {
        println!("cargo:rustc-link-search=native={}", dir);
        println!("cargo:rustc-link-lib=dylib=velox_shim");
        println!("cargo:rustc-cfg=velox_native");
        return;
    }

    // 4) Attempt to build shim from source if present
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let project_root = manifest_dir
        .parent()
        .expect("velox-ffi is in crates/velox-ffi")
        .parent()
        .expect("project root");
    let shim_dir = project_root.join("cpp/velox_shim");
    if !shim_dir.exists() {
        // No shim available; skip linking (runtime may use libloading)
        return;
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let build_dir = out_dir.join("velox_shim_build");
    let _ = std::fs::create_dir_all(&build_dir);

    // Configure stub/native mode and CMAKE_PREFIX_PATH hints
    let mut cmake_cfg = Command::new("cmake");
    cmake_cfg.current_dir(&build_dir);
    cmake_cfg.arg(shim_dir.as_os_str());
    // Default: stub ON unless VELOX_BUILD_DIR/VELOX_HOME present or VELOX_SHIM_STUB explicitly set
    let explicit_stub = env::var("VELOX_SHIM_STUB").ok();
    let velox_build = env::var("VELOX_BUILD_DIR").ok();
    let velox_home = env::var("VELOX_HOME").ok();
    let use_native = explicit_stub.as_deref() == Some("OFF")
        || (explicit_stub.is_none() && (velox_build.is_some() || velox_home.is_some()));
    cmake_cfg.arg(format!(
        "-DVELOX_SHIM_STUB={}",
        if use_native { "OFF" } else { "ON" }
    ));
    if let Some(dir) = velox_build.or(velox_home) {
        // Help CMake find Velox with a prefix path
        cmake_cfg.arg(format!("-DCMAKE_PREFIX_PATH={}", dir));
    }
    let status = cmake_cfg.status().expect("failed to run cmake");
    if !status.success() {
        panic!("cmake configuration for velox_shim failed");
    }

    let status = Command::new("cmake")
        .current_dir(&build_dir)
        .arg("--build")
        .arg(".")
        .status()
        .expect("failed to build velox_shim");
    if !status.success() {
        panic!("building velox_shim failed");
    }

    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=dylib=velox_shim");
    println!("cargo:rustc-cfg=velox_native");
}
