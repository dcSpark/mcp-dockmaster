use std::collections::HashMap;
use tokio::process::Child;

/// ProcessManager: tracks running Child processes
///
/// This module is responsible for storing and managing references to running
/// Child processes and their stdin/stdout handles. It maintains:
/// - A map of tool IDs to their Child processes
/// - A map of tool IDs to their stdin/stdout handles for communication
///
/// It has no knowledge about the database or MCPState, focusing solely on
/// process tracking.
#[derive(Default)]
pub struct ProcessManager {
    pub processes: HashMap<String, Option<Child>>,
    pub process_ios: HashMap<String, (tokio::process::ChildStdin, tokio::process::ChildStdout)>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self::default()
    }
}
