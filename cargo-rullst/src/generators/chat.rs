use crate::generators::is_rullst_project;
use std::fs;
use std::path::Path;

pub fn scaffold_chat_session() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        return Err("This command must be run inside a Rullst project".into());
    }

    let model_path = Path::new("src/models/chat.rs");

    // Create the models
    let content = r#"use rullst::orm::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Orm)]
#[orm(table = "chat_sessions")]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Orm)]
#[orm(table = "chat_messages")]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub role: String, // system, user, assistant
    pub content: String,
    pub created_at: i64,
}

impl ChatSession {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            id: rullst::utils::uuid(),
            title: title.into(),
            created_at: rullst::utils::now(),
        }
    }
    
    /// Returns the history for this session, ordered by time.
    pub async fn history(&self) -> Result<Vec<ChatMessage>, rullst::orm::Error> {
        ChatMessage::query()
            .where_eq("session_id", &self.id)
            .order_by("created_at", "ASC")
            .all()
            .await
    }
}
"#;

    fs::create_dir_all("src/models")?;
    fs::write(model_path, content)?;

    let service_path = Path::new("src/ai/chat_service.rs");
    let service_content = r#"use crate::models::chat::{ChatSession, ChatMessage};
use rullst::ai::{AiClient, ChatBuilder};

pub struct StatefulChat {
    session: ChatSession,
    client: AiClient,
}

impl StatefulChat {
    pub fn new(session: ChatSession, client: AiClient) -> Self {
        Self { session, client }
    }

    pub async fn send(&self, text: &str) -> Result<String, rullst::ai::AiError> {
        let history = self.session.history().await.map_err(|_| rullst::ai::AiError::Other("DB error".into()))?;
        
        let mut builder = self.client.chat();
        for msg in history {
            match msg.role.as_str() {
                "system" => builder = builder.system(msg.content),
                "user" => builder = builder.user(msg.content),
                "assistant" => builder = builder.assistant(msg.content),
                _ => {}
            }
        }
        
        builder = builder.user(text);
        
        // Save user message
        let mut user_msg = ChatMessage {
            id: rullst::utils::uuid(),
            session_id: self.session.id.clone(),
            role: "user".to_string(),
            content: text.to_string(),
            created_at: rullst::utils::now(),
        };
        let _ = user_msg.save().await;
        
        let response = builder.send().await?;
        
        // Save assistant message
        let mut asst_msg = ChatMessage {
            id: rullst::utils::uuid(),
            session_id: self.session.id.clone(),
            role: "assistant".to_string(),
            content: response.clone(),
            created_at: rullst::utils::now(),
        };
        let _ = asst_msg.save().await;

        Ok(response)
    }
}
"#;

    fs::create_dir_all("src/ai")?;
    fs::write(service_path, service_content)?;

    println!("✅ ChatSession and ChatMessage models successfully scaffolded!");
    println!("👉 Check out src/models/chat.rs and src/ai/chat_service.rs");

    Ok(())
}
