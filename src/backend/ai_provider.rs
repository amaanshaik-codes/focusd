use rusqlite::{Connection, params};
use std::time::Duration;
use reqwest::Client;
use serde::{Serialize, Deserialize};
use tokio::time::sleep;
use std::collections::HashMap;
use chrono::{Utc, DateTime};
use std::time::Duration as StdDuration;
use std::sync::Mutex as StdMutex;
use once_cell::sync::Lazy as LazyOnce;

use crate::backend::personality_db::PERSONALITY_DB_PATH;
use crate::{encrypt_api_key, decrypt_api_key};
use keyring::{Entry};

#[derive(Debug, Serialize, Deserialize)]
pub struct AiResult {
    pub success: bool,
    pub message: Option<String>,
    pub content: Option<String>,
    pub code: Option<String>,
}

// In-memory master secret cache with TTL (best-effort, process-local)
static MASTER_CACHE: LazyOnce<StdMutex<HashMap<String, (String, DateTime<Utc>)>>> = LazyOnce::new(|| StdMutex::new(HashMap::new()));
const MASTER_TTL_SECS: i64 = 300; // 5 minutes

#[tauri::command]
pub async fn store_master_secret_in_keyring(label: String, secret: String) -> Result<(), String> {
    let entry = Entry::new("focusd_master", &label);
    entry.set_password(&secret).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn get_master_secret_from_keyring(label: String) -> Result<Option<String>, String> {
    let entry = Entry::new("focusd_master", &label);
    match entry.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(_) => Ok(None),
    }
}

#[tauri::command]
pub async fn cache_master_secret_temp(label: String, secret: String) -> Result<(), String> {
    let mut map = MASTER_CACHE.lock().unwrap();
    map.insert(label, (secret, Utc::now()));
    Ok(())
}

#[tauri::command]
pub async fn clear_master_secret_cache(label: String) -> Result<(), String> {
    let mut map = MASTER_CACHE.lock().unwrap();
    map.remove(&label);
    Ok(())
}

fn get_cached_master(label: &str) -> Option<String> {
    let mut map = MASTER_CACHE.lock().unwrap();
    if let Some((s, ts)) = map.get(label) {
        if (Utc::now() - *ts).num_seconds() < MASTER_TTL_SECS {
            return Some(s.clone());
        }
    }
    None
}

// Prompt templates per user
#[tauri::command]
pub async fn set_prompt_template(user_id: i64, name: String, template: String) -> Result<(), String> {
    let name_clone = name.clone();
    let template_clone = template.clone();
    let user_id_clone = user_id;
    let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
        conn.execute("CREATE TABLE IF NOT EXISTS prompt_templates (user_id INTEGER, name TEXT, template TEXT, PRIMARY KEY(user_id, name))", []).map_err(|e| e.to_string())?;
        conn.execute("INSERT OR REPLACE INTO prompt_templates (user_id, name, template) VALUES (?, ?, ?)", params![user_id_clone, name_clone, template_clone]).map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?;
    res
}

#[tauri::command]
pub async fn get_prompt_template(user_id: i64, name: String) -> Result<Option<String>, String> {
    let name_clone = name.clone();
    let res = tokio::task::spawn_blocking(move || -> Result<Option<String>, String> {
        let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT template FROM prompt_templates WHERE user_id = ? AND name = ?").map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![user_id, name_clone]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            Ok(row.get(0).map_err(|e| e.to_string())?)
        } else { Ok(None) }
    }).await.map_err(|e| e.to_string())?;
    res
}

#[tauri::command]
pub async fn list_prompt_templates(user_id: i64) -> Result<Vec<(String,String)>, String> {
    let res = tokio::task::spawn_blocking(move || -> Result<Vec<(String,String)>, String> {
        let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT name, template FROM prompt_templates WHERE user_id = ?").map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
        let mut res = Vec::new();
        while let Some(r) = rows.next().map_err(|e| e.to_string())? {
            let name: String = r.get(0).map_err(|e| e.to_string())?;
            let template: String = r.get(1).map_err(|e| e.to_string())?;
            res.push((name, template));
        }
        Ok(res)
    }).await.map_err(|e| e.to_string())?;
    res
}

