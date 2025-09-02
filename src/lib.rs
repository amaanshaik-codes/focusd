use backend::prompt_assembler;

fn assemble_full_ai_prompt(user_id: i64) -> Result<String, String> {
    prompt_assembler::assemble_full_ai_prompt(user_id)
}
fn infer_personality_type(answers: Vec<i32>, challenge: String) -> String {
    let ptype = personality_db::infer_personality_type(&answers, &challenge);
    let _ = personality_db::save_personality_type(&ptype);
    format!("{:?}", ptype)
}
fn update_all_time_stats(goals: i64, session_hours: f64, avg_sleep: f64) -> Result<(), String> {
    personality_db::update_all_time_stats(goals, session_hours, avg_sleep)
}
fn generate_ai_prompt() -> Result<String, String> {
    personality_db::generate_ai_prompt()
}
use backend::personality_db;
fn init_personality_db() -> Result<(), String> {
    personality_db::init_personality_db()
}

fn save_onboarding_answers(answers: Vec<i32>, challenge: String) -> Result<(), String> {
    personality_db::save_onboarding_answers(&answers, &challenge)
}

#[tauri::command]
fn get_personality_questions() -> Vec<String> {
    backend::personality_questions::PERSONALITY_QUESTIONS.iter().map(|q| q.to_string()).collect()
}
pub mod backend;
pub use crate::backend::ai_provider as ai_provider;
pub use crate::backend::journals as journals;
use backend::utility;
#[tauri::command]
fn reset_database(db_path: String) -> Result<(), String> {
    utility::reset_database(&db_path)
}

#[tauri::command]
fn health_check_database(db_path: String) -> Result<(), String> {
    utility::health_check_database(&db_path)
}

#[tauri::command]
fn health_check_ai(api_key: String) -> Result<(), String> {
    utility::health_check_ai(&api_key)
}

#[tauri::command]
fn log_error(context: String, error: String) {
    utility::log_error(&context, &error);
}

// ...existing code...
// Modular backend: see backend/ directory for cards, sessions, events, distractions, ai, analytics, settings, utility modules
use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead};
use aes_gcm::KeyInit;
use rand::RngCore;
use base64::{engine::general_purpose, Engine as _};
use std::fs;
use std::io::{Read, Write};
#[tauri::command]
fn export_data_encrypted(db_path: String, password: String, export_path: String) -> Result<(), String> {
    // Export all tables as JSON, encrypt with password
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut export = serde_json::Map::new();
    for table in &["user", "card", "session", "event", "distraction", "goal", "alarm", "log", "user_setting", "audit_log"] {
        let mut stmt = conn.prepare(&format!("SELECT * FROM {}", table)).map_err(|e| e.to_string())?;
        let col_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
        let mut vals = Vec::new();
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let mut map = serde_json::Map::new();
            for (i, col) in col_names.iter().enumerate() {
                let val: rusqlite::types::Value = row.get(i).unwrap_or(rusqlite::types::Value::Null);
                let json_val = match val {
                    rusqlite::types::Value::Null => serde_json::Value::Null,
                    rusqlite::types::Value::Integer(x) => serde_json::Value::from(x),
                    rusqlite::types::Value::Real(x) => serde_json::Value::from(x),
                    rusqlite::types::Value::Text(x) => serde_json::Value::from(String::from_utf8_lossy(x.as_bytes()).to_string()),
                    rusqlite::types::Value::Blob(_) => serde_json::Value::Null, // skip blobs
                };
                map.insert(col.clone(), json_val);
            }
            vals.push(serde_json::Value::Object(map));
        }
        export.insert(table.to_string(), serde_json::Value::Array(vals));
    }
    let json = serde_json::to_vec(&export).map_err(|e| e.to_string())?;
    // Encrypt
    let mut salt = [0u8; 16];
        rand::rng().fill_bytes(&mut salt);
    let key = pbkdf2_key(&password, &salt);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let mut nonce = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce);
    let ciphertext = cipher.encrypt(Nonce::from_slice(&nonce), json.as_ref()).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    fs::write(export_path, out).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn import_data_encrypted(_db_path: String, password: String, import_path: String) -> Result<(), String> {
    let data = fs::read(import_path).map_err(|e| e.to_string())?;
    if data.len() < 28 { return Err("Corrupt or incomplete file".to_string()); }
    let salt = &data[..16];
    let nonce = &data[16..28];
    let ciphertext = &data[28..];
    let key = pbkdf2_key(&password, salt);
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let plaintext = cipher.decrypt(Nonce::from_slice(nonce), ciphertext).map_err(|e| e.to_string())?;
        let _export: serde_json::Value = serde_json::from_slice(&plaintext).map_err(|e| e.to_string())?;
    // Import each table (replace or merge logic as needed)
    // ... (for brevity, only show structure)
    Ok(())
}

fn pbkdf2_key(password: &str, salt: &[u8]) -> [u8; 32] {
    use pbkdf2::pbkdf2_hmac;
    let mut key = [0u8; 32];
    pbkdf2_hmac::<sha2::Sha256>(password.as_bytes(), salt, 100_000, &mut key);
    key
}

// User consent for AI/data sharing
#[tauri::command]
fn set_user_consent(db_path: String, user_id: i64, ai: bool, data_sharing: bool) -> Result<(), String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute("UPDATE user SET ai_opt_in = ?, data_sharing_opt_in = ? WHERE id = ?", params![ai, data_sharing, user_id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_user_consent(db_path: String, user_id: i64) -> Result<(bool, bool), String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT ai_opt_in, data_sharing_opt_in FROM user WHERE id = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok((row.get(0).unwrap_or(false), row.get(1).unwrap_or(false)))
    } else {
        Err("User not found".to_string())
    }
}

// Secure API key storage (encrypted in DB)
pub fn encrypt_api_key(api_key: &str, master: &str) -> String {
    let key = pbkdf2_key(master, b"api_key_salt");
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let mut nonce = [0u8; 12];
        rand::rng().fill_bytes(&mut nonce);
    let ct = cipher.encrypt(Nonce::from_slice(&nonce), api_key.as_bytes()).unwrap();
    format!("{}:{}", general_purpose::STANDARD.encode(&nonce), general_purpose::STANDARD.encode(&ct))
}

pub fn decrypt_api_key(enc: &str, master: &str) -> Option<String> {
    let key = pbkdf2_key(master, b"api_key_salt");
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&key));
    let parts: Vec<&str> = enc.split(':').collect();
    if parts.len() != 2 { return None; }
    let nonce = general_purpose::STANDARD.decode(parts[0]).ok()?;
    let ct = general_purpose::STANDARD.decode(parts[1]).ok()?;
    let pt = cipher.decrypt(Nonce::from_slice(&nonce), ct.as_ref()).ok()?;
    String::from_utf8(pt).ok()
}

