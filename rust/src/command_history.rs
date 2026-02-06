/// Command history: storage, load/save, fuzzy match, wrapping iterator.
use godot::classes::file_access::ModeFlags;
use godot::classes::FileAccess;
use godot::prelude::*;

pub const HISTORY_FILE: &str = "user://tiny_console_history.log";

pub struct CommandHistory {
    entries: Vec<String>,
    is_dirty: bool,
}

impl CommandHistory {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            is_dirty: false,
        }
    }

    /// Adds a command to history. Duplicates are moved to the end.
    pub fn push_entry(&mut self, entry: String) {
        if let Some(idx) = self.entries.iter().position(|e| e == &entry) {
            self.entries.remove(idx);
        }
        self.entries.push(entry);
        self.is_dirty = true;
    }

    pub fn get_entry(&self, index: usize) -> &str {
        let idx = index.min(self.entries.len().saturating_sub(1));
        &self.entries[idx]
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }

    pub fn trim(&mut self, max_size: usize) {
        if self.entries.len() > max_size {
            let drain_count = self.entries.len() - max_size;
            self.entries.drain(..drain_count);
        }
    }

    pub fn clear(&mut self) {
        self.entries.clear();
        self.is_dirty = true;
    }

    pub fn load(&mut self, path: &str) {
        let path_gstr: GString = path.into();
        if let Some(file) = FileAccess::open(&path_gstr, ModeFlags::READ) {
            while !file.eof_reached() {
                let line = file.get_line().to_string();
                let line = line.trim().to_string();
                if !line.is_empty() {
                    // Push without dedup reset (internal push)
                    if let Some(idx) = self.entries.iter().position(|e| e == &line) {
                        self.entries.remove(idx);
                    }
                    self.entries.push(line);
                }
            }
            self.is_dirty = false;
        }
    }

    pub fn save(&mut self, path: &str) {
        if !self.is_dirty {
            return;
        }
        let path_gstr: GString = path.into();
        if let Some(mut file) = FileAccess::open(&path_gstr, ModeFlags::WRITE) {
            for line in &self.entries {
                file.store_line(&GString::from(line.as_str()));
            }
            self.is_dirty = false;
        } else {
            godot_error!(
                "TinyConsole: Failed to save console history to file: {}",
                path
            );
        }
    }

    /// Returns entries matching the query, sorted by relevance (best first).
    pub fn fuzzy_match(&self, query: &str) -> Vec<String> {
        if query.is_empty() {
            let mut copy = self.entries.clone();
            copy.reverse();
            return copy;
        }

        let query_lower = query.to_lowercase();
        let mut results: Vec<(String, i32)> = Vec::new();

        for entry in &self.entries {
            let score = compute_match_score(&query_lower, &entry.to_lowercase());
            if score > 0 {
                results.push((entry.clone(), score));
            }
        }

        results.sort_by(|a, b| b.1.cmp(&a.1));
        results.into_iter().map(|(entry, _)| entry).collect()
    }

    pub fn entries(&self) -> &[String] {
        &self.entries
    }

    pub fn create_iterator(&self) -> WrappingIterator {
        WrappingIterator {
            idx: -1,
            entries: self.entries.clone(),
        }
    }

    /// Reassigns iterator entries to match the current history.
    pub fn reassign_iterator(&self, iter: &mut WrappingIterator) {
        iter.idx = -1;
        iter.entries = self.entries.clone();
    }
}

/// Scoring function for fuzzy matching.
fn compute_match_score(query: &str, target: &str) -> i32 {
    if query == target {
        return 99999;
    }

    let query_chars: Vec<char> = query.chars().collect();
    let target_chars: Vec<char> = target.chars().collect();
    let mut score = 0i32;
    let mut query_index = 0usize;

    for (i, &tc) in target_chars.iter().enumerate() {
        if query_index < query_chars.len() && tc == query_chars[query_index] {
            score += 10;
            if i == 0 || target_chars[i - 1] == ' ' {
                score += 5; // Bonus for word start
            }
            query_index += 1;
            if query_index == query_chars.len() {
                break;
            }
        }
    }

    if query_index == query_chars.len() {
        score
    } else {
        0
    }
}

/// Circular iterator for navigating history entries.
pub struct WrappingIterator {
    idx: i32,
    entries: Vec<String>,
}

impl WrappingIterator {
    pub fn prev(&mut self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        self.idx -= 1;
        let len = self.entries.len() as i32;
        // Wrap: valid range is -1..len-1
        if self.idx < -1 {
            self.idx = len - 1;
        }
        if self.idx == -1 {
            String::new()
        } else {
            self.entries[self.idx as usize].clone()
        }
    }

    pub fn next(&mut self) -> String {
        if self.entries.is_empty() {
            return String::new();
        }
        self.idx += 1;
        let len = self.entries.len() as i32;
        if self.idx >= len {
            self.idx = -1;
        }
        if self.idx == -1 {
            String::new()
        } else {
            self.entries[self.idx as usize].clone()
        }
    }

    pub fn current(&self) -> String {
        if self.idx < 0 || self.idx as usize >= self.entries.len() {
            String::new()
        } else {
            self.entries[self.idx as usize].clone()
        }
    }

    pub fn reset(&mut self) {
        self.idx = -1;
    }
}