// Automated journaling / lockscreen note generation
#[tauri::command]
pub async fn generate_journal_entry(user_id: i64, provider: String, master_label: String, prompt_template_name: String, timeout_secs: Option<u64>, model: Option<String>, _store_in_keyring: bool) -> Result<AiResult, String> {
    // consent check + fetch template before any await that touches DB internals
    let user_check = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT ai_opt_in, ai_provider FROM user WHERE id = ?").map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let ai_flag: bool = row.get::<_, Option<bool>>(0).unwrap_or(Some(false)).unwrap_or(false);
            if !ai_flag { return Err("User has not consented to AI operations".to_string()); }
            Ok(())
        } else { Err("User not found".to_string()) }
    }).await.map_err(|e| e.to_string())?;
    user_check?;
    let template = get_prompt_template(user_id, prompt_template_name.clone()).await?;
    let template = template.ok_or("Template not found".to_string())?;

    // Get master secret (try cache, then keyring). If store_in_keyring true, on set we will write
    let master_opt = get_cached_master(&master_label).or_else(|| {
        // try keyring
        let e = Entry::new("focusd_master", &master_label);
        e.get_password().ok()
    });
    let master = master_opt.ok_or("Master secret not found or unlocked")?;

    // Get provider key (use internal sync helper to avoid changing async Send bounds)
    let key_opt = fetch_provider_api_key(user_id, provider.clone(), master.clone())?;
    let key = key_opt.ok_or("API key not found for provider")?;

    // Fill template (simple replacement of {{prompt}} and {{user_id}})
    let filled = template.replace("{{user_id}}", &user_id.to_string()).replace("{{prompt}}", "Please summarize my day and generate a short lockscreen note.");

    // Call provider
    let res = match provider.to_lowercase().as_str() {
        "chatgpt" | "openai" => call_chatgpt(&key, &filled, timeout_secs.unwrap_or(30), model.clone()).await,
        "gemini" | "google" => call_gemini(&key, &filled, timeout_secs.unwrap_or(30), model.clone()).await,
        other => Err(format!("Unknown provider: {}", other)),
    };

    match res {
    Ok(content) => {
            // Safety checks and redaction
            match crate::backend::safety::policy_check(&content) {
                Ok(()) => {
                    // redact PII before saving/display
                    let redacted = crate::backend::safety::redact_pii(&content);
                    // Persist journal entry off the async reactor
                    let provider_clone = provider.clone();
                    let model_clone = model.clone();
                    let redacted_clone = redacted.clone();
                    let save_res = tokio::task::spawn_blocking(move || {
                        crate::backend::journals::save_journal_entry(user_id, provider_clone, model_clone, redacted_clone, None)
                    }).await.map_err(|e| e.to_string())?;
                    match save_res {
                        Ok(id) => Ok(AiResult { success: true, message: Some(format!("saved: {}", id)), content: Some(content), code: None }),
                        Err(e) => Ok(AiResult { success: true, message: Some(format!("save_failed: {}", e)), content: Some(content), code: Some("save_error".to_string()) }),
                    }
                }
                Err(policy_err) => Ok(AiResult { success: false, message: Some(policy_err), content: None, code: Some("policy_violation".to_string()) }),
            }
        }
        Err(e) => Ok(AiResult { success: false, message: Some(e), content: None, code: Some("provider_error".to_string()) }),
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub enum AiProvider {
    ChatGPT,
    Gemini,
}

#[tauri::command]
pub async fn set_provider_api_key(user_id: i64, provider: String, api_key: String, master: String) -> Result<AiResult, String> {
    // Store encrypted API key in DB for portability, and also attempt to store in OS keyring for extra security.
    let api_key_clone = api_key.clone();
    let provider_clone = provider.clone();
    let master_clone = master.clone();
    let res = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
        let enc = encrypt_api_key(&api_key_clone, &master_clone);
        conn.execute("CREATE TABLE IF NOT EXISTS api_keys (user_id INTEGER, provider TEXT, key_enc TEXT, PRIMARY KEY(user_id, provider))", []).map_err(|e| e.to_string())?;
        conn.execute("INSERT OR REPLACE INTO api_keys (user_id, provider, key_enc) VALUES (?, ?, ?)", params![user_id, provider_clone, enc]).map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?;
    res?;

    // Try keyring (best-effort)
    let kr = Entry::new(&format!("focusd_provider_{}", provider), &format!("user_{}", user_id));
    if let Err(e) = kr.set_password(&api_key) {
        eprintln!("keyring set error: {:?}", e);
    }
    Ok(AiResult { success: true, message: None, content: None, code: None })
}

#[tauri::command]
pub async fn get_provider_api_key(user_id: i64, provider: String, master: String) -> Result<AiResult, String> {
    match tokio::task::spawn_blocking(move || fetch_provider_api_key(user_id, provider.clone(), master)).await.map_err(|e| e.to_string())? {
        Ok(opt) => Ok(AiResult { success: true, message: None, content: opt, code: None }),
        Err(e) => Ok(AiResult { success: false, message: Some(e), content: None, code: Some("key_error".to_string()) }),
    }
}

// Internal helper used by async flows to avoid changing their await/Send behaviour
pub fn fetch_provider_api_key(user_id: i64, provider: String, master: String) -> Result<Option<String>, String> {
    // Try OS keyring first (preferred), then fall back to DB-encrypted copy.
    let kr = Entry::new(&format!("focusd_provider_{}", provider), &format!("user_{}", user_id));
    match kr.get_password() {
        Ok(pw) if !pw.is_empty() => return Ok(Some(pw)),
        _ => { /* fall back to DB */ }
    }

    let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT key_enc FROM api_keys WHERE user_id = ? AND provider = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id, provider]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let enc: String = row.get(0).map_err(|e| e.to_string())?;
        Ok(decrypt_api_key(&enc, &master))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn delete_provider_api_key(user_id: i64, provider: String) -> Result<AiResult, String> {
    let provider_clone = provider.clone();
    let _res = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
        conn.execute("DELETE FROM api_keys WHERE user_id = ? AND provider = ?", params![user_id, provider_clone]).map_err(|e| e.to_string())?;
        Ok(())
    }).await.map_err(|e| e.to_string())?;
    let kr = Entry::new(&format!("focusd_provider_{}", provider), &format!("user_{}", user_id));
    let _ = kr.delete_password(); // best-effort
    Ok(AiResult { success: true, message: None, content: None, code: None })
}

