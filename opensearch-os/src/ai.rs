//! Minimal AI client for OpenSearch OS — talks to any OpenAI-compatible
//! chat-completions endpoint (DeepSeek by default). Blocking (ureq), runs on a
//! worker thread so the UI never stalls.

use anyhow::{anyhow, Result};

// DeepSeek V4 Flash (OpenAI-compatible). Override endpoint/model via env if desired.
const DEFAULT_ENDPOINT: &str = "https://api.deepseek.com/chat/completions";
const DEFAULT_MODEL: &str = "deepseek-chat";

// ── API key resolution ────────────────────────────────────────────────────────
// Order: env var → %APPDATA%/opensearch-os/ai_key.txt → hardcoded constant below.
// Leave the constant empty in source (never commit a real key); the user pastes
// their DeepSeek key into the file or env var.
const HARDCODED_KEY: &str = "";

pub fn api_key() -> Option<String> {
    if let Ok(k) = std::env::var("DEEPSEEK_API_KEY") {
        if !k.trim().is_empty() { return Some(k.trim().to_string()); }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = std::path::Path::new(&appdata).join("opensearch-os").join("ai_key.txt");
        if let Ok(s) = std::fs::read_to_string(&p) {
            let k = s.trim().to_string();
            if !k.is_empty() { return Some(k); }
        }
    }
    if !HARDCODED_KEY.is_empty() { return Some(HARDCODED_KEY.to_string()); }
    None
}

fn endpoint() -> String {
    std::env::var("OPENSEARCH_AI_ENDPOINT").unwrap_or_else(|_| DEFAULT_ENDPOINT.to_string())
}
fn model() -> String {
    std::env::var("OPENSEARCH_AI_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string())
}

/// One-shot chat completion (non-streaming). Returns the assistant's text.
pub fn complete(system: &str, user: &str) -> Result<String> {
    let key = api_key().ok_or_else(|| anyhow!(
        "No API key. Set DEEPSEEK_API_KEY or put your key in %APPDATA%/opensearch-os/ai_key.txt"
    ))?;

    let body = serde_json::json!({
        "model": model(),
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "stream": false,
        "temperature": 0.3,
    });

    let resp = ureq::post(&endpoint())
        .set("Authorization", &format!("Bearer {}", key))
        .set("Content-Type", "application/json")
        .timeout(std::time::Duration::from_secs(60))
        .send_json(body);

    let resp = match resp {
        Ok(r) => r,
        Err(ureq::Error::Status(code, r)) => {
            let msg = r.into_string().unwrap_or_default();
            return Err(anyhow!("AI error {code}: {}", msg.chars().take(300).collect::<String>()));
        }
        Err(e) => return Err(anyhow!("AI request failed: {e}")),
    };

    let v: serde_json::Value = resp.into_json().map_err(|e| anyhow!("bad AI response: {e}"))?;
    let text = v["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| anyhow!("AI response had no content"))?;
    Ok(text.trim().to_string())
}

/// Map a command + input to a (system prompt, user content) and run it.
/// Commands: ask, explain, grammar, translate, summarize.
pub fn run(cmd: &str, input: &str) -> Result<String> {
    let input = input.trim();
    if input.is_empty() {
        return Err(anyhow!("Nothing to send — type text or copy something first."));
    }
    let (system, user): (&str, String) = match cmd {
        "ask" | "chat" => (
            "You are a concise, helpful assistant. Answer directly in at most a few short paragraphs.",
            input.to_string(),
        ),
        "explain" => (
            "Explain the following clearly and simply for a general audience. Be concise.",
            input.to_string(),
        ),
        "grammar" => (
            "Fix the spelling and grammar of the text. Output ONLY the corrected text, with no preamble or quotes.",
            input.to_string(),
        ),
        "translate" => (
            "You are a translator. If the input names a target language (e.g. 'X to Spanish'), translate X into it; otherwise translate the text to English. Output ONLY the translation.",
            input.to_string(),
        ),
        "summarize" => (
            "Summarize the following text concisely as a few short bullet points.",
            input.to_string(),
        ),
        "bugs" => (
            "You are a code reviewer. List likely bugs and issues in the following code as short bullet points. Be specific.",
            input.to_string(),
        ),
        _ => (
            "You are a concise, helpful assistant.",
            input.to_string(),
        ),
    };
    complete(system, &user)
}
