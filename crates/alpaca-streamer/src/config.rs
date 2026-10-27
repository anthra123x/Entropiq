/// Centralized credential and environment configuration loader.
///
/// Eliminates duplication between `supervisor.rs` and `live_stream_demo.rs`.
/// Supports process environment variables (highest priority) with `.env` file fallback.
/// Values with surrounding quotes (`"value"` or `'value'`) are stripped automatically.
use std::collections::HashMap;

/// Parsed credentials and runtime configuration.
#[derive(Debug, Clone)]
pub struct EnvConfig {
    pub oauth_token: Option<String>,
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
    pub feed: String,
}

impl EnvConfig {
    /// Loads and validates configuration from environment variables and `.env` fallback.
    /// Does NOT panic or unwrap — all missing values produce `None` or sensible defaults.
    pub fn load() -> Self {
        let file_map = load_dotenv_file();

        let oauth_token = resolve_var("ALPACA_ACCESS_TOKEN", &file_map).filter(|s| !s.is_empty());
        let api_key = resolve_var("ALPACA_API_KEY", &file_map).filter(|s| !s.is_empty());
        let api_secret = resolve_var("ALPACA_API_SECRET", &file_map).filter(|s| !s.is_empty());
        let feed = resolve_var("ALPACA_FEED", &file_map)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "crypto".to_string());

        Self {
            oauth_token,
            api_key,
            api_secret,
            feed,
        }
    }

    /// Returns true if any credential set is available.
    pub fn has_credentials(&self) -> bool {
        self.oauth_token.is_some()
            || (self.api_key.is_some() && self.api_secret.is_some())
    }

    /// Logs which credential mode is active (no secrets are printed).
    pub fn log_auth_mode(&self) {
        if self.oauth_token.is_some() {
            println!("[+] Modo de Autenticacion: Alpaca OAuth Bearer Token (Activo)");
        } else if self.api_key.is_some() && self.api_secret.is_some() {
            println!("[+] Modo de Autenticacion: Alpaca API Key / Secret (Activo)");
        } else {
            println!("[!] Sin credenciales remotas; ejecutando en modo simulador local puro.");
        }
    }
}

/// Reads from process environment first, then falls back to the `.env` map.
fn resolve_var(key: &str, file_map: &HashMap<String, String>) -> Option<String> {
    std::env::var(key)
        .ok()
        .or_else(|| file_map.get(key).cloned())
        .map(|v| strip_quotes(v))
}

/// Parses `.env` file with comment and empty line filtering.
/// Strips surrounding single or double quotes from values (DEF-01 fix: handles `KEY="value"` form).
fn load_dotenv_file() -> HashMap<String, String> {
    let content = match std::fs::read_to_string(".env") {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };

    let mut map = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            map.insert(k.trim().to_string(), strip_quotes(v.trim().to_string()));
        }
    }
    map
}

/// Strips surrounding single or double quotes from env values.
fn strip_quotes(mut s: String) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let first = bytes[0];
        let last = bytes[bytes.len() - 1];
        if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
            s = s[1..s.len() - 1].to_string();
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_quotes() {
        assert_eq!(strip_quotes(r#""my_token""#.to_string()), "my_token");
        assert_eq!(strip_quotes("'my_key'".to_string()), "my_key");
        assert_eq!(strip_quotes("plain".to_string()), "plain");
        assert_eq!(strip_quotes("".to_string()), "");
    }

    #[test]
    fn test_env_config_no_creds() {
        // When no env vars are set, has_credentials must be false
        // (assumes test env does not have these vars set)
        // We test the EnvConfig struct directly
        let cfg = EnvConfig {
            oauth_token: None,
            api_key: None,
            api_secret: None,
            feed: "equities".to_string(),
        };
        assert!(!cfg.has_credentials());
    }

    #[test]
    fn test_env_config_with_key_secret() {
        let cfg = EnvConfig {
            oauth_token: None,
            api_key: Some("test_key".to_string()),
            api_secret: Some("test_secret".to_string()),
            feed: "equities".to_string(),
        };
        assert!(cfg.has_credentials());
    }
}
