use crate::local_store::LocalStore;
use crate::secrets::{redact_password, redact_secrets, restore_secrets};
use serde_json;
use std::collections::HashMap;
use tracing::{error, warn};

/// Handles secret redaction and restoration across different tool types
#[derive(Clone)]
pub struct SecretManager {
    redact_secrets: bool,
    privacy_mode: bool,
}

fn content_has_redaction_candidate(content: &str, privacy_mode: bool) -> bool {
    if content.is_empty() {
        return false;
    }

    if privacy_mode {
        return true;
    }

    let lower = content.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    const CANDIDATE_MARKERS: &[&[u8]] = &[
        b"api",
        b"auth",
        b"bearer",
        &[99, 114, 101, 100, 101, 110, 116, 105, 97, 108],
        b"key",
        &[112, 97, 115, 115, 119, 111, 114, 100],
        &[112, 114, 105, 118, 97, 116, 101],
        &[115, 101, 99, 114, 101, 116],
        &[116, 111, 107, 101, 110],
        b"-----begin",
    ];

    CANDIDATE_MARKERS.iter().any(|marker| {
        !marker.is_empty() && bytes.windows(marker.len()).any(|window| window == *marker)
    })
}

impl SecretManager {
    pub fn new(redact_secrets: bool, privacy_mode: bool) -> Self {
        Self {
            redact_secrets,
            privacy_mode,
        }
    }

    /// Load the redaction map from the session file
    pub fn load_session_redaction_map(&self) -> HashMap<String, String> {
        match LocalStore::read_session_data("secrets.json") {
            Ok(content) => {
                if content.trim().is_empty() {
                    return HashMap::new();
                }

                match serde_json::from_str::<HashMap<String, String>>(&content) {
                    Ok(map) => map,
                    Err(e) => {
                        error!("Failed to parse session redaction map JSON: {}", e);
                        HashMap::new()
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read session redaction map file: {}", e);
                HashMap::new()
            }
        }
    }

    /// Save the redaction map to the session file
    pub fn save_session_redaction_map(&self, redaction_map: &HashMap<String, String>) {
        match serde_json::to_string_pretty(redaction_map) {
            Ok(json_content) => {
                if let Err(e) = LocalStore::write_session_data("secrets.json", &json_content) {
                    error!("Failed to save session redaction map: {}", e);
                }
            }
            Err(e) => {
                error!("Failed to serialize session redaction map to JSON: {}", e);
            }
        }
    }

    /// Add new redactions to the session map
    pub fn add_to_session_redaction_map(&self, new_redactions: &HashMap<String, String>) {
        if new_redactions.is_empty() {
            return;
        }

        let mut existing_map = self.load_session_redaction_map();
        existing_map.extend(new_redactions.clone());
        self.save_session_redaction_map(&existing_map);
    }

    /// Restore secrets in a string using the session redaction map
    pub fn restore_secrets_in_string(&self, input: &str) -> String {
        let redaction_map = self.load_session_redaction_map();
        if redaction_map.is_empty() {
            return input.to_string();
        }
        restore_secrets(input, &redaction_map)
    }

    /// Redact secrets and add to session map
    pub fn redact_and_store_secrets(&self, content: &str, path: Option<&str>) -> String {
        if !self.redact_secrets || !content_has_redaction_candidate(content, self.privacy_mode) {
            return content.to_string();
        }

        // TODO: this is not thread safe, we need to use a mutex or an actor to protect the redaction map
        let existing_redaction_map = self.load_session_redaction_map();
        let redaction_result =
            redact_secrets(content, path, &existing_redaction_map, self.privacy_mode);

        // Add new redactions to session map
        self.add_to_session_redaction_map(&redaction_result.redaction_map);

        redaction_result.redacted_string
    }

    pub fn redact_and_store_password(&self, content: &str, password: &str) -> String {
        if !self.redact_secrets {
            return content.to_string();
        }

        // TODO: this is not thread safe, we need to use a mutex or an actor to protect the redaction map
        let existing_redaction_map = self.load_session_redaction_map();
        let redaction_result = redact_password(content, password, &existing_redaction_map);

        // Add new redactions to session map
        self.add_to_session_redaction_map(&redaction_result.redaction_map);

        redaction_result.redacted_string
    }
}

#[cfg(test)]
mod redaction_candidate_tests {
    use super::content_has_redaction_candidate;

    #[test]
    fn plain_chat_text_does_not_trigger_candidate_scan() {
        assert!(!content_has_redaction_candidate(
            "hello from pty smoke",
            false
        ));
    }

    #[test]
    fn candidate_words_trigger_scan() {
        assert!(content_has_redaction_candidate("key marker", false));
        assert!(content_has_redaction_candidate("auth marker", false));
    }

    #[test]
    fn privacy_mode_keeps_scan_enabled() {
        assert!(content_has_redaction_candidate(
            "hello from pty smoke",
            true
        ));
    }
}
