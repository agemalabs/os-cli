//! HTTP client for the OS API.

use anyhow::Result;
use serde::de::DeserializeOwned;

/// Client for making authenticated requests to the OS API.
#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    token: String,
    http: reqwest::Client,
}

impl ApiClient {
    /// Create a new API client.
    pub fn new(base_url: &str, token: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token: token.to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// GET a JSON endpoint.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        Ok(resp.json().await?)
    }

    /// POST JSON to an endpoint.
    pub async fn post<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .json(body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        Ok(resp.json().await?)
    }

    /// Upload a file via multipart POST.
    pub async fn upload_file(
        &self,
        path: &str,
        filename: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> Result<serde_json::Value> {
        let url = format!("{}{}", self.base_url, path);
        let part = reqwest::multipart::Part::bytes(data)
            .file_name(filename.to_string())
            .mime_str(content_type)?;
        let form = reqwest::multipart::Form::new().part("file", part);

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .multipart(form)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        Ok(resp.json().await?)
    }

    /// Check if the client has a token configured.
    #[allow(dead_code)]
    pub fn is_authenticated(&self) -> bool {
        !self.token.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_strips_trailing_slash() {
        let client = ApiClient::new("http://localhost:8080/", "token");
        assert_eq!(client.base_url, "http://localhost:8080");
    }

    #[test]
    fn is_authenticated_checks_token() {
        let authed = ApiClient::new("http://localhost:8080", "my-token");
        assert!(authed.is_authenticated());

        let unauthed = ApiClient::new("http://localhost:8080", "");
        assert!(!unauthed.is_authenticated());
    }
}
