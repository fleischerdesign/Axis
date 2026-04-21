use axis_domain::models::cloud::{CloudStatus, CloudAccount, AccountStatus};
use axis_domain::ports::cloud::{CloudProvider, CloudError, CloudStream};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::watch;
use std::path::PathBuf;
use tokio_stream::wrappers::WatchStream;

pub struct LocalCloudProvider {
    status_tx: watch::Sender<CloudStatus>,
    config_dir: PathBuf,
}

impl LocalCloudProvider {
    pub fn new(config_dir: PathBuf) -> Self {
        let accounts_path = config_dir.join("cloud_accounts.json");
        let accounts = std::fs::read_to_string(accounts_path)
            .ok()
            .and_then(|json| serde_json::from_str::<Vec<CloudAccount>>(&json).ok())
            .unwrap_or_default();

        let (status_tx, _) = watch::channel(CloudStatus { accounts });

        Self {
            status_tx,
            config_dir,
        }
    }

    pub fn add_account(&self, account: CloudAccount) {
        let mut status = self.status_tx.borrow().clone();
        status.accounts.retain(|a| a.id != account.id);
        status.accounts.push(account);
        let _ = self.status_tx.send(status.clone());
        self.save_accounts(&status.accounts);
    }

    fn save_accounts(&self, accounts: &[CloudAccount]) {
        let accounts_path = self.config_dir.join("cloud_accounts.json");
        if let Ok(json) = serde_json::to_string_pretty(accounts) {
            let _ = std::fs::write(accounts_path, json);
        }
    }
}

#[async_trait]
impl CloudProvider for LocalCloudProvider {
    async fn get_status(&self) -> Result<CloudStatus, CloudError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<CloudStream, CloudError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }

    async fn remove_account(&self, account_id: &str) -> Result<(), CloudError> {
        let mut status = self.status_tx.borrow().clone();
        status.accounts.retain(|a| a.id != account_id);
        let _ = self.status_tx.send(status.clone());
        self.save_accounts(&status.accounts);
        Ok(())
    }
}
