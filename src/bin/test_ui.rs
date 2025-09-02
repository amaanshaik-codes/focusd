use rusqlite::{Connection, params};
use tokio::runtime::Runtime;
use std::env;
use focusd_lib::ai_provider::{store_master_secret_in_keyring, call_chatgpt, call_gemini};

fn main() {
    println!("Starting test UI harness...");
    let rt = Runtime::new().expect("failed to start tokio runtime");

    // Ensure DB schema exists and seed a test user and template
    if let Ok(conn) = Connection::open("focusd_personality.db") {
        let _ = conn.execute("CREATE TABLE IF NOT EXISTS user (id INTEGER PRIMARY KEY, ai_opt_in BOOLEAN, ai_provider TEXT, ai_api_key TEXT)", []);
        let _ = conn.execute("CREATE TABLE IF NOT EXISTS prompt_templates (user_id INTEGER, name TEXT, template TEXT, PRIMARY KEY(user_id, name))", []);
        let _ = conn.execute("CREATE TABLE IF NOT EXISTS api_keys (user_id INTEGER, provider TEXT, key_enc TEXT, PRIMARY KEY(user_id, provider))", []);
        // Insert test user if not exists
        let mut stmt = conn.prepare("SELECT id FROM user WHERE id = ?").unwrap();
        let exists = stmt.exists(params![1]).unwrap_or(false);
        if !exists {
            let _ = conn.execute("INSERT INTO user (id, ai_opt_in, ai_provider) VALUES (?, ?, ?)", params![1, true, "chatgpt"]);
        }
        // Insert a default prompt template
        let mut ts = conn.prepare("SELECT template FROM prompt_templates WHERE user_id = ? AND name = ?").unwrap();
        let texists = ts.exists(params![1, "default"]).unwrap_or(false);
        if !texists {
            let _ = conn.execute("INSERT OR REPLACE INTO prompt_templates (user_id, name, template) VALUES (?, ?, ?)", params![1, "default", "{{prompt}}\n-- by focusd for user {{user_id}}"]);
        }
    }

    rt.block_on(async {
        // 1) store master secret in keyring (best-effort) - optional, won't be used for provider calls below
        match store_master_secret_in_keyring("test_master".to_string(), "s3cr3t".to_string()).await {
            Ok(_) => println!("store_master_secret_in_keyring: OK"),
            Err(e) => println!("store_master_secret_in_keyring: ERR: {}", e),
        }

        // Read API keys from environment (do NOT store them)
        let openai_key = env::var("OPENAI_KEY").ok();
        let gemini_key = env::var("GEMINI_KEY").ok();

        // Call OpenAI-compatible endpoint if key present
        if let Some(key) = openai_key {
            println!("Calling OpenAI-compatible provider (no key will be stored)");
            let prompt = "Write a one-line friendly summary of today's accomplishments.";
            match call_chatgpt(&key, prompt, 10, None).await {
                Ok(resp) => println!("OpenAI response (truncated 200 chars): {}", &resp.chars().take(200).collect::<String>()),
                Err(e) => println!("OpenAI call failed: {}", e),
            }
        } else {
            println!("OPENAI_KEY not provided; skipping OpenAI test");
        }

        // Call Gemini if key present
        if let Some(key) = gemini_key {
            println!("Calling Gemini provider (no key will be stored)");
            let prompt = "Write a one-line friendly summary of today's accomplishments.";
            match call_gemini(&key, prompt, 10, None).await {
                Ok(resp) => println!("Gemini response (truncated 200 chars): {}", &resp.chars().take(200).collect::<String>()),
                Err(e) => println!("Gemini call failed: {}", e),
            }
        } else {
            println!("GEMINI_KEY not provided; skipping Gemini test");
        }

        println!("Test UI harness finished (keys not stored)." );
    });
}
