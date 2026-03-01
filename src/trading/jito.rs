use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use reqwest::Client;

pub struct JitoClient {
    client: Client,
    endpoint: String,
    api_key: Option<String>,
}

impl JitoClient {
    pub fn new(endpoint: String, api_key: Option<String>) -> Self {
        JitoClient {
            client: Client::new(),
            endpoint,
            api_key,
        }
    }

    pub async fn send_bundle(&self, tx_bytes: &[u8], tip_sol: f64) -> Result<String> {
        let b64 = general_purpose::STANDARD.encode(tx_bytes);
        let mut body = serde_json::json!({
            "transactions": [b64],
            "tip": tip_sol,
        });
        if let Some(key) = &self.api_key {
            body["apiKey"] = serde_json::Value::String(key.clone());
        }
        let resp = self
            .client
            .post(format!("{}/api/v1/bundles", self.endpoint))
            .json(&body)
            .send()
            .await?
            .text()
            .await?;
        Ok(resp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_dummy() {
        let client = JitoClient::new("https://example.com".to_string(), None);
        let res = client.send_bundle(&[1, 2, 3], 0.0001).await;
        assert!(res.is_err());
    }
}
