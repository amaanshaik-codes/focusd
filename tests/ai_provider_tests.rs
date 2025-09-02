use serde_json::json;

#[test]
fn test_encrypt_decrypt_roundtrip() {
    // use crate-level helpers (now public)
    let master = "test-master";
    let api_key = "sk-test-123";
    let enc = focusd_lib::encrypt_api_key(api_key, master);
    let dec = focusd_lib::decrypt_api_key(&enc, master).expect("decrypt failed");
    assert_eq!(dec, api_key);
}

#[test]
fn test_parse_chatgpt_response_variants() {
    // variant with choices[0].message.content
    let j = json!({
        "choices": [ { "message": { "content": "Hello from chatgpt" } } ]
    });
    let out = focusd_lib::ai_provider::parse_chatgpt_response(&j);
    assert_eq!(out, "Hello from chatgpt");

    // variant with choices[0].text
    let j2 = json!({ "choices": [ { "text": "Legacy text" } ] });
    let out2 = focusd_lib::ai_provider::parse_chatgpt_response(&j2);
    assert_eq!(out2, "Legacy text");
}

#[test]
fn test_parse_gemini_response() {
    let j = json!({ "candidates": [ { "content": "Gemini says hi" } ] });
    let out = focusd_lib::ai_provider::parse_gemini_response(&j);
    assert_eq!(out, "Gemini says hi");
}
