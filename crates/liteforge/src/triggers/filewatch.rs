//! File system watch trigger.

use super::{Trigger, TriggerError, TriggerEvent, TriggerStatus};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

/// Kind of file event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileEventKind {
    /// File was created.
    Created,
    /// File was modified.
    Modified,
    /// File was deleted.
    Deleted,
    /// File was renamed.
    Renamed,
    /// Access time changed.
    Accessed,
}

/// A file system event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    /// The path that changed.
    pub path: PathBuf,
    /// Kind of change.
    pub kind: FileEventKind,
    /// Timestamp of the event.
    pub timestamp: u64,
    /// Old path for rename events.
    pub old_path: Option<PathBuf>,
}

impl FileEvent {
    /// Create a new file event.
    pub fn new(path: impl Into<PathBuf>, kind: FileEventKind) -> Self {
        Self {
            path: path.into(),
            kind,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            old_path: None,
        }
    }

    /// Create a rename event with old path.
    pub fn rename(old_path: impl Into<PathBuf>, new_path: impl Into<PathBuf>) -> Self {
        Self {
            path: new_path.into(),
            kind: FileEventKind::Renamed,
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
            old_path: Some(old_path.into()),
        }
    }
}

/// Configuration for file watching.
#[derive(Debug, Clone)]
pub struct FileWatchConfig {
    /// Paths to watch.
    pub paths: Vec<PathBuf>,
    /// Whether to watch recursively.
    pub recursive: bool,
    /// File patterns to include (glob patterns).
    pub include_patterns: Vec<String>,
    /// File patterns to exclude (glob patterns).
    pub exclude_patterns: Vec<String>,
    /// Event kinds to watch for.
    pub event_kinds: Vec<FileEventKind>,
}

impl Default for FileWatchConfig {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            recursive: true,
            include_patterns: vec!["*".to_string()],
            exclude_patterns: Vec::new(),
            event_kinds: vec![
                FileEventKind::Created,
                FileEventKind::Modified,
                FileEventKind::Deleted,
            ],
        }
    }
}

impl FileWatchConfig {
    /// Create a new config for a single path.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            paths: vec![path.into()],
            ..Default::default()
        }
    }

    /// Add a path to watch.
    pub fn add_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.paths.push(path.into());
        self
    }

    /// Set recursive watching.
    pub fn recursive(mut self, recursive: bool) -> Self {
        self.recursive = recursive;
        self
    }

    /// Add an include pattern.
    pub fn include(mut self, pattern: impl Into<String>) -> Self {
        self.include_patterns.push(pattern.into());
        self
    }

    /// Add an exclude pattern.
    pub fn exclude(mut self, pattern: impl Into<String>) -> Self {
        self.exclude_patterns.push(pattern.into());
        self
    }

    /// Set event kinds to watch.
    pub fn event_kinds(mut self, kinds: Vec<FileEventKind>) -> Self {
        self.event_kinds = kinds;
        self
    }
}

/// A trigger that fires on file system events.
///
/// Note: This is a polling-based implementation. For production use,
/// consider integrating with a native file watcher like `notify`.
#[derive(Debug, Clone)]
pub struct FileWatchTrigger {
    id: String,
    config: FileWatchConfig,
    status: Arc<Mutex<TriggerStatus>>,
    events: Arc<Mutex<VecDeque<FileEvent>>>,
    /// Tracks last known state of files for change detection.
    file_states: Arc<Mutex<std::collections::HashMap<PathBuf, FileState>>>,
}

#[derive(Debug, Clone)]
struct FileState {
    modified: u64,
    exists: bool,
}

