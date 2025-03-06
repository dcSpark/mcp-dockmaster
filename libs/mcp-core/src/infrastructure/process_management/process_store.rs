use std::collections::HashMap;
use std::sync::Arc;
use tokio::process::{Child, ChildStdin, ChildStdout};
use tokio::sync::RwLock;
use log::{info, error};

#[derive(Default)]
pub struct ProcessStore {
    // For each tool_id, store the Child
    processes: RwLock<HashMap<String, Child>>,
    // For each tool_id, store the (stdin, stdout) pipes
    process_ios: RwLock<HashMap<String, (ChildStdin, ChildStdout)>>,
}

impl ProcessStore {
    pub fn new() -> Self {
        Self {
            processes: RwLock::new(HashMap::new()),
            process_ios: RwLock::new(HashMap::new()),
        }
    }

    /// Insert a new process (and its I/O) for a given tool_id
    pub async fn insert_process(
        &self,
        tool_id: String,
        child: Child,
        io: (ChildStdin, ChildStdout),
    ) {
        {
            let mut proc_map = self.processes.write().await;
            proc_map.insert(tool_id.clone(), child);
        }
        {
            let mut io_map = self.process_ios.write().await;
            io_map.insert(tool_id, io);
        }
    }

    /// Remove a process from the store (returns the old Child/IO if it existed)
    pub async fn remove_process(&self, tool_id: &str) -> Option<(Child, (ChildStdin, ChildStdout))> {
        let mut proc_map = self.processes.write().await;
        let mut io_map = self.process_ios.write().await;

        let child_opt = proc_map.remove(tool_id);
        let io_opt = io_map.remove(tool_id);
        if let (Some(child), Some(io)) = (child_opt, io_opt) {
            Some((child, io))
        } else {
            None
        }
    }

    /// Kill all running processes
    pub async fn kill_all_processes(&self) {
        let mut proc_map = self.processes.write().await;
        for (tool_id, mut child) in proc_map.drain() {
            match child.kill().await {
                Ok(_) => info!("Killed process for tool {}", tool_id),
                Err(e) => error!("Failed to kill process for tool {}: {}", tool_id, e),
            }
        }

        // Clear out the I/O map so we don't hang onto old handles
        let mut io_map = self.process_ios.write().await;
        io_map.clear();
    }

    /// Kill a single process by tool_id
    pub async fn kill_process(&self, tool_id: &str) {
        let mut proc_map = self.processes.write().await;
        if let Some(mut child) = proc_map.remove(tool_id) {
            match child.kill().await {
                Ok(_) => info!("Killed process for tool {}", tool_id),
                Err(e) => error!("Failed to kill process for tool {}: {}", tool_id, e),
            }
        }
        let mut io_map = self.process_ios.write().await;
        io_map.remove(tool_id);
    }

    /// Get a reference to (stdin, stdout) if you need to read/write
    pub async fn get_io(
        &self, 
        tool_id: &str
    ) -> Option<(ChildStdin, ChildStdout)> {
        let mut io_map = self.process_ios.write().await;
        io_map.remove(tool_id)
    }
    
    /// Get a mutable reference to the process_ios HashMap
    pub async fn get_process_ios_mut(&self) -> tokio::sync::RwLockWriteGuard<'_, HashMap<String, (ChildStdin, ChildStdout)>> {
        self.process_ios.write().await
    }
    
    /// Check if a process is running
    pub async fn is_process_running(&self, tool_id: &str) -> bool {
        let proc_map = self.processes.read().await;
        proc_map.contains_key(tool_id)
    }
}
