use super::anvil_simlator::AnvilSimulator;
use anyhow::Result;
use ethers::providers::Middleware; // Import the struct from the parent module

impl AnvilSimulator {
    pub async fn take_snapshot(&self) -> Result<String> {
        let snapshot_id: String = self.client.provider().request("evm_snapshot", ()).await?;
        Ok(snapshot_id)
    }

    pub async fn revert_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let success: bool = self
            .client
            .provider()
            .request("evm_revert", [snapshot_id])
            .await?;
        if !success {
            return Err(anyhow::anyhow!("Failed to revert snapshot"));
        }
        Ok(())
    }
}
