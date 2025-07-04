//! Codex Vault Next-Gen Tauri Application
//!
//! This is the main Tauri application that provides the desktop interface
//! for the Codex Vault offline AI-powered knowledge repository.

use std::sync::Arc;
use tauri::{Manager, State};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use anyhow;

use codex_core::{CodexCore, CodexResult, CodexError};

/// Application state containing the core library instance
pub struct AppState {
    pub core: Arc<RwLock<Option<CodexCore>>>,
}

/// Response wrapper for Tauri commands
#[derive(Debug, Serialize)]
pub struct CommandResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

/// AI response structure matching frontend expectations
#[derive(Debug, Serialize)]
pub struct AiResponse {
    pub content: String,
    pub model: String,
    pub processing_time_ms: u64,
    pub tokens_used: u32,
}

/// System metrics response structure
#[derive(Debug, Serialize)]
pub struct SystemMetricsResponse {
    pub cpu_usage: f64,
    pub memory_usage_mb: f64,
    pub total_memory_mb: f64,
    pub ai_model_loaded: bool,
    pub uptime_seconds: u64,
}

/// Health check response structure
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub core_initialized: bool,
    pub ai_available: bool,
    pub database_connected: bool,
}

impl<T> CommandResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
        }
    }
}

impl<T> From<CodexResult<T>> for CommandResponse<T> {
    fn from(result: CodexResult<T>) -> Self {
        match result {
            Ok(data) => Self::success(data),
            Err(e) => Self::error(e.to_string()),
        }
    }
}

/// Document data transfer object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentDto {
    pub id: String,
    pub title: String,
    pub content: String,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub category: Option<String>,
    pub tags: Vec<String>,
    pub language: String,
    pub reading_time: Option<i32>,
    pub difficulty_level: Option<i32>,
    pub created_at: String,
    pub updated_at: String,
    pub view_count: i64,
    pub is_favorite: bool,
}

/// Search options for frontend
#[derive(Debug, Clone, Deserialize)]
pub struct SearchOptionsDto {
    pub search_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
}

/// Search result for frontend
#[derive(Debug, Clone, Serialize)]
pub struct SearchResultDto {
    pub documents: Vec<DocumentDto>,
    pub total_count: usize,
    pub query: String,
    pub search_time_ms: u64,
    pub has_more: bool,
}

// =====================================================
// CORE MANAGEMENT COMMANDS
// =====================================================

/// Initialize the core library
#[tauri::command]
async fn initialize_core(state: State<'_, AppState>) -> Result<CommandResponse<bool>, tauri::Error> {
    tracing::info!("Initializing Codex Core library");
    
    let core = match CodexCore::new().await {
        Ok(core) => core,
        Err(e) => {
            tracing::error!("Failed to initialize core: {}", e);
            return Ok(CommandResponse::error(format!("Failed to initialize core: {}", e)));
        }
    };

    let mut core_lock = state.core.write().await;
    *core_lock = Some(core);
    
    tracing::info!("Codex Core library initialized successfully");
    Ok(CommandResponse::success(true))
}

/// Get core library health status
#[tauri::command]
async fn get_health_status(state: State<'_, AppState>) -> Result<CommandResponse<bool>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let health = core.health_check().await;
        Ok(CommandResponse::from(health.map(|h| h.overall)))
    } else {
        Ok(CommandResponse::error("Core not initialized".to_string()))
    }
}

// =====================================================
// DOCUMENT MANAGEMENT COMMANDS
// =====================================================

/// Import a document from file path
#[tauri::command]
async fn import_document(
    file_path: String,
    state: State<'_, AppState>,
) -> Result<CommandResponse<String>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let result = core.content.import_document(&file_path).await;
        Ok(CommandResponse::from(result.map(|id| id.to_string())))
    } else {
        Ok(CommandResponse::error("Core not initialized".to_string()))
    }
}

/// Import text content
#[tauri::command]
async fn import_text_content(
    title: String,
    content: String,
    content_type: Option<String>,
    state: State<'_, AppState>,
) -> Result<CommandResponse<String>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let result = core.content.import_text_content(title, content, content_type).await;
        Ok(CommandResponse::from(result.map(|id| id.to_string())))
    } else {
        Ok(CommandResponse::error("Core not initialized".to_string()))
    }
}

