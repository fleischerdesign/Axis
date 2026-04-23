use axis_domain::ports::layout::{LayoutProvider, LayoutError};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::process::Command;
use std::fs;
use log::{info, error};
use std::path::PathBuf;

pub struct NiriLayoutProvider {
    config_dir: PathBuf,
}

impl NiriLayoutProvider {
    pub fn new(config_dir: PathBuf) -> Arc<Self> {
        Arc::new(Self { config_dir })
    }

    fn get_config_path(&self) -> PathBuf {
        self.config_dir.join("niri.kdl")
    }
}

#[async_trait]
impl LayoutProvider for NiriLayoutProvider {
    async fn set_active_border_color(&self, color_hex: String) -> Result<(), LayoutError> {
        let path = self.get_config_path();
        
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                error!("[niri-layout] Failed to create config directory: {}", e);
                return Err(LayoutError::ProviderError(e.to_string()));
            }
        }

        let kdl_content = format!(
            "layout {{\n    focus-ring {{\n        active-color \"{}\"\n    }}\n}}\n",
            color_hex
        );

        if let Err(e) = fs::write(&path, kdl_content) {
            error!("[niri-layout] Failed to write niri.kdl: {}", e);
            return Err(LayoutError::ProviderError(e.to_string()));
        }

        let status = Command::new("niri")
            .arg("msg")
            .arg("action")
            .arg("load-config-file")
            .status()
            .await
            .map_err(|e| {
                error!("[niri-layout] Failed to execute niri msg: {}", e);
                LayoutError::ProviderError(e.to_string())
            })?;

        if status.success() {
            info!("[niri-layout] Border color updated to {} and config reloaded", color_hex);
            Ok(())
        } else {
            error!("[niri-layout] Niri reload failed with status: {}", status);
            Err(LayoutError::ProviderError(format!("Niri reload failed with status: {}", status)))
        }
    }
}
