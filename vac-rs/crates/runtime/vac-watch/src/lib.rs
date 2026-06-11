//! File watch adapter boundary for producing VAC runtime jobs.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileWatchTrigger {
    pub glob: String,
    pub action: String,
}

impl FileWatchTrigger {
    #[must_use]
    pub fn describe(&self) -> String {
        format!("{} -> {}", self.glob, self.action)
    }
}