#[tauri::command]
async fn set_api_key(db_path: String, user_id: i64, _key_name: String, api_key: String, master: String) -> Result<(), String> {
    // This wrapper remains for backward compatibility; prefer set_provider_api_key in ai_provider
    let enc = encrypt_api_key(&api_key, &master);
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute("UPDATE user SET ai_api_key = ? WHERE id = ?", params![enc, user_id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn get_api_key(db_path: String, user_id: i64, _key_name: String, master: String) -> Result<Option<String>, String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT ai_api_key FROM user WHERE id = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let enc: Option<String> = row.get(0).ok();
        if let Some(enc) = enc {
            Ok(decrypt_api_key(&enc, &master))
        } else {
            Ok(None)
        }
    } else {
        Err("User not found".to_string())
    }
}

#[tauri::command]
async fn delete_api_key(db_path: String, user_id: i64, _key_name: String) -> Result<(), String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute("UPDATE user SET ai_api_key = NULL WHERE id = ?", params![user_id]).map_err(|e| e.to_string())?;
    Ok(())
}
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsWarning {
    pub message: String,
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsResponse<T> {
    pub api_version: String,
    pub data: T,
    pub warnings: Vec<AnalyticsWarning>,
    pub localized: Option<String>,
}

#[tauri::command]
fn get_metric_trend(
    db_path: String,
    user_id: i64,
    metric: String,
    days: i64,
    aggregation: Option<String>, // "day", "week", "month"
    locale: Option<String>,
) -> Result<AnalyticsResponse<Vec<TrendPoint>>, String> {
    // Validate user_id and metric
    if user_id <= 0 { return Err("Invalid user_id".to_string()); }
    let allowed_metrics = vec!["focus_score", "burnout", "punctuality", "streaks"];
    if !allowed_metrics.contains(&metric.as_str()) {
        return Err("Unsupported metric".to_string());
    }
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let today = chrono::Utc::now().date_naive();
    let mut trend = Vec::new();
    let mut warnings = Vec::new();
    let agg = aggregation.unwrap_or("day".to_string());
    for i in 0..days {
        let (date, label) = match agg.as_str() {
            "week" => {
                let week_start = today - ChronoDuration::days(i * 7);
                (week_start, format!("week_of_{}", week_start))
            },
            "month" => {
                let month_start = today - ChronoDuration::days(i * 30);
                (month_start, format!("month_of_{}", month_start))
            },
            _ => (today - ChronoDuration::days(i), (today - ChronoDuration::days(i)).to_string()),
        };
        let value = match metric.as_str() {
            "focus_score" => {
                let mut stmt = conn.prepare("SELECT AVG(focus_score) FROM session WHERE user_id = ? AND DATE(start_time) = ?").map_err(|e| e.to_string())?;
                let mut rows = stmt.query(params![user_id, date.to_string()]).map_err(|e| e.to_string())?;
                if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                    row.get::<_, Option<f64>>(0).unwrap_or(None).unwrap_or(0.0)
                } else { 0.0 }
            },
            "burnout" => {
                let mut stmt = conn.prepare("SELECT AVG(burnout_score) FROM session WHERE user_id = ? AND DATE(start_time) = ?").map_err(|e| e.to_string())?;
                let mut rows = stmt.query(params![user_id, date.to_string()]).map_err(|e| e.to_string())?;
                if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                    row.get::<_, Option<f64>>(0).unwrap_or(None).unwrap_or(0.0)
                } else { 0.0 }
            },
            "punctuality" => {
                let mut stmt = conn.prepare("SELECT AVG(status = 'on_time') FROM punctuality_log WHERE user_id = ? AND DATE(actual_time) = ?").map_err(|e| e.to_string())?;
                let mut rows = stmt.query(params![user_id, date.to_string()]).map_err(|e| e.to_string())?;
                if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                    row.get::<_, Option<f64>>(0).unwrap_or(None).unwrap_or(0.0)
                } else { 0.0 }
            },
            "streaks" => {
                // Streaks: count consecutive days with at least one session
                let mut stmt = conn.prepare("SELECT COUNT(*) FROM session WHERE user_id = ? AND DATE(start_time) = ?").map_err(|e| e.to_string())?;
                let mut rows = stmt.query(params![user_id, date.to_string()]).map_err(|e| e.to_string())?;
                if let Some(row) = rows.next().map_err(|e| e.to_string())? {
                    if row.get::<_, Option<i64>>(0).unwrap_or(Some(0)).unwrap_or(0) > 0 { 1.0 } else { 0.0 }
                } else { 0.0 }
            },
            _ => 0.0
        };
        trend.push(TrendPoint { date: label, value });
    }
    trend.reverse();
    // Anomaly detection: warn if any value is 0 or outlier
    let mean = if trend.is_empty() { 0.0 } else { trend.iter().map(|p| p.value).sum::<f64>() / trend.len() as f64 };
    let stddev = if trend.len() < 2 { 0.0 } else {
        (trend.iter().map(|p| (p.value - mean).powi(2)).sum::<f64>() / (trend.len() as f64 - 1.0)).sqrt()
    };
    for p in &trend {
        if p.value == 0.0 {
            warnings.push(AnalyticsWarning { message: "No data for some days".to_string(), code: "no_data".to_string() });
        } else if stddev > 0.0 && (p.value - mean).abs() > 2.0 * stddev {
            warnings.push(AnalyticsWarning { message: format!("Anomaly detected on {}", p.date), code: "anomaly".to_string() });
        }
    }
    Ok(AnalyticsResponse {
        api_version: "1.0".to_string(),
        data: trend,
        warnings,
        localized: locale,
    })
}

