use std::path::PathBuf;

/// Returns the VAC home directory: `~/.vac/`
pub fn vac_home_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".vac")
}
