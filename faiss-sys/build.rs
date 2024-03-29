use std::env;
use std::path::{Path, PathBuf};

fn main() {
    #[cfg(feature = "static")]
    static_link_faiss();
    #[cfg(not(feature = "static"))]
    println!("cargo:rustc-link-lib=faiss_c");
}

#[cfg(target_os = "macos")]
fn macos_find_omp() -> Option<PathBuf> {
    let prefix = env::var("HOMEBREW_PREFIX").ok()?;
    let base = Path::new(&prefix).join("opt/libomp");

    if base.exists() {
        Some(base)
    } else {
        None
    }
}

#[cfg(target_os = "macos")]
fn macos_cfg(cfg: &mut cmake::Config) -> Option<String> {
    let Some(omp_root) = macos_find_omp() else {
        return None;
    };

    cfg.cxxflag(&format!("-I{}/include", omp_root.display()));

    Some(format!("{}/lib", omp_root.display()))
}

#[cfg(feature = "static")]
fn static_link_faiss() {
    let mut cfg = cmake::Config::new("faiss");

    #[cfg(target_os = "macos")]
    let omp_path = macos_cfg(&mut cfg);

    cfg.define("FAISS_ENABLE_C_API", "ON")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("FAISS_ENABLE_GPU", if cfg!(feature = "gpu") {
            "ON"
        } else {
            "OFF"
        })
        .define("FAISS_ENABLE_PYTHON", "OFF")
        .define("BUILD_TESTING", "OFF")
        .very_verbose(true);

    let dst = cfg.build();
    let faiss_location = dst.join("lib");
    let faiss_c_location = dst.join("build/c_api");
    println!(
        "cargo:rustc-link-search=native={}",
        faiss_location.display()
    );
    println!(
        "cargo:rustc-link-search=native={}",
        faiss_c_location.display()
    );
    println!("cargo:rustc-link-lib=static=faiss_c");
    println!("cargo:rustc-link-lib=static=faiss");
    link_cxx();

    #[cfg(target_os = "macos")]
    if let Some(omp_path) = omp_path {
        println!("cargo:rustc-link-search=native={omp_path}");
    }

    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-lib=omp");

    #[cfg(not(target_os = "macos"))]
    println!("cargo:rustc-link-lib=gomp");

    println!("cargo:rustc-link-lib=blas");
    println!("cargo:rustc-link-lib=lapack");
    if cfg!(feature = "gpu") {
        let cuda_path = cuda_lib_path();
        println!("cargo:rustc-link-search=native={}/lib64", cuda_path);
        println!("cargo:rustc-link-lib=cudart");
        println!("cargo:rustc-link-lib=cublas");
    }
}

#[cfg(feature = "static")]
fn link_cxx() {
    let cxx = match std::env::var("CXXSTDLIB") {
        Ok(s) if s.is_empty() => None,
        Ok(s) => Some(s),
        Err(_) => {
            let target = std::env::var("TARGET").unwrap();
            if target.contains("msvc") {
                None
            } else if target.contains("apple")
                | target.contains("freebsd")
                | target.contains("openbsd")
            {
                Some("c++".to_string())
            } else {
                Some("stdc++".to_string())
            }
        }
    };
    if let Some(cxx) = cxx {
        println!("cargo:rustc-link-lib={}", cxx);
    }
}

#[cfg(feature = "static")]
fn cuda_lib_path() -> String {
    // look for CUDA_PATH in environment,
    // then CUDA_LIB_PATH,
    // then CUDA_INCLUDE_PATH
    if let Ok(cuda_path) = std::env::var("CUDA_PATH") {
        return cuda_path;
    }
    if let Ok(cuda_lib_path) = std::env::var("CUDA_LIB_PATH") {
        return cuda_lib_path;
    }
    if let Ok(cuda_include_path) = std::env::var("CUDA_INCLUDE_PATH") {
        return cuda_include_path;
    }

    panic!("Could not find CUDA: environment variables `CUDA_PATH`, `CUDA_LIB_PATH`, or `CUDA_INCLUDE_PATH` must be set");
}