/// Get document by ID
#[tauri::command]
async fn get_document(
    document_id: String,
    state: State<'_, AppState>,
) -> Result<CommandResponse<DocumentDto>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let id = match Uuid::parse_str(&document_id) {
            Ok(id) => id,
            Err(_) => return Ok(CommandResponse::error("Invalid document ID".to_string())),
        };

        let result = core.content.get_document(id).await;
        match result {
            Ok(Some(doc)) => Ok(CommandResponse::success(document_to_dto(&doc))),
            Ok(None) => Ok(CommandResponse::error("Document not found".to_string())),
            Err(e) => Ok(CommandResponse::error(e.to_string())),
        }
    } else {
        Ok(CommandResponse::error("Core not initialized".to_string()))
    }
}

/// Get recent documents
#[tauri::command]
async fn get_recent_documents(
    limit: i64,
    state: State<'_, AppState>,
) -> Result<CommandResponse<Vec<DocumentDto>>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let result = core.content.get_recent_documents(limit).await;
        Ok(CommandResponse::from(result.map(|docs| {
            docs.into_iter().map(|doc| document_to_dto(&doc)).collect()
        })))
    } else {
        Ok(CommandResponse::error("Core not initialized".to_string()))
    }
}

/// Search documents
#[tauri::command]
async fn search_documents(
    query: String,
    options: SearchOptionsDto,
    state: State<'_, AppState>,
) -> Result<CommandResponse<SearchResultDto>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let search_options = dto_to_search_options(options);
        let result = core.content.search_documents(&query, search_options).await;
        
        match result {
            Ok(search_results) => {
                let dto = SearchResultDto {
                    documents: search_results.documents.into_iter()
                        .map(|sr| document_to_dto(&sr.document))
                        .collect(),
                    total_count: search_results.total_count,
                    query: search_results.query,
                    search_time_ms: search_results.search_time_ms,
                    has_more: search_results.has_more,
                };
                Ok(CommandResponse::success(dto))
            }
            Err(e) => Ok(CommandResponse::error(e.to_string())),
        }
    } else {
        Ok(CommandResponse::error("Core not initialized".to_string()))
    }
}

/// Toggle document favorite status
#[tauri::command]
async fn toggle_favorite(
    document_id: String,
    state: State<'_, AppState>,
) -> Result<CommandResponse<bool>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let id = match Uuid::parse_str(&document_id) {
            Ok(id) => id,
            Err(_) => return Ok(CommandResponse::error("Invalid document ID".to_string())),
        };

        let result = core.content.toggle_favorite(id).await;
        Ok(CommandResponse::from(result))
    } else {
        Ok(CommandResponse::error("Core not initialized".to_string()))
    }
}

// =====================================================
// SYSTEM COMMANDS
// =====================================================

/// Get system metrics (CPU, memory, AI status)
#[tauri::command]
async fn get_system_metrics(
    state: State<'_, AppState>,
) -> Result<SystemMetricsResponse, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        match core.ai.get_system_metrics().await {
            Ok(metrics) => {
                Ok(SystemMetricsResponse {
                    cpu_usage: metrics.system_cpu_usage,
                    memory_usage_mb: metrics.system_memory_usage_mb,
                    total_memory_mb: metrics.total_memory_mb,
                    ai_model_loaded: true, // If we got metrics, model is loaded
                    uptime_seconds: metrics.uptime_seconds,
                })
            },
            Err(_) => {
                Ok(SystemMetricsResponse {
                    cpu_usage: 0.0,
                    memory_usage_mb: 0.0,
                    total_memory_mb: 0.0,
                    ai_model_loaded: false,
                    uptime_seconds: 0,
                })
            }
        }
    } else {
        Ok(SystemMetricsResponse {
            cpu_usage: 0.0,
            memory_usage_mb: 0.0,
            total_memory_mb: 0.0,
            ai_model_loaded: false,
            uptime_seconds: 0,
        })
    }
}

