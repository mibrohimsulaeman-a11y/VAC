use std::collections::VecDeque;

use crate::redaction::redact_memory_text;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkingMemoryEntry {
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct WorkingMemory {
    max_entries: usize,
    entries: VecDeque<WorkingMemoryEntry>,
}

impl WorkingMemory {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: VecDeque::new(),
        }
    }

    pub fn push(&mut self, id: impl Into<String>, text: impl Into<String>) {
        if self.max_entries == 0 {
            return;
        }
        while self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(WorkingMemoryEntry {
            id: id.into(),
            text: redact_memory_text(text),
        });
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn entries(&self) -> impl Iterator<Item = &WorkingMemoryEntry> {
        self.entries.iter()
    }
}
