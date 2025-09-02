use regex::Regex;

/// Redact obvious PII patterns from text. This is a conservative first-pass.
pub fn redact_pii(input: &str) -> String {
    let mut out = input.to_string();
    // emails
    let email_re = Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap();
    out = email_re.replace_all(&out, "[REDACTED_EMAIL]").to_string();
    // phone numbers (simple)
    let phone_re = Regex::new(r"(?m)(?:\+?\d[\d .-]{7,}\d)").unwrap();
    out = phone_re.replace_all(&out, "[REDACTED_PHONE]").to_string();
    // api keys like strings (long alphanumeric)
    let apikey_re = Regex::new(r"(?i)\b(?:sk|api|token|key)[-_ ]?[A-Za-z0-9]{20,}\b").unwrap();
    out = apikey_re.replace_all(&out, "[REDACTED_KEY]").to_string();
    out
}

/// Basic policy check: returns Ok(()) if safe, Err(reason) if violation found.
pub fn policy_check(input: &str) -> Result<(), String> {
    // very small profanity list for demo
    let profane = ["shit", "fuck", "bitch"];
    let lower = input.to_lowercase();
    for p in &profane {
        if lower.contains(p) {
            return Err(format!("policy_violation: found disallowed word: {}", p));
        }
    }
    // if too long (excessively large) consider it suspicious
    if input.len() > 10000 {
        return Err("policy_violation: content too large".to_string());
    }
    Ok(())
}
