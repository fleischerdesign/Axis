use axis_domain::models::ssh::{SshSession, SshStatus};
use axis_domain::ports::ssh::{SshProvider, SshError, SshStream};
use async_trait::async_trait;
use tokio::sync::watch;
use tokio_stream::wrappers::WatchStream;
use std::sync::Arc;
use std::time::Duration;

pub struct ProcSshProvider {
    status_tx: watch::Sender<SshStatus>,
}

impl ProcSshProvider {
    pub fn new() -> Arc<Self> {
        let (status_tx, _) = watch::channel(SshStatus::default());
        let tx = status_tx.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let status = Self::scan_sessions();
                let _ = tx.send(status);
            }
        });

        Arc::new(Self { status_tx })
    }

    fn scan_sessions() -> SshStatus {
        let mut sessions = Vec::new();

        let dir = match std::fs::read_dir("/proc") {
            Ok(d) => d,
            Err(_) => return SshStatus::default(),
        };

        for entry in dir.flatten() {
            let path = entry.path();
            let pid_str = match path.file_name().and_then(|n| n.to_str()) {
                Some(name) => name,
                None => continue,
            };

            let pid: u32 = match pid_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let cmdline = match std::fs::read(path.join("cmdline")) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let process_name = cmdline
                .split(|&b| b == 0)
                .next()
                .unwrap_or(&[])
                .to_vec();
            let process_name = match String::from_utf8(process_name) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if process_name.contains("listener")
                || process_name.contains("[net]")
                || process_name.contains("[accepted]")
                || process_name.ends_with(" [priv]")
            {
                continue;
            }

            let rest = if let Some(r) = process_name.strip_prefix("sshd: ") {
                r
            } else if let Some(r) = process_name.strip_prefix("sshd-session: ") {
                r
            } else {
                continue;
            };

            let (username, terminal) = if let Some(at_pos) = rest.find('@') {
                let user = &rest[..at_pos];
                let term = &rest[at_pos + 1..];
                (user.to_string(), term.to_string())
            } else {
                let user = rest.to_string();
                (user, "notty".to_string())
            };

            let source_ip = Self::read_ssh_connection(pid);
            let connected_for = Self::read_duration(pid);

            sessions.push(SshSession {
                pid,
                username,
                terminal,
                source_ip,
                connected_for,
            });
        }

        let active_count = sessions.len();
        SshStatus { sessions, active_count }
    }

    fn read_ssh_connection(pid: u32) -> Option<String> {
        if let Some(ip) = Self::try_read_environ(pid) {
            return Some(ip);
        }
        let children = Self::read_child_pids(pid);
        for child_pid in children {
            if let Some(ip) = Self::try_read_environ(child_pid) {
                return Some(ip);
            }
        }
        None
    }

    fn try_read_environ(pid: u32) -> Option<String> {
        let environ_path = std::path::PathBuf::from(format!("/proc/{}/environ", pid));
        let data = std::fs::read(&environ_path).ok()?;

        for var in data.split(|&b| b == 0) {
            if let Ok(s) = String::from_utf8(var.to_vec()) {
                if let Some(value) = s.strip_prefix("SSH_CONNECTION=") {
                    return Some(value.split(' ').next()?.to_string());
                }
                if let Some(value) = s.strip_prefix("SSH_CLIENT=") {
                    return Some(value.split(' ').next()?.to_string());
                }
            }
        }
        None
    }

    fn read_child_pids(parent_pid: u32) -> Vec<u32> {
        let mut children = Vec::new();
        let dir = match std::fs::read_dir("/proc") {
            Ok(d) => d,
            Err(_) => return children,
        };
        for entry in dir.flatten() {
            let file_name = entry.file_name();
            let pid_str = match file_name.to_str() {
                Some(s) => s,
                None => continue,
            };
            let child_pid: u32 = match pid_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let stat_path = std::path::PathBuf::from(format!("/proc/{}/stat", child_pid));
            let stat_data = match std::fs::read_to_string(&stat_path) {
                Ok(d) => d,
                Err(_) => continue,
            };
            let after_paren = match stat_data.find(')') {
                Some(pos) => &stat_data[pos + 2..],
                None => continue,
            };
            let fields: Vec<&str> = after_paren.split(' ').collect();
            let ppid: u32 = match fields.get(1).and_then(|s| s.parse().ok()) {
                Some(p) => p,
                None => continue,
            };
            if ppid == parent_pid {
                children.push(child_pid);
            }
        }
        children
    }

    fn read_duration(pid: u32) -> String {
        let stat_path = std::path::PathBuf::from(format!("/proc/{}/stat", pid));
        let stat_data = match std::fs::read_to_string(&stat_path) {
            Ok(d) => d,
            Err(_) => return "unknown".to_string(),
        };

        let after_paren = match stat_data.find(')') {
            Some(pos) => &stat_data[pos + 2..],
            None => return "unknown".to_string(),
        };

        let fields: Vec<&str> = after_paren.split(' ').collect();
        let starttime_ticks: u64 = match fields.get(19).and_then(|s| s.parse().ok()) {
            Some(t) => t,
            None => return "unknown".to_string(),
        };

        let uptime_data = match std::fs::read_to_string("/proc/uptime") {
            Ok(d) => d,
            Err(_) => return "unknown".to_string(),
        };
        let uptime_secs: f64 = match uptime_data.split(' ').next().and_then(|s| s.parse().ok()) {
            Some(u) => u,
            None => return "unknown".to_string(),
        };

        let clk_tck: u64 = 100;
        let process_uptime = uptime_secs - (starttime_ticks as f64 / clk_tck as f64);
        if process_uptime < 0.0 {
            return "just now".to_string();
        }

        Self::format_duration(process_uptime as u64)
    }

    fn format_duration(seconds: u64) -> String {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let minutes = (seconds % 3600) / 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else if minutes > 0 {
            format!("{}m", minutes)
        } else {
            format!("{}s", seconds)
        }
    }
}

#[async_trait]
impl SshProvider for ProcSshProvider {
    async fn get_status(&self) -> Result<SshStatus, SshError> {
        Ok(self.status_tx.borrow().clone())
    }

    async fn subscribe(&self) -> Result<SshStream, SshError> {
        let rx = self.status_tx.subscribe();
        Ok(Box::pin(WatchStream::new(rx)))
    }
}