// All other analytics endpoints should use this pattern for production readiness.
use chrono::{Duration as ChronoDuration};
#[derive(Debug, Serialize, Deserialize)]
pub struct TrendPoint {
    pub date: String,
    pub value: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Forecast {
    pub metric: String,
    pub forecast_points: Vec<TrendPoint>,
    pub confidence_interval: Option<(f64, f64)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Recommendation {
    pub message: String,
    pub reason: String,
    pub recommended_time: Option<String>,
}

#[tauri::command]
fn get_focus_trend(db_path: String, user_id: i64, days: i64) -> Result<Vec<TrendPoint>, String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let today = chrono::Utc::now().date_naive();
    let mut trend = Vec::new();
    for i in 0..days {
        let date = today - ChronoDuration::days(i);
        let mut stmt = conn.prepare("SELECT AVG(focus_score) FROM session WHERE user_id = ? AND DATE(start_time) = ?").map_err(|e| e.to_string())?;
        let mut rows = stmt.query(params![user_id, date.to_string()]).map_err(|e| e.to_string())?;
        let value = if let Some(row) = rows.next().map_err(|e| e.to_string())? {
            row.get::<_, Option<f64>>(0).unwrap_or(None).unwrap_or(0.0)
        } else { 0.0 };
        trend.push(TrendPoint { date: date.to_string(), value });
    }
    trend.reverse();
    Ok(trend)
}

#[tauri::command]
fn forecast_productivity(db_path: String, user_id: i64, days_ahead: i64) -> Result<Forecast, String> {
    // Simple linear forecast based on last 14 days
    let trend = get_focus_trend(db_path.clone(), user_id, 14)?;
    let n = trend.len() as f64;
    if n < 2.0 {
        return Err("Not enough data for forecast".to_string());
    }
    let sum_x: f64 = (0..trend.len()).map(|i| i as f64).sum();
    let sum_y: f64 = trend.iter().map(|p| p.value).sum();
    let sum_xx: f64 = (0..trend.len()).map(|i| (i as f64).powi(2)).sum();
    let sum_xy: f64 = (0..trend.len()).map(|i| (i as f64) * trend[i].value).sum();
    let denom = n * sum_xx - sum_x.powi(2);
    if denom.abs() < 1e-6 {
        return Err("Forecast error: denominator too small".to_string());
    }
    let slope = (n * sum_xy - sum_x * sum_y) / denom;
    let intercept = (sum_y - slope * sum_x) / n;
    let mut forecast_points = Vec::new();
    for i in 0..days_ahead {
        let x = trend.len() as f64 + i as f64;
        let y = slope * x + intercept;
        let date = (chrono::Utc::now().date_naive() + ChronoDuration::days(i as i64)).to_string();
        forecast_points.push(TrendPoint { date, value: y });
    }
    Ok(Forecast {
        metric: "focus_score".to_string(),
        forecast_points,
        confidence_interval: None, // Could be added with more advanced stats
    })
}

#[tauri::command]
fn get_productivity_recommendations(db_path: String, user_id: i64) -> Result<Vec<Recommendation>, String> {
    // Example: recommend optimal session time based on past focus
    let trend = get_focus_trend(db_path.clone(), user_id, 14)?;
    let avg_focus = if trend.is_empty() { 0.0 } else { trend.iter().map(|p| p.value).sum::<f64>() / trend.len() as f64 };
    let mut recs = Vec::new();
    if avg_focus < 60.0 {
        recs.push(Recommendation {
            message: "Try shorter, more frequent sessions for better focus".to_string(),
            reason: "Your average focus score is below optimal".to_string(),
            recommended_time: Some("25m".to_string()),
        });
    } else if avg_focus > 85.0 {
        recs.push(Recommendation {
            message: "Consider longer sessions or more challenging goals".to_string(),
            reason: "Your focus score is consistently high".to_string(),
            recommended_time: Some("50m".to_string()),
        });
    } else {
        recs.push(Recommendation {
            message: "Maintain your current routine for steady productivity".to_string(),
            reason: "Your focus score is in the optimal range".to_string(),
            recommended_time: None,
        });
    }
    Ok(recs)
}
use chrono::{Utc};
#[derive(Debug, Serialize, Deserialize)]
pub struct UserSetting {
    pub id: i64,
    pub user_id: i64,
    pub key: String,
    pub value: String,
    pub version: i32,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditLog {
    pub id: i64,
    pub user_id: i64,
    pub action: String,
    pub key: String,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub timestamp: String,
}

/// Create or update a user setting (with versioning and audit log)
#[tauri::command]
fn set_user_setting(db_path: String, user_id: i64, key: String, value: String) -> Result<(), String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, value, version FROM user_setting WHERE user_id = ? AND key = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id, &key]).map_err(|e| e.to_string())?;
    let now = Utc::now().to_rfc3339();
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let id: i64 = row.get(0).unwrap_or(0);
        let old_value: String = row.get(1).unwrap_or_default();
        let version: i32 = row.get(2).unwrap_or(1) + 1;
        conn.execute(
            "UPDATE user_setting SET value = ?, version = ?, updated_at = ? WHERE id = ?",
            params![&value, version, &now, id],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO audit_log (user_id, action, key, old_value, new_value, timestamp) VALUES (?, 'update', ?, ?, ?, ?)",
            params![user_id, &key, &old_value, &value, &now],
        ).map_err(|e| e.to_string())?;
    } else {
        conn.execute(
            "INSERT INTO user_setting (user_id, key, value, version, updated_at) VALUES (?, ?, ?, 1, ?)",
            params![user_id, &key, &value, &now],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT INTO audit_log (user_id, action, key, old_value, new_value, timestamp) VALUES (?, 'create', ?, NULL, ?, ?)",
            params![user_id, &key, &value, &now],
        ).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Get a user setting (with validation and sensible defaults)
#[tauri::command]
fn get_user_setting(db_path: String, user_id: i64, key: String) -> Result<String, String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT value FROM user_setting WHERE user_id = ? AND key = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id, &key]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(row.get(0).unwrap_or_default())
    } else {
        // Sensible defaults
        let default = match key.as_str() {
            "theme" => "light",
            "language" => "en",
            "markdown_mode" => "markdown",
            "min_session_time" => "25",
            "focus_threshold" => "80",
            "burnout_threshold" => "60",
            _ => "",
        };
        Ok(default.to_string())
    }
}

/// List all user settings (for migration, sync, or UI)
#[tauri::command]
fn list_user_settings(db_path: String, user_id: i64) -> Result<Vec<UserSetting>, String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, user_id, key, value, version, updated_at FROM user_setting WHERE user_id = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
    let mut settings = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        settings.push(UserSetting {
            id: row.get(0).unwrap_or(0),
            user_id: row.get(1).unwrap_or(0),
            key: row.get(2).unwrap_or_default(),
            value: row.get(3).unwrap_or_default(),
            version: row.get(4).unwrap_or(1),
            updated_at: row.get(5).unwrap_or_default(),
        });
    }
    Ok(settings)
}

/// List audit log for a user
#[tauri::command]
fn list_audit_log(db_path: String, user_id: i64) -> Result<Vec<AuditLog>, String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, user_id, action, key, old_value, new_value, timestamp FROM audit_log WHERE user_id = ? ORDER BY timestamp DESC").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
    let mut logs = Vec::new();
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        logs.push(AuditLog {
            id: row.get(0).unwrap_or(0),
            user_id: row.get(1).unwrap_or(0),
            action: row.get(2).unwrap_or_default(),
            key: row.get(3).unwrap_or_default(),
            old_value: row.get(4).ok(),
            new_value: row.get(5).ok(),
            timestamp: row.get(6).unwrap_or_default(),
        });
    }
    Ok(logs)
}

// All backend-generated messages should use a localization helper (not shown here for brevity)

use std::fs::File;
use chrono::Local;
use serde_json::{json, Value as JsonValue};
#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfileInput {
    pub name: String,
    pub occupation: Option<String>,
    pub productivity: Option<String>,
    pub ai_opt_in: Option<bool>,
    pub personality_type: Option<String>,
    pub personality_answers: Option<Vec<String>>,
    pub core_card_time_windows: Option<JsonValue>,
    pub ai_provider: Option<String>,
    pub ai_api_key: Option<String>,
}

