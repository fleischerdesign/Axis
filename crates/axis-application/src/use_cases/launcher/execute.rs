use std::os::unix::process::CommandExt;

use axis_domain::models::launcher::LauncherAction;
use axis_domain::ports::launcher::LauncherError;

#[derive(Default)]
pub struct ExecuteLauncherActionUseCase {}

impl ExecuteLauncherActionUseCase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn execute(&self, action: &LauncherAction) -> Result<(), LauncherError> {
        match action {
            LauncherAction::Noop => Ok(()),
            LauncherAction::Exec(args) => {
                if args.is_empty() {
                    return Ok(());
                }
                log::info!("[launcher] Executing: {args:?}");
                let mut cmd = std::process::Command::new(&args[0]);
                cmd.args(&args[1..])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .process_group(0);
                cmd.spawn().map(|_| ()).map_err(|e| {
                    LauncherError::ProviderError(format!("Failed to execute {:?}: {}", args, e))
                })
            }
            LauncherAction::OpenUrl(url) => {
                log::info!("[launcher] Opening URL: {url}");
                std::process::Command::new("xdg-open")
                    .arg(url)
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .process_group(0)
                    .spawn()
                    .map(|_| ())
                    .map_err(|e| {
                        LauncherError::ProviderError(format!("Failed to open URL {}: {}", url, e))
                    })
            }
            LauncherAction::Internal(cmd) => {
                log::info!("[launcher] Internal command: {cmd}");
                Ok(())
            }
        }
    }
}