// Minimal ChatGPT client (uses OpenAI-compatible REST endpoint)
pub async fn call_chatgpt(api_key: &str, prompt: &str, timeout_secs: u64, model: Option<String>) -> Result<String, String> {
    let client = Client::builder().timeout(StdDuration::from_secs(timeout_secs)).build().map_err(|e| e.to_string())?;
    let url = "https://api.openai.com/v1/chat/completions";
    let model = model.unwrap_or_else(|| "gpt-4o-mini".to_string());
    let body = serde_json::json!({
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "max_tokens": 800
    });

    // Simple retry with exponential backoff
    let mut attempt = 0u32;
    let max_attempts = 3;
    let mut last_err = None;
    while attempt < max_attempts {
        attempt += 1;
        let res = client.post(url)
            .bearer_auth(api_key)
            .json(&body)
            .send().await;
        match res {
            Ok(r) => {
                if !r.status().is_success() {
                    last_err = Some(format!("ChatGPT API error: {}", r.status()));
                } else {
                    let j: serde_json::Value = r.json().await.map_err(|e| e.to_string())?;
                    return Ok(parse_chatgpt_response(&j));
                }
            }
            Err(e) => {
                last_err = Some(e.to_string());
            }
    }
    // backoff (non-blocking)
    let backoff_ms = 100u64 * (2u64.pow(attempt));
    sleep(StdDuration::from_millis(backoff_ms)).await;
    }
    Err(last_err.unwrap_or_else(|| "Unknown ChatGPT error".to_string()))
}

// Minimal Gemini client (Google's PaLM REST API compatibility)
pub async fn call_gemini(api_key: &str, prompt: &str, timeout_secs: u64, model: Option<String>) -> Result<String, String> {
    let client = Client::builder().timeout(StdDuration::from_secs(timeout_secs)).build().map_err(|e| e.to_string())?;
    let model = model.unwrap_or_else(|| "gemini-2.0-flash".to_string());
    // Use the Generative Language API `generateContent` endpoint and the `contents -> parts -> text` body
    let url = format!("https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent", model);
    let body = serde_json::json!({
        "contents": [
            { "parts": [ { "text": prompt } ] }
        ]
    });

    let mut attempt = 0u32;
    let max_attempts = 3;
    let mut last_err = None;
    while attempt < max_attempts {
        attempt += 1;
        // Google GenAI supports both OAuth bearer tokens and simple API keys.
        // If the provided key looks like a Google API key (starts with "AIza"),
        // pass it as a query parameter and x-goog-api-key header instead of Bearer auth.
        let res = if api_key.starts_with("AIza") {
            let url_with_key = format!("{}?key={}", url, api_key);
            client.post(&url_with_key)
                .header("x-goog-api-key", api_key)
                .json(&body)
                .send().await
        } else {
            client.post(&url)
                .bearer_auth(api_key)
                .json(&body)
                .send().await
        };
        match res {
            Ok(r) => {
                let status = r.status();
                // read response body text for richer error messages / parsing
                let body_text = r.text().await.map_err(|e| e.to_string())?;
                if !status.is_success() {
                    last_err = Some(format!("Gemini API error: {}: {}", status, body_text));
                } else {
                    let j: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| e.to_string())?;
                    return Ok(parse_gemini_response(&j));
                }
            }
            Err(e) => {
                last_err = Some(e.to_string());
            }
    }
    let backoff_ms = 100u64 * (2u64.pow(attempt));
    sleep(StdDuration::from_millis(backoff_ms)).await;
    }
    Err(last_err.unwrap_or_else(|| "Unknown Gemini error".to_string()))
}

