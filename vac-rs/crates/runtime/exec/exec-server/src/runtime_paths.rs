use std::path::PathBuf;

use vac_utils_absolute_path::AbsolutePathBuf;

/// Runtime paths needed by exec-server child processes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecServerRuntimePaths {
    /// Stable path to the VAC executable used to launch hidden helper modes.
    pub vac_self_exe: AbsolutePathBuf,
    /// Path to the Linux sandbox helper alias used when the platform sandbox
    /// needs to re-enter VAC by argv0.
    pub vac_linux_sandbox_exe: Option<AbsolutePathBuf>,
}

impl ExecServerRuntimePaths {
    pub fn from_optional_paths(
        vac_self_exe: Option<PathBuf>,
        vac_linux_sandbox_exe: Option<PathBuf>,
    ) -> std::io::Result<Self> {
        let vac_self_exe = vac_self_exe.ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "VAC executable path is not configured",
            )
        })?;
        Self::new(vac_self_exe, vac_linux_sandbox_exe)
    }

    pub fn new(
        vac_self_exe: PathBuf,
        vac_linux_sandbox_exe: Option<PathBuf>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            vac_self_exe: absolute_path(vac_self_exe)?,
            vac_linux_sandbox_exe: vac_linux_sandbox_exe.map(absolute_path).transpose()?,
        })
    }
}

fn absolute_path(path: PathBuf) -> std::io::Result<AbsolutePathBuf> {
    AbsolutePathBuf::from_absolute_path(path.as_path())
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))
}
