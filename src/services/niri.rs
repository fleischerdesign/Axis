use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use niri_ipc::{Response, Request, Workspace, Window, Output, socket::Socket};

#[derive(Clone, Default)]
pub struct NiriData {
    pub workspaces: Vec<Workspace>,
    pub windows: Vec<Window>,
    pub outputs: HashMap<String, Output>,
}

// Wir implementieren manuell PartialEq für NiriData, um send_if_modified zu ermöglichen
impl PartialEq for NiriData {
    fn eq(&self, other: &Self) -> bool {
        if self.workspaces.len() != other.workspaces.len() || self.windows.len() != other.windows.len() {
            return false;
        }
        // Vereinfachter Vergleich: Prüfen ob IDs und Fokus gleich sind
        let ws_eq = self.workspaces.iter().zip(&other.workspaces).all(|(a, b)| a.id == b.id && a.is_active == b.is_active);
        let win_eq = self.windows.iter().zip(&other.windows).all(|(a, b)| a.id == b.id && a.is_focused == b.is_focused && a.workspace_id == b.workspace_id);
        
        ws_eq && win_eq
    }
}

pub struct NiriService;

impl NiriService {
    pub fn spawn() -> tokio::sync::watch::Receiver<NiriData> {
        let (tx, rx) = tokio::sync::watch::channel(NiriData::default());

        thread::spawn(move || {
            loop {
                if let Ok(mut client) = Socket::connect() {
                    loop {
                        let ws = client.send(Request::Workspaces);
                        let wins = client.send(Request::Windows);
                        let outs = client.send(Request::Outputs);

                        if let (Ok(Ok(Response::Workspaces(ws))), 
                                Ok(Ok(Response::Windows(wins))), 
                                Ok(Ok(Response::Outputs(outs)))) = (ws, wins, outs) {
                            
                            let new_data = NiriData {
                                workspaces: ws,
                                windows: wins,
                                outputs: outs,
                            };

                            tx.send_if_modified(|current| {
                                if *current != new_data {
                                    *current = new_data;
                                    true
                                } else {
                                    false
                                }
                            });
                        }
                        thread::sleep(Duration::from_millis(500)); // Etwas langsamer pollen spart massiv Energie
                    }
                }
                thread::sleep(Duration::from_secs(1));
            }
        });

        rx
    }
}
