use solana_sdk::{
    instruction::Instruction,
    message::Message,
    signature::{Keypair, Signer},
    transaction::Transaction,
    hash::Hash,
    pubkey::Pubkey,
};
use anyhow::Result;
use tracing::debug;

pub struct TransactionBuilder {
    wallet: Keypair,
    priority_fee_lamports: u64,
    compute_units: u32,
}

impl TransactionBuilder {
    pub fn new(wallet: Keypair) -> Self {
        Self {
            wallet,
            priority_fee_lamports: 10000, // 0.00001 SOL default
            compute_units: 200_000,
        }
    }
    
    pub fn set_priority_fee(&mut self, fee_lamports: u64) {
        self.priority_fee_lamports = fee_lamports;
    }
    
    pub fn set_compute_units(&mut self, units: u32) {
        self.compute_units = units;
    }
    
    pub fn build_transaction(
        &self,
        instructions: Vec<Instruction>,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        // build instruction list
        let mut all_instructions = Vec::new();
        all_instructions.extend(instructions);
        
        // Create message
        let message = Message::new(&all_instructions, Some(&self.wallet.pubkey()));
        
        // Create and sign transaction
        let mut tx = Transaction::new_unsigned(message);
        tx.sign(&[&self.wallet], recent_blockhash);
        
        debug!(
            "Built transaction with {} instructions, fee payer: {}",
            all_instructions.len(),
            self.wallet.pubkey()
        );
        
        Ok(tx)
    }
    
    pub fn build_swap_transaction(
        &self,
        swap_instructions: Vec<Instruction>,
        recent_blockhash: Hash,
    ) -> Result<Transaction> {
        self.build_transaction(swap_instructions, recent_blockhash)
    }
    
    pub fn sign_transaction(&self, mut tx: Transaction, recent_blockhash: Hash) -> Transaction {
        tx.sign(&[&self.wallet], recent_blockhash);
        tx
    }
    
    pub fn wallet_pubkey(&self) -> Pubkey {
        self.wallet.pubkey()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_sdk::{instruction::Instruction, hash::Hash};
    use solana_sdk::system_program;

    #[test]
    fn build_transaction_includes_compute_budget() {
        let key = Keypair::new();
        let builder = TransactionBuilder::new(key);
        let dummy = Instruction::new_with_bytes(system_program::id(), &[], vec![]);
        let recent = Hash::default();
        let tx = builder.build_transaction(vec![dummy.clone()], recent).unwrap();
        // compute budget instructions + our dummy
        assert!(tx.message.instructions.len() >= 3);
    }
}

