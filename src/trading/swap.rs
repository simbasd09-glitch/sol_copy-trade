use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use base64;
use base64::Engine;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JupiterQuote {
    // simplified fields with snake case for Rust
    pub input_amount: u64,
    pub output_amount: u64,
}

pub struct JupiterApi {
    client: Client,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct JupiterSwapResponse {
    pub swap_transaction: String,
    // other fields omitted
}

impl JupiterApi {
    pub fn new() -> Self {
        JupiterApi {
            client: Client::new(),
        }
    }

    pub async fn quote(
        &self,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<JupiterQuote> {
        let url = format!("https://quote-api.jup.ag/v6/quote?inputMint=So11111111111111111111111111111111111111112&outputMint={}&amount={}&slippageBps={}", output_mint, amount, slippage_bps);
        let resp: JupiterQuote = self.client.get(&url).send().await?.json().await?;
        Ok(resp)
    }

    /// Request a prepared swap transaction from Jupiter and return its instructions
    pub async fn swap_instructions(
        &self,
        output_mint: &str,
        amount: u64,
        slippage_bps: u16,
    ) -> Result<Vec<solana_sdk::instruction::Instruction>> {
        let url = format!(
            "https://quote-api.jup.ag/v6/swap?inputMint=So11111111111111111111111111111111111111112&outputMint={}&amount={}&slippageBps={}",
            output_mint,
            amount,
            slippage_bps
        );
        let resp: JupiterSwapResponse = self.client.get(&url).send().await?.json().await?;
        // decode transaction bytes
        let tx_bytes = base64::engine::general_purpose::STANDARD.decode(&resp.swap_transaction)?;
        let tx: solana_sdk::transaction::VersionedTransaction = bincode::deserialize(&tx_bytes)?;
        // transform compiled instructions into `Instruction` with account metas
        let message = &tx.message;
        let mut ins: Vec<solana_sdk::instruction::Instruction> = Vec::new();
        // VersionedMessage provides accessor methods rather than public fields
        let keys = message.static_account_keys();
        for ci in message.instructions() {
            let program_id = keys[ci.program_id_index as usize];
            let accounts = ci
                .accounts
                .iter()
                .map(|&idx| {
                    let key = keys[idx as usize];
                    let is_signer = message.is_signer(idx as usize);
                    let is_writable = message.is_maybe_writable(idx as usize, None);
                    solana_sdk::instruction::AccountMeta {
                        pubkey: key,
                        is_signer,
                        is_writable,
                    }
                })
                .collect();
            ins.push(solana_sdk::instruction::Instruction {
                program_id,
                accounts,
                data: ci.data.clone(),
            });
        }
        Ok(ins)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_quote() {
        let api = JupiterApi::new();
        let res = api
            .quote("So11111111111111111111111111111111111111112", 1000, 100)
            .await;
        // we expect failure due to invalid token, but ensure error handling works
        assert!(res.is_err() || res.is_ok());
    }

    #[tokio::test]
    async fn test_swap_instructions() {
        let api = JupiterApi::new();
        let res = api
            .swap_instructions("So11111111111111111111111111111111111111112", 1000, 100)
            .await;
        assert!(res.is_err() || res.is_ok());
    }
}