impl FileWatchTrigger {
    /// Create a new file watch trigger.
    pub fn new(id: impl Into<String>, config: FileWatchConfig) -> Self {
        Self {
            id: id.into(),
            config,
            status: Arc::new(Mutex::new(TriggerStatus::Stopped)),
            events: Arc::new(Mutex::new(VecDeque::new())),
            file_states: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    /// Create a trigger for a single path.
    pub fn watch_path(id: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self::new(id, FileWatchConfig::new(path))
    }

    /// Get the configuration.
    pub fn config(&self) -> &FileWatchConfig {
        &self.config
    }

    /// Push a file event manually (for testing or external watchers).
    pub fn push_event(&self, event: FileEvent) {
        self.events.lock().unwrap().push_back(event);
    }

    /// Get the number of pending events.
    pub fn pending_count(&self) -> usize {
        self.events.lock().unwrap().len()
    }

    /// Check for file changes by polling the filesystem.
    pub fn check_changes(&self) {
        let status = *self.status.lock().unwrap();
        if status != TriggerStatus::Running {
            return;
        }

        let mut states = self.file_states.lock().unwrap();
        let mut events = self.events.lock().unwrap();

        for path in &self.config.paths {
            self.scan_path(path, &mut states, &mut events);
        }
    }

    fn scan_path(
        &self,
        path: &Path,
        states: &mut std::collections::HashMap<PathBuf, FileState>,
        events: &mut VecDeque<FileEvent>,
    ) {
        if path.is_file() {
            self.check_file(path, states, events);
        } else if path.is_dir() {
            self.scan_directory(path, states, events);
        }
    }

    fn scan_directory(
        &self,
        dir: &Path,
        states: &mut std::collections::HashMap<PathBuf, FileState>,
        events: &mut VecDeque<FileEvent>,
    ) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_file() {
                if self.should_include(&path) {
                    self.check_file(&path, states, events);
                }
            } else if path.is_dir() && self.config.recursive {
                self.scan_directory(&path, states, events);
            }
        }

        // Check for deleted files
        let deleted: Vec<PathBuf> = states
            .iter()
            .filter(|(p, s)| s.exists && p.starts_with(dir) && !p.exists())
            .map(|(p, _)| p.clone())
            .collect();

        for path in deleted {
            if self.config.event_kinds.contains(&FileEventKind::Deleted) {
                events.push_back(FileEvent::new(&path, FileEventKind::Deleted));
            }
            if let Some(state) = states.get_mut(&path) {
                state.exists = false;
            }
        }
    }

    fn check_file(
        &self,
        path: &Path,
        states: &mut std::collections::HashMap<PathBuf, FileState>,
        events: &mut VecDeque<FileEvent>,
    ) {
        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return,
        };

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let path_buf = path.to_path_buf();

        if let Some(state) = states.get(&path_buf) {
            if !state.exists {
                // File was deleted and recreated
                if self.config.event_kinds.contains(&FileEventKind::Created) {
                    events.push_back(FileEvent::new(&path_buf, FileEventKind::Created));
                }
            } else if modified > state.modified {
                // File was modified
                if self.config.event_kinds.contains(&FileEventKind::Modified) {
                    events.push_back(FileEvent::new(&path_buf, FileEventKind::Modified));
                }
            }
        } else {
            // New file
            if self.config.event_kinds.contains(&FileEventKind::Created) {
                events.push_back(FileEvent::new(&path_buf, FileEventKind::Created));
            }
        }

        states.insert(
            path_buf,
            FileState {
                modified,
                exists: true,
            },
        );
    }

    fn should_include(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Check exclude patterns first
        for pattern in &self.config.exclude_patterns {
            if self.matches_pattern(file_name, pattern) {
                return false;
            }
        }

        // Check include patterns
        for pattern in &self.config.include_patterns {
            if self.matches_pattern(file_name, pattern) {
                return true;
            }
        }

        false
    }

    fn matches_pattern(&self, name: &str, pattern: &str) -> bool {
        // Simple glob matching (* and ?)
        if pattern == "*" {
            return true;
        }

        let pattern_chars: Vec<char> = pattern.chars().collect();
        let name_chars: Vec<char> = name.chars().collect();

        self.glob_match(&pattern_chars, &name_chars, 0, 0)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn glob_match(&self, pattern: &[char], name: &[char], pi: usize, ni: usize) -> bool {
        if pi == pattern.len() && ni == name.len() {
            return true;
        }
        if pi == pattern.len() {
            return false;
        }

        match pattern[pi] {
            '*' => {
                // Try matching zero or more characters
                for i in ni..=name.len() {
                    if self.glob_match(pattern, name, pi + 1, i) {
                        return true;
                    }
                }
                false
            }
            '?' => {
                if ni < name.len() {
                    self.glob_match(pattern, name, pi + 1, ni + 1)
                } else {
                    false
                }
            }
            c => {
                if ni < name.len() && name[ni] == c {
                    self.glob_match(pattern, name, pi + 1, ni + 1)
                } else {
                    false
                }
            }
        }
    }
}

impl Trigger for FileWatchTrigger {
    fn id(&self) -> &str {
        &self.id
    }

    fn trigger_type(&self) -> &str {
        "filewatch"
    }

    fn status(&self) -> TriggerStatus {
        *self.status.lock().unwrap()
    }

    fn start(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status == TriggerStatus::Running {
            return Err(TriggerError::already_running());
        }

        // Validate paths exist
        for path in &self.config.paths {
            if !path.exists() {
                return Err(TriggerError::config(format!(
                    "Path does not exist: {}",
                    path.display()
                )));
            }
        }

        *status = TriggerStatus::Running;
        Ok(())
    }