/// Health check for system status
#[tauri::command]
async fn health_check(
    state: State<'_, AppState>,
) -> Result<HealthResponse, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let ai_health = core.ai.health_check().await.unwrap_or(false);
        let db_health = core.content.health_check().await.unwrap_or(false);
        
        Ok(HealthResponse {
            status: if ai_health && db_health { "healthy".to_string() } else { "degraded".to_string() },
            core_initialized: true,
            ai_available: ai_health,
            database_connected: db_health,
        })
    } else {
        Ok(HealthResponse {
            status: "offline".to_string(),
            core_initialized: false,
            ai_available: false,
            database_connected: false,
        })
    }
}

/// Get available document categories
#[tauri::command]
async fn get_categories(
    state: State<'_, AppState>,
) -> Result<Vec<String>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        match core.content.get_categories().await {
            Ok(categories) => Ok(categories),
            Err(e) => {
                println!("Failed to get categories: {}", e);
                Ok(vec!["Philosophy".to_string(), "Science".to_string(), "Technology".to_string()])
            }
        }
    } else {
        Ok(vec!["Philosophy".to_string(), "Science".to_string(), "Technology".to_string()])
    }
}

// AI COMMANDS
// =====================================================

/// Generate AI response to a query
#[tauri::command]
async fn generate_ai_response(
    prompt: String,
    state: State<'_, AppState>,
) -> Result<AiResponse, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let start_time = std::time::Instant::now();
        let result = core.ai.generate_text(&prompt).await;
        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(content) => {
                // Estimate tokens used (rough approximation: ~4 chars per token)
                let tokens_used = (prompt.len() + content.len()) / 4;
                
                Ok(AiResponse {
                    content,
                    model: "test-llama-7b".to_string(),
                    processing_time_ms,
                    tokens_used: tokens_used as u32,
                })
            },
            Err(e) => Err(tauri::Error::Anyhow(anyhow::anyhow!("AI generation failed: {}", e))),
        }
    } else {
        Err(tauri::Error::Anyhow(anyhow::anyhow!("Core not initialized")))
    }
}

/// Simple chat message structure for conversation context
#[derive(Debug, Deserialize)]
pub struct ChatMessageRequest {
    pub role: String,
    pub content: String,
}

/// Generate AI response with streaming support and conversation context
#[tauri::command]
async fn chat_stream(
    prompt: String,
    conversation_history: Option<Vec<ChatMessageRequest>>,
    app_handle: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<AiResponse, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let start_time = std::time::Instant::now();
        
        // Build context from conversation history
        let mut context_prompt = String::new();
        if let Some(history) = conversation_history {
            for msg in history.iter().rev().take(6) { // Take last 6 messages for context
                if msg.role == "user" {
                    context_prompt.push_str(&format!("User: {}\n", msg.content));
                } else if msg.role == "assistant" {
                    context_prompt.push_str(&format!("Assistant: {}\n", msg.content));
                }
            }
        }
        
        // Add current prompt
        context_prompt.push_str(&format!("User: {}\nAssistant:", prompt));
        
        // Create callback for streaming tokens
        let app_handle_clone = app_handle.clone();
        let callback = move |chunk: String| {
            let _ = app_handle_clone.emit_all("ai-chunk", chunk);
        };
        
        let result = core.ai.generate_text_stream(&context_prompt, callback).await;
        let processing_time_ms = start_time.elapsed().as_millis() as u64;
        
        match result {
            Ok(content) => {
                // Estimate tokens used (rough approximation: ~4 chars per token)
                let tokens_used = (prompt.len() + content.len()) / 4;
                
                let response = AiResponse {
                    content: content.clone(),
                    model: "test-llama-7b".to_string(),
                    processing_time_ms,
                    tokens_used: tokens_used as u32,
                };
                
                // Emit completion event
                let _ = app_handle.emit_all("ai-complete", &response);
                
                Ok(response)
            },
            Err(e) => {
                let error_msg = format!("AI generation failed: {}", e);
                let _ = app_handle.emit_all("ai-error", &error_msg);
                Err(tauri::Error::Anyhow(anyhow::anyhow!(error_msg)))
            }
        }
    } else {
        let error_msg = "Core not initialized";
        let _ = app_handle.emit_all("ai-error", error_msg);
        Err(tauri::Error::Anyhow(anyhow::anyhow!(error_msg)))
    }
}

