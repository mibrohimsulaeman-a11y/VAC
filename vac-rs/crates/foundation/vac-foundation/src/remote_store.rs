use crate::remote_connection::RemoteConnection;
use std::path::PathBuf;
use std::sync::Arc;

pub struct RemoteStore {}

fn shell_single_quote_arg(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

fn mkdir_command(path: &str) -> String {
    format!("mkdir -p {}", shell_single_quote_arg(path))
}

impl RemoteStore {
    /// Get the remote session store path (relative to remote working directory)
    pub fn get_remote_session_store_path() -> PathBuf {
        PathBuf::from(".vac").join("session")
    }

    /// Get the absolute remote session store path by canonicalizing on the remote host
    pub async fn get_absolute_remote_session_store_path(
        conn: &Arc<RemoteConnection>,
    ) -> Result<String, String> {
        let relative_path = Self::get_remote_session_store_path();
        let relative_path_str = relative_path.to_string_lossy().to_string();

        if let Err(e) = conn
            .execute_command(&mkdir_command(&relative_path_str), None, None)
            .await
        {
            return Err(format!("Failed to create remote session directory: {}", e));
        }

        match conn.canonicalize(&relative_path_str).await {
            Ok(abs_path) => Ok(abs_path),
            Err(e) => Err(format!("Failed to canonicalize remote session path: {}", e)),
        }
    }

    /// Get the backup directory path relative to session store
    pub fn get_backup_dir_path() -> PathBuf {
        PathBuf::from("backups")
    }

    /// Get the full backup directory path for a given session ID
    pub fn get_backup_session_path(session_id: &str) -> PathBuf {
        Self::get_backup_dir_path().join(session_id)
    }

    /// Get the backup directory path as a string (for remote operations)
    pub fn get_backup_dir_string() -> String {
        Self::get_remote_session_store_path()
            .join(Self::get_backup_dir_path())
            .to_string_lossy()
            .to_string()
    }

    /// Get the full backup directory path as a string for a given session ID (for remote operations)
    pub fn get_backup_session_string(session_id: &str) -> String {
        Self::get_remote_session_store_path()
            .join(Self::get_backup_session_path(session_id))
            .to_string_lossy()
            .to_string()
    }

    /// Get the absolute backup session path on the remote host
    pub async fn get_absolute_backup_session_path(
        conn: &Arc<RemoteConnection>,
        session_id: &str,
    ) -> Result<String, String> {
        let relative_backup_path = Self::get_backup_session_string(session_id);

        if let Err(e) = conn
            .execute_command(&mkdir_command(&relative_backup_path), None, None)
            .await
        {
            return Err(format!("Failed to create remote backup directory: {}", e));
        }

        match conn.canonicalize(&relative_backup_path).await {
            Ok(abs_path) => Ok(abs_path),
            Err(e) => Err(format!("Failed to canonicalize remote backup path: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{mkdir_command, shell_single_quote_arg};

    #[test]
    fn shell_single_quote_arg_handles_empty_plain_and_embedded_quotes() {
        assert_eq!(shell_single_quote_arg(""), "''");
        assert_eq!(shell_single_quote_arg(".vac/session"), "'.vac/session'");
        assert_eq!(shell_single_quote_arg("a'b"), "'a'\\''b'");
    }

    #[test]
    fn mkdir_command_quotes_path_payload_as_single_argument() {
        assert_eq!(
            mkdir_command(".vac/session/a b;touch /tmp/pwn$(x)'z"),
            "mkdir -p '.vac/session/a b;touch /tmp/pwn$(x)'\\''z'"
        );
    }
}