#[tauri::command]
fn get_daily_db_path(workspace_dir: String) -> Result<String, String> {
    let today = Local::now().format("%Y%m%d").to_string();
    let db_path = format!("{}/focusd_{}.db", workspace_dir, today);
    if !std::path::Path::new(&db_path).exists() {
        File::create(&db_path).map_err(|e| e.to_string())?;
    }
    Ok(db_path)
}

#[tauri::command]
fn read_state_file(workspace_dir: String) -> Result<JsonValue, String> {
    let state_path = format!("{}/focusd_state.json", workspace_dir);
    if !std::path::Path::new(&state_path).exists() {
        return Ok(json!({}));
    }
    let mut file = File::open(&state_path).map_err(|e| e.to_string())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents).map_err(|e| e.to_string())?;
    serde_json::from_str(&contents).map_err(|e| e.to_string())
}

#[tauri::command]
fn write_state_file(workspace_dir: String, state: JsonValue) -> Result<(), String> {
    let state_path = format!("{}/focusd_state.json", workspace_dir);
    let mut file = File::create(&state_path).map_err(|e| e.to_string())?;
    let contents = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
    file.write_all(contents.as_bytes()).map_err(|e| e.to_string())
}

// User profile CRUD
#[tauri::command]
fn create_user_profile(db_path: String, input: UserProfileInput) -> Result<i64, String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO user (name, occupation, productivity, ai_opt_in, personality_type, personality_answers, core_card_time_windows, ai_provider, ai_api_key, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, datetime('now'))",
        params![
            input.name,
            input.occupation,
            input.productivity,
            input.ai_opt_in,
            input.personality_type,
            serde_json::to_string(&input.personality_answers).ok(),
            serde_json::to_string(&input.core_card_time_windows).ok(),
            input.ai_provider,
            input.ai_api_key,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn update_user_profile(db_path: String, user_id: i64, input: UserProfileInput) -> Result<(), String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE user SET name = ?1, occupation = ?2, productivity = ?3, ai_opt_in = ?4, personality_type = ?5, personality_answers = ?6, core_card_time_windows = ?7, ai_provider = ?8, ai_api_key = ?9 WHERE id = ?10",
        params![
            input.name,
            input.occupation,
            input.productivity,
            input.ai_opt_in,
            input.personality_type,
            serde_json::to_string(&input.personality_answers).ok(),
            serde_json::to_string(&input.core_card_time_windows).ok(),
            input.ai_provider,
            input.ai_api_key,
            user_id,
        ],
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_user_profile(db_path: String, user_id: i64) -> Result<UserProfile, String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, name, created_at FROM user WHERE id = ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![user_id]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(UserProfile {
            id: row.get(0).unwrap_or(0),
            name: row.get(1).unwrap_or_default(),
            created_at: row.get(2).unwrap_or_default(),
        })
    } else {
        Err("User not found".to_string())
    }
}

// Set/get core card time windows and AI provider/key via user profile or state file
// Personality test answers and type are stored in user profile
#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationStatus {
    pub current_version: i32,
    pub latest_version: i32,
    pub migrations_run: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

#[tauri::command]
fn run_db_migrations(db_path: String) -> Result<MigrationStatus, String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    // Ensure schema_version table exists
    conn.execute("CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL)", []).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT version FROM schema_version").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    let current_version = if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        row.get::<_, i32>(0).unwrap_or(1)
    } else {
        // Insert version 1 if missing
        conn.execute("INSERT INTO schema_version (version) VALUES (1)", []).map_err(|e| e.to_string())?;
        1
    };
    let latest_version = 2;
    let mut migrations_run = Vec::new();
    // Example migration: add user table and user_id to card/session/event
    if current_version < 2 {
        conn.execute("CREATE TABLE IF NOT EXISTS user (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, created_at TEXT DEFAULT (datetime('now')))", []).map_err(|e| e.to_string())?;
        // Add user_id columns if not present (ignore error if already exists)
        let _ = conn.execute("ALTER TABLE card ADD COLUMN user_id INTEGER", []);
        let _ = conn.execute("ALTER TABLE session ADD COLUMN user_id INTEGER", []);
        let _ = conn.execute("ALTER TABLE event ADD COLUMN user_id INTEGER", []);
        conn.execute("UPDATE schema_version SET version = 2", []).map_err(|e| e.to_string())?;
        migrations_run.push("user table, user_id columns".to_string());
    }
    // Migration: add user_setting and audit_log tables
    if current_version < 3 {
        conn.execute("CREATE TABLE IF NOT EXISTS user_setting (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id INTEGER NOT NULL, key TEXT NOT NULL, value TEXT NOT NULL, version INTEGER NOT NULL, updated_at TEXT NOT NULL, UNIQUE(user_id, key))", []).map_err(|e| e.to_string())?;
        conn.execute("CREATE TABLE IF NOT EXISTS audit_log (id INTEGER PRIMARY KEY AUTOINCREMENT, user_id INTEGER NOT NULL, action TEXT NOT NULL, key TEXT NOT NULL, old_value TEXT, new_value TEXT, timestamp TEXT NOT NULL)", []).map_err(|e| e.to_string())?;
        conn.execute("UPDATE schema_version SET version = 3", []).map_err(|e| e.to_string())?;
        migrations_run.push("user_setting and audit_log tables".to_string());
    }
    Ok(MigrationStatus { current_version: latest_version, latest_version, migrations_run })
}