// Parsing helpers separated for easier unit testing
pub fn parse_chatgpt_response(j: &serde_json::Value) -> String {
    // Try common shapes without duplicating text
    if let Some(content) = j.pointer("/choices/0/message/content") {
        if let Some(s) = content.as_str() { return s.to_string(); }
    }
    if let Some(text) = j.pointer("/choices/0/text") {
        if let Some(s) = text.as_str() { return s.to_string(); }
    }
    // Newer OpenAI responses may include 'choices' -> [{ 'message': { 'content': { 'parts': [...] } } }]
    if let Some(parts) = j.pointer("/choices/0/message/content/parts") {
        if let Some(arr) = parts.as_array() {
            return arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join("");
        }
    }
    // Fallback to concatenating any top-level text-like fields to avoid duplication of repeated fields
    if let Some(c) = j.get("text") {
        if let Some(s) = c.as_str() { return s.to_string(); }
    }
    j.to_string()
}

pub fn parse_gemini_response(j: &serde_json::Value) -> String {
    // Gemini/PaLM responses often contain candidates with content as string or objects
    if let Some(candidate) = j.pointer("/candidates/0") {
        if let Some(s) = candidate.get("content").and_then(|v| v.as_str()) {
            return s.to_string();
        }
        if let Some(obj) = candidate.get("content") {
            // If content is an object with 'text' or 'parts'
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) {
                return text.to_string();
            }
            if let Some(parts) = obj.get("parts").and_then(|v| v.as_array()) {
                let mut out = String::new();
                for p in parts {
                    if let Some(s) = p.as_str() {
                        out.push_str(s);
                    } else if let Some(text) = p.get("text").and_then(|v| v.as_str()) {
                        out.push_str(text);
                    } else if let Some(content) = p.get("content").and_then(|v| v.as_str()) {
                        out.push_str(content);
                    }
                }
                if !out.is_empty() { return out; }
            }
        }
    }
    // Some Gemini responses use 'output' -> 'candidates'
    if let Some(content_val) = j.pointer("/output/candidates/0/content") {
        if let Some(s) = content_val.as_str() { return s.to_string(); }
        if let Some(obj) = content_val.as_object() {
            if let Some(text) = obj.get("text").and_then(|v| v.as_str()) { return text.to_string(); }
            if let Some(parts) = obj.get("parts").and_then(|v| v.as_array()) {
                let mut out = String::new();
                for p in parts {
                    if let Some(s) = p.as_str() {
                        out.push_str(s);
                    } else if let Some(text) = p.get("text").and_then(|v| v.as_str()) {
                        out.push_str(text);
                    }
                }
                if !out.is_empty() { return out; }
            }
        }
    }
    j.to_string()
}

#[tauri::command]
pub async fn generate_ai_via_provider(user_id: i64, provider: String, master: String, prompt: String, timeout_secs: Option<u64>, model: Option<String>) -> Result<AiResult, String> {
    // consent check: ensure user opted into AI
    let ai_opt_in = {
        let conn = Connection::open(PERSONALITY_DB_PATH).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT ai_opt_in FROM user WHERE id = ?").map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
        if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            row.get::<_, Option<bool>>(0).unwrap_or(Some(false)).unwrap_or(false)
        } else {
            return Err("User not found".to_string());
        }
    };
    if !ai_opt_in { return Err("User has not consented to AI operations".to_string()); }

    // Now safe to call key retrieval; use internal helper that returns Option<String>
    let key_opt = fetch_provider_api_key(user_id, provider.clone(), master)?;
    let key = key_opt.ok_or("API key not found for provider")?;
    let to = timeout_secs.unwrap_or(30);
    let res = match provider.to_lowercase().as_str() {
        "chatgpt" | "openai" => call_chatgpt(&key, &prompt, to, model.clone()).await,
        "gemini" | "google" => call_gemini(&key, &prompt, to, model.clone()).await,
        other => Err(format!("Unknown provider: {}", other)),
    };
    match res {
    Ok(c) => {
            // Run policy check and redact before returning/saving
            if let Err(pol) = crate::backend::safety::policy_check(&c) {
                return Ok(AiResult { success: false, message: Some(pol), content: None, code: Some("policy_violation".to_string()) });
            }
            let redacted = crate::backend::safety::redact_pii(&c);
            // Save in background (best-effort)
            let provider_clone = provider.clone();
            let model_clone = model.clone();
            let redacted_clone = redacted.clone();
            let _ = tokio::task::spawn_blocking(move || {
                let _ = crate::backend::journals::save_journal_entry(user_id, provider_clone, model_clone, redacted_clone, None);
            }).await;
            Ok(AiResult { success: true, message: None, content: Some(c), code: None })
        }
        Err(e) => Ok(AiResult { success: false, message: Some(e), content: None, code: Some("provider_error".to_string()) }),
    }
}
