use std::path::Path;

use tempfile::TempDir;
use vac_arg0::Arg0DispatchPaths;
use vac_arg0::Arg0PathEntryGuard;
use vac_arg0::arg0_dispatch;

pub struct TestBinaryDispatchGuard {
    _vac_home: TempDir,
    arg0: Arg0PathEntryGuard,
    _previous_vac_home: Option<std::ffi::OsString>,
}

impl TestBinaryDispatchGuard {
    pub fn paths(&self) -> &Arg0DispatchPaths {
        self.arg0.paths()
    }
}

pub enum TestBinaryDispatchMode {
    DispatchArg0Only,
    Skip,
    InstallAliases,
}

pub fn configure_test_binary_dispatch<F>(
    vac_home_prefix: &str,
    classify: F,
) -> Option<TestBinaryDispatchGuard>
where
    F: FnOnce(&str, Option<&str>) -> TestBinaryDispatchMode,
{
    let mut args = std::env::args_os();
    let argv0 = args.next().unwrap_or_default();
    let exe_name = Path::new(&argv0)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let argv1 = args.next();
    match classify(exe_name, argv1.as_deref().and_then(|arg| arg.to_str())) {
        TestBinaryDispatchMode::DispatchArg0Only => {
            let _ = arg0_dispatch();
            None
        }
        TestBinaryDispatchMode::Skip => None,
        TestBinaryDispatchMode::InstallAliases => {
            let vac_home = match tempfile::Builder::new().prefix(vac_home_prefix).tempdir() {
                Ok(vac_home) => vac_home,
                Err(error) => panic!("failed to create test VAC_HOME: {error}"),
            };
            let previous_vac_home = std::env::var_os("VAC_HOME");
            // Safety: this runs from a test ctor before test threads begin.
            // SAFETY: Unsafe operation is retained behind the existing module boundary; caller must uphold the surrounding invariants until TV verification confirms the refactor.
            unsafe {
                std::env::set_var("VAC_HOME", vac_home.path());
            }

            let arg0 = match arg0_dispatch() {
                Some(arg0) => arg0,
                None => panic!("failed to configure arg0 dispatch aliases for test binary"),
            };
            match previous_vac_home.as_ref() {
                // SAFETY: Unsafe operation is retained behind the existing module boundary; caller must uphold the surrounding invariants until TV verification confirms the refactor.
                Some(value) => unsafe {
                    std::env::set_var("VAC_HOME", value);
                },
                // SAFETY: Unsafe operation is retained behind the existing module boundary; caller must uphold the surrounding invariants until TV verification confirms the refactor.
                None => unsafe {
                    std::env::remove_var("VAC_HOME");
                },
            }

            Some(TestBinaryDispatchGuard {
                _vac_home: vac_home,
                arg0,
                _previous_vac_home: previous_vac_home,
            })
        }
    }
}