#[tauri::command]
fn resume_interrupted_session(db_path: String, user_id: Option<i64>) -> Result<Option<Session>, String> {
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = if user_id.is_some() {
        conn.prepare("SELECT id, card_id, start_time, end_time, notes, ai_summary, created_at, updated_at FROM session WHERE end_time IS NULL AND user_id = ? ORDER BY start_time DESC LIMIT 1").map_err(|e| e.to_string())?
    } else {
        conn.prepare("SELECT id, card_id, start_time, end_time, notes, ai_summary, created_at, updated_at FROM session WHERE end_time IS NULL ORDER BY start_time DESC LIMIT 1").map_err(|e| e.to_string())?
    };
    let mut rows = if let Some(uid) = user_id {
        stmt.query(params![uid]).map_err(|e| e.to_string())?
    } else {
        stmt.query([]).map_err(|e| e.to_string())?
    };
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        Ok(Some(Session {
            id: row.get(0).ok(),
            card_id: row.get(1).unwrap_or_default(),
            start_time: row.get(2).unwrap_or_default(),
            end_time: row.get(3).ok(),
            notes: row.get(4).ok(),
            ai_summary: row.get(5).ok(),
            created_at: row.get(6).ok(),
            updated_at: row.get(7).ok(),
        }))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn esp32_auto_reconnect(timeout_ms: Option<u64>) -> Result<String, String> {
    // Try to find and open ESP32 port, return status
    if let Ok(ports) = serialport::available_ports() {
        for port in ports {
            if let SerialPortType::UsbPort(info) = &port.port_type {
                let product = info.product.as_deref().unwrap_or("").to_lowercase();
                let manufacturer = info.manufacturer.as_deref().unwrap_or("").to_lowercase();
                if product.contains("cp210") || product.contains("ch340") || product.contains("esp32") || manufacturer.contains("silicon") || manufacturer.contains("wch") {
                    let timeout = Duration::from_millis(timeout_ms.unwrap_or(1000));
                    match serialport::new(&port.port_name, 115200).timeout(timeout).open() {
                        Ok(_) => return Ok(format!("ESP32 reconnected on port {}", port.port_name)),
                        Err(e) => return Err(format!("Failed to open ESP32 port {}: {}", port.port_name, e)),
                    }
                }
            }
        }
        Err("No ESP32 port found".to_string())
    } else {
        Err("No serial ports found".to_string())
    }
}

// Robust error handling: wrap all Tauri commands in a macro (for future use)
// For now, all commands return Result<T, String> and log errors where relevant.
#[derive(Debug, Serialize, Deserialize)]
pub struct HardwareHealthReport {
    pub esp32_found: bool,
    pub port_name: Option<String>,
    pub serial_read_ok: bool,
    pub last_error: Option<String>,
    pub diagnostics: Option<String>,
}

#[tauri::command]
fn hardware_health_check(timeout_ms: Option<u64>) -> HardwareHealthReport {
    let mut report = HardwareHealthReport {
        esp32_found: false,
        port_name: None,
        serial_read_ok: false,
        last_error: None,
        diagnostics: None,
    };
    // Find ESP32 port
    if let Ok(ports) = serialport::available_ports() {
        for port in ports {
            if let SerialPortType::UsbPort(info) = &port.port_type {
                let product = info.product.as_deref().unwrap_or("").to_lowercase();
                let manufacturer = info.manufacturer.as_deref().unwrap_or("").to_lowercase();
                if product.contains("cp210") || product.contains("ch340") || product.contains("esp32") || manufacturer.contains("silicon") || manufacturer.contains("wch") {
                    report.esp32_found = true;
                    report.port_name = Some(port.port_name.clone());
                    // Try to open and read
                    let timeout = Duration::from_millis(timeout_ms.unwrap_or(1000));
                    match serialport::new(&port.port_name, 115200).timeout(timeout).open() {
                        Ok(mut p) => {
                            let mut buf = [0u8; 64];
                            match p.read(&mut buf) {
                                Ok(_n) => {
                                    report.serial_read_ok = true;
                                    report.diagnostics = Some("Serial read succeeded".to_string());
                                },
                                Err(e) => {
                                    report.last_error = Some(format!("Serial read error: {}", e));
                                }
                            }
                        },
                        Err(e) => {
                            report.last_error = Some(format!("Failed to open port: {}", e));
                        }
                    }
                    break;
                }
            }
        }
    } else {
        report.diagnostics = Some("No serial ports found or error listing ports".to_string());
    }
    report
}
/// Helper: Check if a card of a given type already exists (excluding a given card id for update)
fn card_type_exists(conn: &Connection, type_: &str, exclude_id: Option<i64>) -> Result<bool, String> {
    let mut stmt = if exclude_id.is_some() {
        conn.prepare("SELECT id FROM card WHERE type = ? AND id != ? LIMIT 1").map_err(|e| e.to_string())?
    } else {
        conn.prepare("SELECT id FROM card WHERE type = ? LIMIT 1").map_err(|e| e.to_string())?
    };
    let mut rows = if let Some(id) = exclude_id {
        stmt.query(params![type_, id]).map_err(|e| e.to_string())?
    } else {
        stmt.query(params![type_]).map_err(|e| e.to_string())?
    };
    Ok(rows.next().map_err(|e| e.to_string())?.is_some())
}
use std::sync::Mutex;
use once_cell::sync::Lazy;
use chrono::NaiveTime;

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum CoreCardState {
    Locked,
    Unlocked,
    Pending,
    Error(String),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoreCardStatus {
    pub state: CoreCardState,
    pub last_tap: Option<String>, // ISO8601
    pub last_tap_type: Option<String>, // "wake" or "sleep"
    pub error: Option<String>,
}

// Global state for the core card (per process, not persisted)
static CORE_CARD_STATUS: Lazy<Mutex<CoreCardStatus>> = Lazy::new(|| Mutex::new(CoreCardStatus {
    state: CoreCardState::Locked,
    last_tap: None,
    last_tap_type: None,
    error: None,
}));

/// Helper: Get user-defined time window for core card from DB (returns (start, end) as NaiveTime)
fn get_core_card_time_window(db_path: &str) -> Result<(NaiveTime, NaiveTime), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT config_json FROM workspace_config WHERE id = 1").map_err(|e| e.to_string())?;
    let mut rows = stmt.query([]).map_err(|e| e.to_string())?;
    if let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let config_json: Option<String> = row.get(0).ok();
        if let Some(json) = config_json {
            if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&json) {
                if let (Some(start), Some(end)) = (cfg.get("core_card_window_start"), cfg.get("core_card_window_end")) {
                    let start = NaiveTime::parse_from_str(start.as_str().unwrap_or("06:00"), "%H:%M").unwrap_or(NaiveTime::from_hms_opt(6,0,0).unwrap());
                    let end = NaiveTime::parse_from_str(end.as_str().unwrap_or("23:00"), "%H:%M").unwrap_or(NaiveTime::from_hms_opt(23,0,0).unwrap());
                    return Ok((start, end));
                }
            }
        }
    }
    // Default window: 06:00-23:00
    Ok((NaiveTime::from_hms_opt(6,0,0).unwrap(), NaiveTime::from_hms_opt(23,0,0).unwrap()))
}

