//! Request history management

use crate::models::{HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A stored request in history with metadata and the latest response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub request: HttpRequest,
    pub response: Option<HttpResponse>,
    pub error: Option<String>,
    pub timestamp: String,
    pub label: Option<String>,
}

/// Manages request history
pub struct RequestHistory {
    entries: VecDeque<HistoryEntry>,
    max_size: usize,
    next_id: u64,
}

impl RequestHistory {
    /// Creates a new history manager with default capacity
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Creates a new history manager with custom capacity
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_size),
            max_size,
            next_id: 1,
        }
    }

    /// Adds a request to history, optionally with its response and error.
    pub fn add(
        &mut self,
        request: HttpRequest,
        response: Option<HttpResponse>,
        error: Option<String>,
        label: Option<String>,
    ) {
        let entry = HistoryEntry {
            id: self.next_id,
            request,
            response,
            error,
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            label,
        };
        self.next_id += 1;

        self.entries.push_back(entry);

        // Remove oldest entry if we exceed max size
        if self.entries.len() > self.max_size {
            self.entries.pop_front();
        }
    }

    /// Gets all history entries
    pub fn get_all(&self) -> Vec<&HistoryEntry> {
        self.entries.iter().collect()
    }

    /// Gets the last N entries
    pub fn get_recent(&self, n: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().rev().take(n).collect()
    }

    /// Clears all history
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Removes a specific entry by id. Returns true if an entry was removed.
    pub fn remove_by_id(&mut self, id: u64) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    /// Returns the number of entries in history
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for RequestHistory {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::HttpMethod;

    #[test]
    fn test_history_add_and_retrieve() {
        let mut history = RequestHistory::new();
        let req = HttpRequest::new("https://api.example.com".to_string(), HttpMethod::Get);

        history.add(req, None, None, Some("Test Request".to_string()));
        assert_eq!(history.len(), 1);

        let entries = history.get_all();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].label, Some("Test Request".to_string()));
    }

    #[test]
    fn test_history_capacity() {
        let mut history = RequestHistory::with_capacity(3);

        for i in 0..5 {
            let req = HttpRequest::new(format!("https://api.example.com/{}", i), HttpMethod::Get);
            history.add(req, None, None, None);
        }

        assert_eq!(history.len(), 3); // Should only keep last 3
    }
}
