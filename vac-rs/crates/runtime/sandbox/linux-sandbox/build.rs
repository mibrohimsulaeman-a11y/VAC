use std::env;
use std::path::Path;
use std::path::PathBuf;

fn main() {
    // Tell rustc/clippy that this is an expected cfg value.
    println!("cargo:rustc-check-cfg=cfg(vendored_bwrap_available)");
    println!("cargo:rerun-if-env-changed=VAC_BWRAP_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_ALLOW_CROSS");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_PATH");
    println!("cargo:rerun-if-env-changed=PKG_CONFIG_SYSROOT_DIR");
    println!("cargo:rerun-if-env-changed=VAC_SKIP_VENDORED_BWRAP");
    println!("cargo:rerun-if-env-changed=VAC_BUILD_VENDORED_BWRAP");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_VENDORED_BWRAP");

    // Rebuild if the vendored bwrap sources change.
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let vendor_dir = manifest_dir.join("../vendor/bubblewrap");
    println!(
        "cargo:rerun-if-changed={}",
        vendor_dir.join("bubblewrap.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vendor_dir.join("bind-mount.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vendor_dir.join("network.c").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        vendor_dir.join("utils.c").display()
    );

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "linux" {
        return;
    }

    if env::var_os("VAC_SKIP_VENDORED_BWRAP").is_some() {
        println!(
            "cargo:warning=skipping build-time bubblewrap because VAC_SKIP_VENDORED_BWRAP is set"
        );
        return;
    }

    let Some(src_dir) = resolve_optional_bwrap_source_dir(&manifest_dir) else {
        return;
    };

    if let Err(err) = try_build_vendored_bwrap(src_dir) {
        panic!("failed to compile vendored bubblewrap for Linux target: {err}");
    }
}

fn try_build_vendored_bwrap(src_dir: PathBuf) -> Result<(), String> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").map_err(|err| err.to_string())?);
    let libcap = pkg_config::Config::new()
        .probe("libcap")
        .map_err(|err| format!("libcap not available via pkg-config: {err}"))?;

    let config_h = out_dir.join("config.h");
    std::fs::write(
        &config_h,
        r#"#pragma once
#define PACKAGE_STRING "bubblewrap built at vac build-time"
"#,
    )
    .map_err(|err| format!("failed to write {}: {err}", config_h.display()))?;

    let mut build = cc::Build::new();
    build
        .file(src_dir.join("bubblewrap.c"))
        .file(src_dir.join("bind-mount.c"))
        .file(src_dir.join("network.c"))
        .file(src_dir.join("utils.c"))
        .include(&out_dir)
        .include(&src_dir)
        .define("_GNU_SOURCE", None)
        // Rename `main` so we can call it via FFI.
        .define("main", Some("bwrap_main"));
    for include_path in libcap.include_paths {
        // Use -idirafter so target sysroot headers win (musl cross builds),
        // while still allowing libcap headers from the host toolchain.
        build.flag(format!("-idirafter{}", include_path.display()));
    }

    build.compile("build_time_bwrap");
    println!("cargo:rustc-cfg=vendored_bwrap_available");
    Ok(())
}

/// Resolve the optional bubblewrap source directory used for build-time compilation.
///
/// Priority:
/// 1. `VAC_BWRAP_SOURCE_DIR` points at an existing bubblewrap checkout.
/// 2. The `vendored-bwrap` feature or `VAC_BUILD_VENDORED_BWRAP=1` uses `vac-rs/vendor/bubblewrap`.
/// 3. Default developer builds do not compile build-time bubblewrap.
fn resolve_optional_bwrap_source_dir(manifest_dir: &Path) -> Option<PathBuf> {
    if let Ok(path) = env::var("VAC_BWRAP_SOURCE_DIR") {
        let src_dir = PathBuf::from(path);
        if src_dir.exists() {
            return Some(src_dir);
        }
        panic!(
            "VAC_BWRAP_SOURCE_DIR was set but does not exist: {}",
            src_dir.display()
        );
    }

    if env::var_os("VAC_BUILD_VENDORED_BWRAP").is_none()
        && env::var_os("CARGO_FEATURE_VENDORED_BWRAP").is_none()
    {
        return None;
    }

    let vendor_dir = manifest_dir.join("../vendor/bubblewrap");
    if vendor_dir.exists() {
        return Some(vendor_dir);
    }

    panic!(
        "VAC_BUILD_VENDORED_BWRAP is set, but vendored bubblewrap was not found at {}.\n\
Set VAC_BWRAP_SOURCE_DIR to an existing checkout or vendor bubblewrap under vac-rs/vendor.",
        vendor_dir.display()
    );
}
