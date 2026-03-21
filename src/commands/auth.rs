//! CLI authentication — OAuth flow (GitHub or Google).
//!
//! 1. Start a temporary local HTTP server on a random port
//! 2. Open browser to API's OAuth start with cli_port param
//! 3. API handles OAuth, redirects token to CLI's local server
//! 4. CLI captures token, writes config, done

use anyhow::Result;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::config;

/// OAuth provider.
pub enum Provider {
    GitHub,
    Google,
}

impl Provider {
    fn path(&self) -> &str {
        match self {
            Provider::GitHub => "/auth/github/start",
            Provider::Google => "/auth/google/start",
        }
    }

    fn name(&self) -> &str {
        match self {
            Provider::GitHub => "GitHub",
            Provider::Google => "Google",
        }
    }
}

/// Run the OAuth login flow with the given provider.
pub async fn login(api_url: &str, provider: Provider) -> Result<()> {
    println!("Authenticating with {}...\n", provider.name());

    // Start a temporary local server to receive the callback
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    // Open browser to the API's OAuth start, passing our local port
    let auth_url = format!("{}{}?cli_port={}", api_url, provider.path(), port);
    println!("Opening browser to authenticate...");
    println!("If the browser doesn't open, visit:\n  {}\n", auth_url);

    if open::that(&auth_url).is_err() {
        println!("Could not open browser automatically.");
    }

    println!("Waiting for authentication...");

    // Accept one connection — the redirect from the API
    let (mut stream, _) = listener.accept().await?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse token from the request: GET /callback?token=xxx HTTP/1.1
    let token = parse_token_from_request(&request);

    match token {
        Some(token) if !token.is_empty() => {
            // Send success response to browser
            let html = "<!DOCTYPE html><html><body><h2>Authenticated!</h2><p>You can close this tab and return to the terminal.</p></body></html>";
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            stream.write_all(response.as_bytes()).await?;

            // Save to config
            let mut cfg = config::load_or_init()?;
            cfg.token = token;
            cfg.api_url = api_url.to_string();
            config::save(&cfg)?;

            println!("\nAuthenticated successfully!");
            println!("Config saved to {}", config::config_path().display());
        }
        _ => {
            let html = "<!DOCTYPE html><html><body><h2>Authentication failed</h2><p>No token received. Try again.</p></body></html>";
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                html.len(),
                html
            );
            stream.write_all(response.as_bytes()).await?;

            anyhow::bail!("Authentication failed — no token received");
        }
    }

    Ok(())
}

/// Extract token from an HTTP request line.
fn parse_token_from_request(request: &str) -> Option<String> {
    request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|path| {
            path.split('?').nth(1).and_then(|qs| {
                qs.split('&')
                    .find(|p| p.starts_with("token="))
                    .map(|p| p.strip_prefix("token=").unwrap_or("").to_string())
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_token_from_query_string() {
        let req = "GET /callback?token=abc-123-def HTTP/1.1\r\nHost: localhost";
        assert_eq!(
            parse_token_from_request(req),
            Some("abc-123-def".to_string())
        );
    }

    #[test]
    fn parse_token_with_extra_params() {
        let req = "GET /callback?other=x&token=my-token&foo=bar HTTP/1.1";
        assert_eq!(parse_token_from_request(req), Some("my-token".to_string()));
    }

    #[test]
    fn parse_token_missing() {
        let req = "GET /callback?other=value HTTP/1.1";
        assert!(parse_token_from_request(req).is_none());
    }

    #[test]
    fn provider_paths() {
        assert_eq!(Provider::GitHub.path(), "/auth/github/start");
        assert_eq!(Provider::Google.path(), "/auth/google/start");
    }
}
