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

    /// PUT JSON to an endpoint.
    pub async fn put<T: DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .put(&url)
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

    /// DELETE an endpoint.
    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("API error {}: {}", status, body);
        }

        Ok(())
    }

    /// POST /chat — contextual AI chat.
    pub async fn chat(
        &self,
        question: &str,
        project_slug: Option<&str>,
        lead_id: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut body = serde_json::json!({ "question": question });
        if let Some(slug) = project_slug {
            body["project_slug"] = serde_json::json!(slug);
        }
        if let Some(id) = lead_id {
            body["lead_id"] = serde_json::json!(id);
        }
        self.post("/chat", &body).await
    }

    /// POST /leads/:id/notes — add a note to a lead.
    pub async fn add_lead_note(
        &self,
        lead_id: &str,
        content: &str,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({ "content": content });
        self.post(&format!("/leads/{}/notes", lead_id), &body).await
    }

    /// PUT /leads/:id — update a lead.
    pub async fn update_lead(
        &self,
        lead_id: &str,
        updates: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.put(&format!("/leads/{}", lead_id), updates).await
    }

    /// DELETE /leads/:id — soft-delete a lead.
    pub async fn delete_lead(&self, lead_id: &str) -> Result<()> {
        self.delete(&format!("/leads/{}", lead_id)).await
    }

    /// GET /leads/:id/contacts — list contacts for a lead.
    #[allow(dead_code)]
    pub async fn get_lead_contacts(&self, lead_id: &str) -> Result<serde_json::Value> {
        self.get(&format!("/leads/{}/contacts", lead_id)).await
    }

    /// POST /leads/:id/contacts — add a contact to a lead.
    pub async fn add_lead_contact(
        &self,
        lead_id: &str,
        contact: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.post(&format!("/leads/{}/contacts", lead_id), contact)
            .await
    }

    /// POST /projects/:slug/decisions — create a decision.
    pub async fn create_decision(
        &self,
        project_slug: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut body = serde_json::json!({ "title": title });
        if let Some(desc) = description {
            body["description"] = serde_json::json!(desc);
        }
        self.post(&format!("/projects/{}/decisions", project_slug), &body)
            .await
    }

    /// PUT /projects/:slug/tasks/:id — update a task.
    pub async fn update_task(
        &self,
        project_slug: &str,
        task_id: &str,
        updates: &serde_json::Value,
    ) -> Result<serde_json::Value> {
        self.put(
            &format!("/projects/{}/tasks/{}", project_slug, task_id),
            updates,
        )
        .await
    }

    /// PUT /projects/:slug/decisions/:id/resolve — resolve a decision.
    pub async fn resolve_decision(
        &self,
        project_slug: &str,
        decision_id: &str,
        resolution: &str,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({ "resolution": resolution });
        self.put(
            &format!("/projects/{}/decisions/{}/resolve", project_slug, decision_id),
            &body,
        )
        .await
    }

    /// POST /projects — create a project.
    pub async fn create_project(
        &self,
        name: &str,
        slug: &str,
        description: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut body = serde_json::json!({ "name": name, "slug": slug });
        if let Some(desc) = description {
            body["description"] = serde_json::json!(desc);
        }
        self.post("/projects", &body).await
    }

    /// `POST /projects/:slug/repos` — link a GitHub repo to a project.
    pub async fn link_repo(
        &self,
        project_slug: &str,
        github_repo: &str,
        label: Option<&str>,
    ) -> Result<serde_json::Value> {
        let mut body = serde_json::json!({ "github_repo": github_repo });
        if let Some(l) = label {
            body["label"] = serde_json::json!(l);
        }
        self.post(&format!("/projects/{}/repos", project_slug), &body)
            .await
    }

    /// `POST /projects/:slug/team` — add a team member by email.
    pub async fn add_team_member(
        &self,
        project_slug: &str,
        email: &str,
        role: &str,
    ) -> Result<serde_json::Value> {
        let body = serde_json::json!({ "email": email, "role": role });
        self.post(&format!("/projects/{}/team", project_slug), &body)
            .await
    }

    /// `DELETE /projects/:slug/team/:user_id` — remove a team member.
    #[allow(dead_code)]
    pub async fn remove_team_member(
        &self,
        project_slug: &str,
        user_id: &str,
    ) -> Result<()> {
        self.delete(&format!("/projects/{}/team/{}", project_slug, user_id))
            .await
    }

    /// `GET /projects/:slug/repos` — list linked repos for a project.
    pub async fn list_repos(&self, project_slug: &str) -> Result<serde_json::Value> {
        self.get(&format!("/projects/{}/repos", project_slug))
            .await
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
