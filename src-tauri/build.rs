fn main() {
    #[cfg(windows)]
    stage_windows_runtime();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        link_clang_runtime();
    }
    tauri_build::build()
}

/// ggml's Metal code uses `@available(...)` checks. With a deployment target
/// below the SDK (we support macOS 13) clang turns those into calls to
/// `__isPlatformVersionAtLeast` from its compiler runtime, which rustc does
/// not link on its own.
fn link_clang_runtime() {
    let output = std::process::Command::new("clang")
        .arg("--print-resource-dir")
        .output()
        .expect("clang must be installed (Xcode command line tools)");
    let resource_dir = String::from_utf8(output.stdout).expect("utf-8 path");
    println!("cargo:rustc-link-search=native={}/lib/darwin", resource_dir.trim());
    println!("cargo:rustc-link-lib=static=clang_rt.osx");
}

/// Windows loads DLLs from the exe's folder. Collect everything the app needs
/// at runtime into `runtime/`, which `tauri.windows.conf.json` installs next
/// to the exe: transcribe.cpp + ggml (incl. the Vulkan and per-CPU backend
/// modules, staged into the profile dir by transcribe-cpp-sys) and the VC++
/// runtime, so a fresh Windows install needs nothing extra.
#[cfg(windows)]
fn stage_windows_runtime() {
    use std::path::{Path, PathBuf};

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let runtime = manifest.join("runtime");
    std::fs::create_dir_all(&runtime).expect("create runtime dir");

    // OUT_DIR = <target>/<profile>/build/<crate>-<hash>/out
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let profile_dir = out_dir.ancestors().nth(3).expect("profile dir").to_path_buf();

    let copy = |src: &Path| {
        if let Some(name) = src.file_name() {
            std::fs::copy(src, runtime.join(name)).unwrap_or_else(|e| panic!("copy {}: {e}", src.display()));
        }
    };

    let engine_dlls: Vec<PathBuf> = std::fs::read_dir(&profile_dir)
        .expect("read profile dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            name.ends_with(".dll") && (name.starts_with("ggml") || name.starts_with("transcribe"))
        })
        .collect();
    if engine_dlls.is_empty() {
        panic!("no transcribe/ggml DLLs in {} – is transcribe-cpp built with `shared`?", profile_dir.display());
    }
    engine_dlls.iter().for_each(|p| copy(p));

    let system32 = PathBuf::from(std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into())).join("System32");
    for name in ["vcruntime140.dll", "vcruntime140_1.dll", "msvcp140.dll"] {
        copy(&system32.join(name));
    }

    println!("cargo:rerun-if-changed=build.rs");
}