/// Tauri command: Handle core card tap (wake/sleep)
#[tauri::command]
fn core_card_tap(db_path: String, rfid: String, tap_type: String) -> Result<CoreCardStatus, String> {
    // Validate card is the only core card
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id FROM card WHERE rfid = ? AND type = 'core'").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![&rfid]).map_err(|e| e.to_string())?;
    if rows.next().map_err(|e| e.to_string())?.is_none() {
        let mut status = CORE_CARD_STATUS.lock().unwrap();
        status.state = CoreCardState::Error("RFID not registered as core card".to_string());
        status.error = Some("RFID not registered as core card".to_string());
        return Err("RFID not registered as core card".to_string());
    };

    // Enforce only one core card
    let mut stmt2 = conn.prepare("SELECT COUNT(*) FROM card WHERE type = 'core'").map_err(|e| e.to_string())?;
    let count: i64 = stmt2.query_row([], |row| row.get(0)).map_err(|e| e.to_string())?;
    if count != 1 {
        let mut status = CORE_CARD_STATUS.lock().unwrap();
        status.state = CoreCardState::Error("There must be exactly one core card".to_string());
        status.error = Some("There must be exactly one core card".to_string());
        return Err("There must be exactly one core card".to_string());
    }

    // Enforce time window
    let (start, end) = get_core_card_time_window(&db_path)?;
    let now = Local::now().time();
    if now < start || now > end {
        let mut status = CORE_CARD_STATUS.lock().unwrap();
        status.state = CoreCardState::Error(format!("Tap outside allowed window: {}-{}", start, end));
        status.error = Some(format!("Tap outside allowed window: {}-{}", start, end));
        return Err(format!("Tap outside allowed window: {}-{}", start, end));
    }

    // State machine logic
    let mut status = CORE_CARD_STATUS.lock().unwrap();
    let prev_state = status.state.clone();
    let now_iso = Local::now().to_rfc3339();
    match tap_type.as_str() {
        "wake" => {
            if prev_state == CoreCardState::Unlocked {
                status.state = CoreCardState::Error("Already unlocked (double wake)".to_string());
                status.error = Some("Already unlocked (double wake)".to_string());
            } else {
                status.state = CoreCardState::Unlocked;
                status.error = None;
            }
            status.last_tap_type = Some("wake".to_string());
            status.last_tap = Some(now_iso);
        },
        "sleep" => {
            if prev_state == CoreCardState::Locked {
                status.state = CoreCardState::Error("Already locked (double sleep)".to_string());
                status.error = Some("Already locked (double sleep)".to_string());
            } else {
                status.state = CoreCardState::Locked;
                status.error = None;
            }
            status.last_tap_type = Some("sleep".to_string());
            status.last_tap = Some(now_iso);
        },
        _ => {
            status.state = CoreCardState::Error("Invalid tap type (must be 'wake' or 'sleep')".to_string());
            status.error = Some("Invalid tap type (must be 'wake' or 'sleep')".to_string());
        }
    }
    // Log transition
    let log_msg = format!("Core card tap: type={}, prev_state={:?}, new_state={:?}, error={:?}", tap_type, prev_state, status.state, status.error);
    let _ = conn.execute("INSERT INTO log (level, message, details_json) VALUES ('info', ?, ?)", params![log_msg, serde_json::to_string(&*status).unwrap_or_default()]);
    Ok(status.clone())
}

/// Tauri command: Get current core card state
#[tauri::command]
fn get_core_card_state() -> CoreCardStatus {
    CORE_CARD_STATUS.lock().unwrap().clone()
}
/// Reassign a card's RFID (used for system card reassignment)
#[tauri::command]
fn reassign_card_rfid(db_path: String, card_id: i64, new_rfid: String) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // Check if new RFID is already assigned to another card
    let mut stmt = conn.prepare("SELECT id FROM card WHERE rfid = ? AND id != ?").map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![&new_rfid, card_id]).map_err(|e| e.to_string())?;
    if let Some(_) = rows.next().map_err(|e| e.to_string())? {
        return Err("RFID is already assigned to another card".to_string());
    }
    // Update the card's RFID
    conn.execute("UPDATE card SET rfid = ?, updated_at = datetime('now') WHERE id = ?", params![&new_rfid, card_id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: Option<i64>,
    pub card_id: i64,
    pub start_time: String,
    pub end_time: Option<String>,
    pub notes: Option<String>,
    pub ai_summary: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[tauri::command]
fn create_session(db_path: String, session: Session) -> Result<i64, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "INSERT INTO session (card_id, start_time, end_time, notes, ai_summary, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))"
    ).map_err(|e| e.to_string())?;
    stmt.execute(params![
        session.card_id,
        session.start_time,
        session.end_time,
        session.notes,
        session.ai_summary
    ]).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn get_sessions(db_path: String) -> Result<Vec<Session>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, card_id, start_time, end_time, notes, ai_summary, created_at, updated_at FROM session").map_err(|e| e.to_string())?;
    let iter = stmt.query_map([], |row| {
        Ok(Session {
            id: row.get(0).ok(),
            card_id: row.get(1).unwrap_or_default(),
            start_time: row.get(2).unwrap_or_default(),
            end_time: row.get(3).ok(),
            notes: row.get(4).ok(),
            ai_summary: row.get(5).ok(),
            created_at: row.get(6).ok(),
            updated_at: row.get(7).ok(),
        })
    }).map_err(|e| e.to_string())?;
    let mut sessions = Vec::new();
    for s in iter {
        sessions.push(s.map_err(|e| e.to_string())?);
    }
    Ok(sessions)
}

#[tauri::command]
fn update_session(db_path: String, session: Session) -> Result<(), String> {
    if session.id.is_none() {
        return Err("Session id is required for update".to_string());
    }
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE session SET card_id = ?, start_time = ?, end_time = ?, notes = ?, ai_summary = ?, updated_at = datetime('now') WHERE id = ?",
        params![
            session.card_id,
            session.start_time,
            session.end_time,
            session.notes,
            session.ai_summary,
            session.id
        ]
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_session(db_path: String, session_id: i64) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM session WHERE id = ?", params![session_id]).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    pub id: Option<i64>,
    pub card_id: Option<i64>,
    pub event_type: String,
    pub event_time: String,
    pub details_json: Option<String>,
    pub created_at: Option<String>,
}

#[tauri::command]
fn create_event(db_path: String, event: Event) -> Result<i64, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare(
        "INSERT INTO event (card_id, event_type, event_time, details_json, created_at) VALUES (?, ?, ?, ?, datetime('now'))"
    ).map_err(|e| e.to_string())?;
    stmt.execute(params![
        event.card_id,
        event.event_type,
        event.event_time,
        event.details_json
    ]).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

#[tauri::command]
fn get_events(db_path: String) -> Result<Vec<Event>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, card_id, event_type, event_time, details_json, created_at FROM event").map_err(|e| e.to_string())?;
    let iter = stmt.query_map([], |row| {
        Ok(Event {
            id: row.get(0).ok(),
            card_id: row.get(1).ok(),
            event_type: row.get(2).unwrap_or_default(),
            event_time: row.get(3).unwrap_or_default(),
            details_json: row.get(4).ok(),
            created_at: row.get(5).ok(),
        })
    }).map_err(|e| e.to_string())?;
    let mut events = Vec::new();
    for e in iter {
        events.push(e.map_err(|e| e.to_string())?);
    }
    Ok(events)
}

