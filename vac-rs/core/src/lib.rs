//! Thin VAC core orchestration/export crate.
//!
//! O5/O6 zero-residual decomposition keeps `vac-core` as a small facade over
//! control-plane, configuration, project-workspace, and CLI compatibility
//! exports. Historical runtime/session compatibility bridges were removed for
//! zero-residual runtime validation; domain implementation now lives behind
//! direct capability crates instead of legacy path-bridge includes.

#![deny(clippy::print_stdout, clippy::print_stderr)]

pub use vac_control_plane::control_plane;
pub use vac_control_plane::local_runtime;

pub mod config {
    use std::path::PathBuf;

    pub fn find_vac_home() -> std::io::Result<PathBuf> {
        if let Some(path) = std::env::var_os("VAC_HOME") {
            return Ok(PathBuf::from(path));
        }
        dirs::home_dir()
            .map(|home| home.join(".vac"))
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "home directory is unavailable"))
    }

    pub mod edit {
        use std::fs;
        use std::path::Path;

        use anyhow::Context;
        use toml_edit::DocumentMut;
        use toml_edit::Item as TomlItem;
        use toml_edit::Table as TomlTable;

        #[derive(Clone, Debug)]
        pub enum ConfigEdit {
            SetPath { segments: Vec<String>, value: TomlItem },
            ClearPath { segments: Vec<String> },
        }

        pub fn apply_blocking(
            vac_home: &Path,
            _cwd: Option<&Path>,
            edits: &[ConfigEdit],
        ) -> anyhow::Result<()> {
            fs::create_dir_all(vac_home)
                .with_context(|| format!("failed to create VAC home at {}", vac_home.display()))?;
            let config_path = vac_home.join("config.toml");
            let raw = fs::read_to_string(&config_path).unwrap_or_default();
            let mut document = raw.parse::<DocumentMut>().unwrap_or_default();

            for edit in edits {
                match edit {
                    ConfigEdit::SetPath { segments, value } => set_path(&mut document, segments, value.clone()),
                    ConfigEdit::ClearPath { segments } => clear_path(&mut document, segments),
                }
            }

            fs::write(&config_path, document.to_string())
                .with_context(|| format!("failed to write {}", config_path.display()))?;
            Ok(())
        }

        fn set_path(document: &mut DocumentMut, segments: &[String], value: TomlItem) {
            let Some((leaf, parents)) = segments.split_last() else {
                return;
            };
            let mut item = document.as_item_mut();
            for segment in parents {
                if !item[segment.as_str()].is_table() {
                    item[segment.as_str()] = TomlItem::Table(TomlTable::new());
                }
                item = &mut item[segment.as_str()];
            }
            item[leaf.as_str()] = value;
        }

        fn clear_path(document: &mut DocumentMut, segments: &[String]) {
            let Some((leaf, parents)) = segments.split_last() else {
                return;
            };
            let mut item = document.as_item_mut();
            for segment in parents {
                if !item[segment.as_str()].is_table() {
                    return;
                }
                item = &mut item[segment.as_str()];
            }
            if let Some(table) = item.as_table_mut() {
                table.remove(leaf.as_str());
            }
        }
    }
}

pub use vac_capability_ownership::project_workspace;

pub mod util {
    use vac_protocol::ThreadId;

    pub fn resume_command(thread_name: Option<&str>, thread_id: Option<ThreadId>) -> Option<String> {
        let selector = thread_name
            .filter(|name| !name.trim().is_empty())
            .map(shell_quote)
            .or_else(|| thread_id.map(|id| id.to_string()))?;
        Some(format!("vac resume {selector}"))
    }

    fn shell_quote(value: &str) -> String {
        if value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.' | '/' | ':'))
        {
            return value.to_string();
        }
        format!("'{}'", value.replace("'", "'\\''"))
    }
}
