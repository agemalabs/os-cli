//! `os user` — user management commands (list, invite, role, remove).

use crate::api_client::ApiClient;
use anyhow::Result;

/// List all users in the organization.
pub async fn list(client: &ApiClient) -> Result<()> {
    let resp: serde_json::Value = client.get("/users").await?;
    let users = resp["data"].as_array();

    match users {
        Some(users) if !users.is_empty() => {
            println!(
                "\n{:<28} {:<16} {:<8} {}",
                "EMAIL", "NAME", "ROLE", "CREATED"
            );
            for user in users {
                let email = user["email"].as_str().unwrap_or("?");
                let name = user["name"].as_str().unwrap_or("-");
                let role = user["role"].as_str().unwrap_or("?");
                let created = user["created_at"].as_str().unwrap_or("?");
                println!("{:<28} {:<16} {:<8} {}", email, name, role, created);
            }
            println!();
        }
        _ => {
            println!("No users found.");
        }
    }

    Ok(())
}

/// Invite a new user by email.
pub async fn invite(
    client: &ApiClient,
    email: &str,
    name: Option<&str>,
    role: &str,
) -> Result<()> {
    let mut body = serde_json::json!({
        "email": email,
        "role": role,
    });

    if let Some(name) = name {
        body["name"] = serde_json::json!(name);
    }

    let _resp: serde_json::Value = client.post("/users/invite", &body).await?;

    println!("Invited {} as {}", email, role);
    println!("They can now log in via Google or GitHub.");

    Ok(())
}

/// Change a user's role.
pub async fn role(client: &ApiClient, email: &str, new_role: &str) -> Result<()> {
    // First, find the user by email to get their ID
    let resp: serde_json::Value = client.get("/users").await?;
    let users = resp["data"].as_array().ok_or_else(|| {
        anyhow::anyhow!("Unexpected response format")
    })?;

    let user = users
        .iter()
        .find(|u| u["email"].as_str() == Some(email))
        .ok_or_else(|| anyhow::anyhow!("User not found: {}", email))?;

    let user_id = user["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing user id"))?;

    let body = serde_json::json!({ "role": new_role });
    let _resp: serde_json::Value = client
        .put(&format!("/users/{}", user_id), &body)
        .await?;

    println!("Updated {} to role: {}", email, new_role);

    Ok(())
}

/// Remove a user.
pub async fn remove(client: &ApiClient, email: &str) -> Result<()> {
    // First, find the user by email to get their ID
    let resp: serde_json::Value = client.get("/users").await?;
    let users = resp["data"].as_array().ok_or_else(|| {
        anyhow::anyhow!("Unexpected response format")
    })?;

    let user = users
        .iter()
        .find(|u| u["email"].as_str() == Some(email))
        .ok_or_else(|| anyhow::anyhow!("User not found: {}", email))?;

    let user_id = user["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing user id"))?;

    client.delete(&format!("/users/{}", user_id)).await?;

    println!("Removed user: {}", email);

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn format_user_row() {
        let line = format!("{:<28} {:<16} {:<8} {}", "a@b.com", "Alice", "admin", "2026-03-21");
        assert!(line.contains("a@b.com"));
        assert!(line.contains("Alice"));
        assert!(line.contains("admin"));
    }
}