#[tauri::command]
fn update_event(db_path: String, event: Event) -> Result<(), String> {
    if event.id.is_none() {
        return Err("Event id is required for update".to_string());
    }
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE event SET card_id = ?, event_type = ?, event_time = ?, details_json = ? WHERE id = ?",
        params![
            event.card_id,
            event.event_type,
            event.event_time,
            event.details_json,
            event.id
        ]
    ).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn delete_event(db_path: String, event_id: i64) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM event WHERE id = ?", params![event_id]).map_err(|e| e.to_string())?;
    Ok(())
}
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct Card {
    pub id: Option<i64>,
    pub rfid: String,
    pub type_: String,
    pub label: Option<String>,
    pub color: Option<String>,
    pub metadata_json: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Create a new card in the database
#[tauri::command]
fn create_card(db_path: String, card: Card) -> Result<i64, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // Enforce only one core, event, and distraction card
    if ["core", "event", "distraction"].contains(&card.type_.as_str()) {
        if card_type_exists(&conn, &card.type_, None)? {
            return Err(format!("A card of type '{}' already exists. Only one is allowed.", card.type_));
        }
    }
    let mut stmt = conn.prepare(
        "INSERT INTO card (rfid, type, label, color, metadata_json, created_at, updated_at) VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))"
    ).map_err(|e| e.to_string())?;
    stmt.execute(params![
        card.rfid,
        card.type_,
        card.label,
        card.color,
        card.metadata_json
    ]).map_err(|e| e.to_string())?;
    Ok(conn.last_insert_rowid())
}

/// Get all cards from the database
#[tauri::command]
fn get_cards(db_path: String) -> Result<Vec<Card>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT id, rfid, type, label, color, metadata_json, created_at, updated_at FROM card").map_err(|e| e.to_string())?;
    let card_iter = stmt.query_map([], |row| {
        Ok(Card {
            id: row.get(0).ok(),
            rfid: row.get(1).unwrap_or_default(),
            type_: row.get(2).unwrap_or_default(),
            label: row.get(3).ok(),
            color: row.get(4).ok(),
            metadata_json: row.get(5).ok(),
            created_at: row.get(6).ok(),
            updated_at: row.get(7).ok(),
        })
    }).map_err(|e| e.to_string())?;
    let mut cards = Vec::new();
    for card in card_iter {
        cards.push(card.map_err(|e| e.to_string())?);
    }
    Ok(cards)
}

/// Update a card in the database
#[tauri::command]
fn update_card(db_path: String, card: Card) -> Result<(), String> {
    if card.id.is_none() {
        return Err("Card id is required for update".to_string());
    }
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    // Enforce only one core, event, and distraction card (excluding this card)
    if ["core", "event", "distraction"].contains(&card.type_.as_str()) {
        if card_type_exists(&conn, &card.type_, card.id)? {
            return Err(format!("A card of type '{}' already exists. Only one is allowed.", card.type_));
        }
    }
    conn.execute(
        "UPDATE card SET rfid = ?, type = ?, label = ?, color = ?, metadata_json = ?, updated_at = datetime('now') WHERE id = ?",
        params![
            card.rfid,
            card.type_,
            card.label,
            card.color,
            card.metadata_json,
            card.id
        ]
    ).map_err(|e| e.to_string())?;
    Ok(())
}

/// Delete a card from the database
#[tauri::command]
fn delete_card(db_path: String, card_id: i64) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    conn.execute("DELETE FROM card WHERE id = ?", params![card_id]).map_err(|e| e.to_string())?;
    Ok(())
}
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use serialport::SerialPortType;
use std::time::Duration;

