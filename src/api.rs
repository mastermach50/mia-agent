use log::{debug, trace};
use reqwest::{Client, header::{AUTHORIZATION, REFERER, HeaderMap}};
use anyhow::{Context, Ok, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::config::AppConfig;

#[derive(Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub reasoning: Option<String>,
    pub content: String
}

pub async fn completion(messages: &Vec<Message>) -> Result<Message> {
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        format!("Bearer {}", std::env::var("OPENROUTER_API_KEY").unwrap()).parse().unwrap()
    );
    headers.insert(
        REFERER,
        "https://mathewma3.in".parse().unwrap()
    );
    headers.insert(
        "X-OpenRouter-Title",
        "Mia Agent".parse().unwrap()
    );

    let client = Client::builder()
        .default_headers(headers)
        .build()?;

    let payload = json!({
        "messages": messages,
        "model": AppConfig::global().model.name,
        "reasoning": {
            "effort": AppConfig::global().model.reasoning
        }
    });

    let response = client.post("https://openrouter.ai/api/v1/chat/completions")
        .json(&payload)
        .send()
        .await
        .context("Failed to parse API response")?;
    debug!("Response Status: {}", response.status());

    if !response.status().is_success() {
        anyhow::bail!("API request failed with status {}", response.status());
    }

    let content = response.json::<serde_json::Value>()
        .await?;

    trace!("Response Content: {:?}", content);
    let message: Message = serde_json::from_value(content["choices"][0]["message"].clone())
        .context("Failed to decode message")?;
    Ok(message)
}

