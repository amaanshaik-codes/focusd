use tokio::runtime::Runtime;
use std::env;
use focusd_lib::ai_provider::call_gemini;

fn main() {
    println!("Starting Gemini-only test harness...");
    let rt = Runtime::new().expect("failed to start tokio runtime");
    rt.block_on(async {
        let gemini_key = env::var("GEMINI_KEY").ok();
        if let Some(key) = gemini_key {
            println!("Calling Gemini provider (key passed via env)");
            let prompt = "Write a one-line friendly summary of today's accomplishments.";
            match call_gemini(&key, prompt, 10, None).await {
                Ok(resp) => println!("Gemini response (truncated 400 chars): {}", &resp.chars().take(400).collect::<String>()),
                Err(e) => println!("Gemini call failed: {}", e),
            }
        } else {
            println!("GEMINI_KEY not provided; aborting test.");
        }
        println!("Gemini test harness finished (key not stored).");
    });
}