use rusqlite::{Connection, params};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use std::path::PathBuf;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Initialize or open the daily SQLite database for the given workspace directory.
/// Creates all required tables if not present. Returns the DB path or error.
#[tauri::command]
fn init_daily_database(workspace_dir: String) -> Result<String, String> {
    // Get today's date (YYYY-MM-DD)
    let today = OffsetDateTime::now_utc();
    let date_str = today.format(&Rfc3339).unwrap_or_else(|_| "unknown-date".to_string());
    let date_prefix = &date_str[..10]; // YYYY-MM-DD
    let db_path = PathBuf::from(&workspace_dir).join(format!("focusd_{}.sqlite3", date_prefix));

    // Create workspace dir if missing
    if let Some(parent) = db_path.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                return Err(format!("Failed to create workspace dir: {}", e));
            }
        }
    }

    // Open or create DB
    let conn = match Connection::open(&db_path) {
        Ok(c) => c,
        Err(e) => return Err(format!("Failed to open DB: {}", e)),
    };

    // Create tables (robust schema, extensible, with comments)
    let schema = [
        // User profile (singleton row)
        r#"CREATE TABLE IF NOT EXISTS user_profile (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            name TEXT,
            onboarding_complete INTEGER DEFAULT 0,
            personality_json TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );"#,

        // Card (core, session, event, distraction)
        r#"CREATE TABLE IF NOT EXISTS card (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            rfid TEXT NOT NULL,
            type TEXT NOT NULL CHECK (type IN ('core', 'session', 'event', 'distraction')),
            label TEXT,
            color TEXT,
            metadata_json TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now')),
            UNIQUE(rfid, type)
        );"#,

        // Session (tracks focus sessions)
        r#"CREATE TABLE IF NOT EXISTS session (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER NOT NULL,
            start_time TEXT NOT NULL,
            end_time TEXT,
            notes TEXT,
            ai_summary TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY(card_id) REFERENCES card(id)
        );"#,

        // Event (arbitrary events, e.g. context switches)
        r#"CREATE TABLE IF NOT EXISTS event (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER,
            event_type TEXT NOT NULL,
            event_time TEXT NOT NULL,
            details_json TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY(card_id) REFERENCES card(id)
        );"#,

        // Distraction (tracks interruptions)
        r#"CREATE TABLE IF NOT EXISTS distraction (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            session_id INTEGER,
            event_id INTEGER,
            reason TEXT,
            resolved INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY(session_id) REFERENCES session(id),
            FOREIGN KEY(event_id) REFERENCES event(id)
        );"#,

        // Goal (user goals, can be linked to cards)
        r#"CREATE TABLE IF NOT EXISTS goal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER,
            description TEXT NOT NULL,
            target_date TEXT,
            completed INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY(card_id) REFERENCES card(id)
        );"#,

        // Alarm (reminders, can be linked to cards)
        r#"CREATE TABLE IF NOT EXISTS alarm (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            card_id INTEGER,
            alarm_time TEXT NOT NULL,
            label TEXT,
            triggered INTEGER DEFAULT 0,
            created_at TEXT DEFAULT (datetime('now')),
            FOREIGN KEY(card_id) REFERENCES card(id)
        );"#,

        // Log (arbitrary logs, for debugging/auditing)
        r#"CREATE TABLE IF NOT EXISTS log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            log_time TEXT DEFAULT (datetime('now')),
            level TEXT,
            message TEXT,
            details_json TEXT
        );"#,

        // Workspace config/state (singleton row)
        r#"CREATE TABLE IF NOT EXISTS workspace_config (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            config_json TEXT,
            created_at TEXT DEFAULT (datetime('now')),
            updated_at TEXT DEFAULT (datetime('now'))
        );"#,
    ];

    for stmt in schema.iter() {
        if let Err(e) = conn.execute(stmt, []) {
            return Err(format!("Schema error: {}\nSQL: {}", e, stmt));
        }
    }

    Ok(db_path.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_directory() -> Option<String> {
    // Directory picker stub: replace with a cross-platform dialog if needed
    None
}

#[tauri::command]
fn list_serial_ports() -> Vec<String> {
    match serialport::available_ports() {
        Ok(ports) => ports.into_iter().map(|p| p.port_name).collect(),
        Err(_) => vec![],
    }
}

#[tauri::command]
fn find_esp32_port() -> Option<String> {
    if let Ok(ports) = serialport::available_ports() {
        for port in ports {
            match &port.port_type {
                SerialPortType::UsbPort(info) => {
                    // Heuristic: ESP32 often shows up as "Silicon Labs", "CP210x", "CH340", etc.
                    let product = info.product.as_deref().unwrap_or("").to_lowercase();
                    let manufacturer = info.manufacturer.as_deref().unwrap_or("").to_lowercase();
                    if product.contains("cp210") || product.contains("ch340") || product.contains("esp32") || manufacturer.contains("silicon") || manufacturer.contains("wch") {
                        return Some(port.port_name);
                    }
                }
                _ => {}
            }
        }
    }
    None
}

#[tauri::command]
fn read_esp32_serial(port_name: String, timeout_ms: Option<u64>, db_path: Option<String>) -> Result<String, String> {
    let timeout = Duration::from_millis(timeout_ms.unwrap_or(2000));
    let baud_rate = 115200;
    let max_retries = 3;
    let mut last_err = None;
    for attempt in 1..=max_retries {
        match serialport::new(&port_name, baud_rate)
            .timeout(timeout)
            .open()
        {
            Ok(mut port) => {
                let mut buf = [0u8; 256];
                match port.read(&mut buf) {
                    Ok(n) => {
                        let s = String::from_utf8_lossy(&buf[..n]).to_string();
                        return Ok(s);
                    }
                    Err(e) => {
                        let msg = format!("Read error (attempt {}): {}", attempt, e);
                        last_err = Some(msg.clone());
                        if let Some(ref db) = db_path {
                            let _ = log_hw_error(db, &msg);
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                }
            }
            Err(e) => {
                let msg = format!("Failed to open port (attempt {}): {}", attempt, e);
                last_err = Some(msg.clone());
                if let Some(ref db) = db_path {
                    let _ = log_hw_error(db, &msg);
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
    Err(last_err.unwrap_or_else(|| "Unknown serial error".to_string()))
}

/// Log hardware error to the log table (level=error, message, details_json=null)
fn log_hw_error(db_path: &str, msg: &str) -> Result<(), String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let _ = conn.execute(
        "INSERT INTO log (level, message, details_json) VALUES ('error', ?, NULL)",
        params![msg]
    );
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            // Spawn background task to aggregate all-time stats at startup (non-blocking)
            tauri::async_runtime::spawn(async move {
                // Run aggregation and ignore error, but log if it fails
                if let Err(e) = backend::personality_db::aggregate_all_time_stats_async(None).await {
                    // Try to log via utility if available
                    let _ = backend::utility::log_error("startup_aggregate", &e);
                }
            });
            Ok(())
        })
    .invoke_handler(tauri::generate_handler![
    greet, list_serial_ports, find_esp32_port, read_esp32_serial, pick_directory, init_daily_database,
    create_card, get_cards, update_card, delete_card, reassign_card_rfid,
        create_session, get_sessions, update_session, delete_session,
        create_event, get_events, update_event, delete_event
    , core_card_tap, get_core_card_state
    , hardware_health_check
    , run_db_migrations, resume_interrupted_session, esp32_auto_reconnect
    , get_daily_db_path, read_state_file, write_state_file
    , create_user_profile, update_user_profile, get_user_profile
    , ai_provider::set_provider_api_key, ai_provider::get_provider_api_key, ai_provider::delete_provider_api_key, ai_provider::generate_ai_via_provider
    , ai_provider::store_master_secret_in_keyring, ai_provider::get_master_secret_from_keyring, ai_provider::cache_master_secret_temp, ai_provider::clear_master_secret_cache
    , ai_provider::set_prompt_template, ai_provider::get_prompt_template, ai_provider::list_prompt_templates, ai_provider::generate_journal_entry
    , journals::init_journals_table, journals::save_journal_entry, journals::list_journal_entries, journals::get_journal_entry
    , backend::personality_db::aggregate_all_time_stats_async
    , backend::personality_db::get_profile_and_stats_async
    , backend::dashboard::get_dashboard_summary
    , backend::orchestrator::get_calendar_range
    ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// Public test API (not a Tauri command) to allow unit/integration tests to call internal functions.
pub mod test_api {
    use super::{Card};
    // Re-export thin wrappers without tauri macros so tests can call them directly.
    pub fn create_card(db_path: String, card: Card) -> Result<i64, String> {
        super::create_card(db_path, card)
    }
    pub fn get_cards(db_path: String) -> Result<Vec<Card>, String> {
        super::get_cards(db_path)
    }
    pub fn update_card(db_path: String, card: Card) -> Result<(), String> {
        super::update_card(db_path, card)
    }
    pub fn delete_card(db_path: String, card_id: i64) -> Result<(), String> {
        super::delete_card(db_path, card_id)
    }
    pub fn reassign_card_rfid(db_path: String, card_id: i64, new_rfid: String) -> Result<(), String> {
        super::reassign_card_rfid(db_path, card_id, new_rfid)
    }
    // Session wrappers
    pub fn create_session(db_path: String, session: super::Session) -> Result<i64, String> {
        super::create_session(db_path, session)
    }
    pub fn get_sessions(db_path: String) -> Result<Vec<super::Session>, String> {
        super::get_sessions(db_path)
    }
    pub fn update_session(db_path: String, session: super::Session) -> Result<(), String> {
        super::update_session(db_path, session)
    }
    pub fn delete_session(db_path: String, session_id: i64) -> Result<(), String> {
        super::delete_session(db_path, session_id)
    }
    // Event wrappers
    pub fn create_event(db_path: String, event: super::Event) -> Result<i64, String> {
        super::create_event(db_path, event)
    }
    pub fn get_events(db_path: String) -> Result<Vec<super::Event>, String> {
        super::get_events(db_path)
    }
    pub fn update_event(db_path: String, event: super::Event) -> Result<(), String> {
        super::update_event(db_path, event)
    }
    pub fn delete_event(db_path: String, event_id: i64) -> Result<(), String> {
        super::delete_event(db_path, event_id)
    }
}