/// Perform RAG query
#[tauri::command]
async fn rag_query(
    query: String,
    context_limit: Option<usize>,
    state: State<'_, AppState>,
) -> Result<CommandResponse<serde_json::Value>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let limit = context_limit.unwrap_or(5);
        let result = core.ai.rag_query(&query, limit).await;
        
        match result {
            Ok(rag_response) => {
                let json_response = serde_json::to_value(&rag_response)
                    .map_err(|e| CommandResponse::error(format!("Serialization error: {}", e)))?;
                Ok(CommandResponse::success(json_response))
            }
            Err(e) => Ok(CommandResponse::error(e.to_string())),
        }
    } else {
        Ok(CommandResponse::error("Core not initialized".to_string()))
    }
}

/// Summarize document
#[tauri::command]
async fn summarize_document(
    document_id: String,
    max_length: Option<usize>,
    state: State<'_, AppState>,
) -> Result<CommandResponse<String>, tauri::Error> {
    let core_lock = state.core.read().await;
    
    if let Some(ref core) = *core_lock {
        let id = match Uuid::parse_str(&document_id) {
            Ok(id) => id,
            Err(_) => return Ok(CommandResponse::error("Invalid document ID".to_string())),
        };

        // Get document content
        if let Ok(Some(doc)) = core.content.get_document(id).await {
            let result = core.ai.summarize(&doc.content, max_length).await;
            Ok(CommandResponse::from(result))
        } else {
            Ok(CommandResponse::error("Document not found".to_string()))
        }
    } else {
        Ok(CommandResponse::error("Core not initialized".to_string()))
    }
}

// =====================================================
// UTILITY FUNCTIONS
// =====================================================

/// Convert database document to DTO
fn document_to_dto(doc: &codex_core::db::models::Document) -> DocumentDto {
    DocumentDto {
        id: doc.id.to_string(),
        title: doc.title.clone(),
        content: doc.content.clone(),
        summary: doc.summary.clone(),
        author: doc.author.clone(),
        category: doc.category.clone(),
        tags: doc.get_tags(),
        language: doc.language.clone(),
        reading_time: doc.reading_time,
        difficulty_level: doc.difficulty_level,
        created_at: doc.created_at.to_rfc3339(),
        updated_at: doc.updated_at.to_rfc3339(),
        view_count: doc.view_count,
        is_favorite: doc.is_favorite,
    }
}

/// Convert DTO to search options
fn dto_to_search_options(dto: SearchOptionsDto) -> codex_core::content::SearchOptions {
    use codex_core::content::{SearchOptions, SearchType, SortBy, SortOrder};

    let search_type = match dto.search_type.as_deref() {
        Some("full_text") => SearchType::FullText,
        Some("semantic") => SearchType::Semantic,
        Some("hybrid") => SearchType::Hybrid,
        _ => SearchType::Hybrid,
    };

    SearchOptions {
        search_type,
        limit: dto.limit.unwrap_or(20),
        offset: dto.offset.unwrap_or(0),
        category: dto.category,
        tags: dto.tags,
        author: dto.author,
        language: None,
        difficulty_level: None,
        date_range: None,
        similarity_threshold: Some(0.3),
        sort_by: SortBy::Relevance,
        sort_order: SortOrder::Descending,
    }
}

// =====================================================
// TAURI APPLICATION SETUP
// =====================================================

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create application state
    let app_state = AppState {
        core: Arc::new(RwLock::new(None)),
    };

    tauri::Builder::default()
        .manage(app_state)
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            initialize_core,
            get_health_status,
            health_check,
            get_system_metrics,
            get_categories,
            import_document,
            import_text_content,
            get_document,
            get_recent_documents,
            search_documents,
            toggle_favorite,
            generate_ai_response,
            chat_stream,
            rag_query,
            summarize_document,
        ])
        .setup(|app| {
            // Get app handle for async initialization
            let app_handle = app.handle().clone();
            
            // Initialize core in background
            tauri::async_runtime::spawn(async move {
                tracing::info!("Starting background core initialization");
                
                let state: State<AppState> = app_handle.state();
                if let Err(e) = initialize_core(state).await {
                    tracing::error!("Failed to initialize core during setup: {:?}", e);
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}