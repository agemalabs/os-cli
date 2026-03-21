//! `os token` — show or regenerate MCP token.

use crate::api_client::ApiClient;
use anyhow::Result;

/// Show or regenerate the user's MCP token.
pub async fn run(client: &ApiClient, regenerate: bool) -> Result<()> {
    if regenerate {
        let resp: serde_json::Value = client
            .post("/mcp/token/regenerate", &serde_json::json!({}))
            .await?;
        let token = resp["data"]["token"].as_str().unwrap_or("?");
        println!("New MCP token: {}", token);
        println!("\nUpdate your Claude Desktop config with this token.");
    } else {
        let resp: serde_json::Value = client.get("/mcp/token").await?;
        let token = resp["data"]["token"].as_str().unwrap_or("?");
        println!("MCP token: {}", token);
        println!("\nUse this in Claude Desktop or any MCP-compatible tool:");
        println!("  Authorization: Bearer {}", token);
    }

    Ok(())
}
