use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use tokio::io::{self, AsyncWriteExt};
use log;
use once_cell::sync::Lazy;

pub struct SessionChannels {
    pub command: Arc<TokioMutex<io::WriteHalf<io::SimplexStream>>>,
    pub notification: Arc<TokioMutex<io::WriteHalf<io::SimplexStream>>>,
}

#[derive(Default)]
pub struct SSESessionManager {
    pub(crate) sessions: TokioMutex<HashMap<String, SessionChannels>>,
}

impl SSESessionManager {
    pub fn new() -> Self {
        Self {
            sessions: TokioMutex::new(HashMap::new()),
        }
    }

    pub async fn register_session(
        &self, 
        session_id: String, 
        command_writer: Arc<TokioMutex<io::WriteHalf<io::SimplexStream>>>,
        notification_writer: Arc<TokioMutex<io::WriteHalf<io::SimplexStream>>>,
    ) {
        let mut sessions = self.sessions.lock().await;
        sessions.insert(session_id, SessionChannels {
            command: command_writer,
            notification: notification_writer,
        });
    }

    pub async fn remove_session(&self, session_id: &str) {
        let mut sessions = self.sessions.lock().await;
        sessions.remove(session_id);
    }

    pub async fn broadcast_message(&self, message: &str) -> Vec<String> {
        let mut failed_sessions = Vec::new();
        let sessions = self.sessions.lock().await;
        
        for (session_id, channels) in sessions.iter() {
            let mut writer = channels.notification.lock().await;
            let message_bytes = message.as_bytes();
            
            if let Err(e) = async {
                writer.write_all(message_bytes).await?;
                writer.write_u8(b'\n').await?;
                writer.flush().await?;
                Ok::<_, std::io::Error>(())
            }.await {
                log::error!("Failed to broadcast to session {}: {}", session_id, e);
                failed_sessions.push(session_id.clone());
            }
        }
        
        failed_sessions
    }

    pub async fn send_message(&self, session_id: &str, message: &str) -> Result<(), String> {
        let sessions = self.sessions.lock().await;
        if let Some(channels) = sessions.get(session_id) {
            let mut writer = channels.command.lock().await;
            let message_bytes = message.as_bytes();
            
            async {
                writer.write_all(message_bytes).await?;
                writer.write_u8(b'\n').await?;
                writer.flush().await?;
                Ok::<_, std::io::Error>(())
            }.await.map_err(|e| format!("Failed to send message to session {}: {}", session_id, e))
        } else {
            Err(format!("Session {} not found", session_id))
        }
    }
}

// Global session manager instance
pub static SESSION_MANAGER: Lazy<SSESessionManager> = Lazy::new(|| {
    SSESessionManager::new()
});