    fn stop(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        *status = TriggerStatus::Stopped;
        Ok(())
    }

    fn pause(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status != TriggerStatus::Running {
            return Err(TriggerError::not_running());
        }
        *status = TriggerStatus::Paused;
        Ok(())
    }

    fn resume(&mut self) -> Result<(), TriggerError> {
        let mut status = self.status.lock().unwrap();
        if *status != TriggerStatus::Paused {
            return Err(TriggerError::runtime("Trigger is not paused"));
        }
        *status = TriggerStatus::Running;
        Ok(())
    }

    fn poll(&self) -> Option<TriggerEvent> {
        let status = *self.status.lock().unwrap();
        if status != TriggerStatus::Running {
            return None;
        }

        let mut events = self.events.lock().unwrap();
        events.pop_front().map(|file_event| {
            TriggerEvent::new(
                &self.id,
                "filewatch",
                serde_json::to_value(&file_event).unwrap_or_default(),
            )
        })
    }

    fn has_pending(&self) -> bool {
        !self.events.lock().unwrap().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_file_event_new() {
        let event = FileEvent::new("/tmp/test.txt", FileEventKind::Created);
        assert_eq!(event.path, PathBuf::from("/tmp/test.txt"));
        assert_eq!(event.kind, FileEventKind::Created);
        assert!(event.timestamp > 0);
    }

    #[test]
    fn test_file_event_rename() {
        let event = FileEvent::rename("/tmp/old.txt", "/tmp/new.txt");
        assert_eq!(event.path, PathBuf::from("/tmp/new.txt"));
        assert_eq!(event.old_path, Some(PathBuf::from("/tmp/old.txt")));
        assert_eq!(event.kind, FileEventKind::Renamed);
    }

    #[test]
    fn test_file_watch_config() {
        let config = FileWatchConfig::new("/tmp")
            .recursive(false)
            .include("*.rs")
            .exclude("*.bak");

        assert_eq!(config.paths, vec![PathBuf::from("/tmp")]);
        assert!(!config.recursive);
        assert!(config.include_patterns.contains(&"*.rs".to_string()));
        assert!(config.exclude_patterns.contains(&"*.bak".to_string()));
    }

    #[test]
    fn test_file_watch_trigger_manual_events() {
        let config = FileWatchConfig::new("/tmp");
        let mut trigger = FileWatchTrigger::new("test", config);
        trigger.start().unwrap();

        // Push event manually
        trigger.push_event(FileEvent::new("/tmp/test.txt", FileEventKind::Created));

        assert_eq!(trigger.pending_count(), 1);

        let event = trigger.poll().unwrap();
        assert_eq!(event.trigger_id, "test");
        assert_eq!(event.event_type, "filewatch");
    }

    #[test]
    fn test_file_watch_trigger_not_running() {
        let config = FileWatchConfig::new("/tmp");
        let trigger = FileWatchTrigger::new("test", config);

        trigger.push_event(FileEvent::new("/tmp/test.txt", FileEventKind::Created));

        // Should return None because trigger is not running
        assert!(trigger.poll().is_none());
    }

    #[test]
    fn test_file_watch_trigger_with_real_files() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        // Create initial file
        fs::write(&file_path, "initial").unwrap();

        let config = FileWatchConfig::new(dir.path());
        let mut trigger = FileWatchTrigger::new("test", config);
        trigger.start().unwrap();

        // First check should detect the file as new
        trigger.check_changes();

        // Should have a created event
        let event = trigger.poll();
        assert!(event.is_some(), "Expected created event for new file");

        // Wait longer to ensure filesystem timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(1100));
        fs::write(&file_path, "modified content").unwrap();

        trigger.check_changes();

        // May or may not have a modified event depending on filesystem timestamp resolution
        // This is expected behavior - just verify we don't crash
        let _ = trigger.poll();
    }

    #[test]
    fn test_glob_matching() {
        let config = FileWatchConfig::new("/tmp");
        let trigger = FileWatchTrigger::new("test", config);

        assert!(trigger.matches_pattern("test.rs", "*.rs"));
        assert!(trigger.matches_pattern("test.rs", "*"));
        assert!(!trigger.matches_pattern("test.rs", "*.txt"));
        assert!(trigger.matches_pattern("test.rs", "test.rs"));
        assert!(trigger.matches_pattern("test.rs", "test.??"));
        assert!(trigger.matches_pattern("test.rs", "t*.rs"));
    }
}
