use async_channel::{bounded, Receiver};
use niri_ipc::{socket::Socket, Output, Request, Response, Window, Workspace, Event};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

#[derive(Clone, Default, Debug)]
pub struct NiriData {
    pub workspaces: Vec<Workspace>,
    pub windows: Vec<Window>,
    pub outputs: HashMap<String, Output>,
}

impl PartialEq for NiriData {
    fn eq(&self, other: &Self) -> bool {
        if self.workspaces.len() != other.workspaces.len() || self.windows.len() != other.windows.len() {
            return false;
        }
        let ws_eq = self.workspaces.iter().zip(&other.workspaces)
            .all(|(a, b)| a.id == b.id && a.is_active == b.is_active);
        let win_eq = self.windows.iter().zip(&other.windows).all(|(a, b)| {
            a.id == b.id && a.is_focused == b.is_focused && a.workspace_id == b.workspace_id
        });
        ws_eq && win_eq
    }
}

pub struct NiriService;

impl NiriService {
    pub fn spawn() -> Receiver<NiriData> {
        let (data_tx, data_rx) = bounded(10);

        thread::spawn(move || {
            loop {
                if let Ok(mut client) = Socket::connect() {
                    println!("Niri: Connected to IPC");
                    
                    // Initialen Zustand abfragen
                    if let Some(data) = Self::fetch_full_state(&mut client) {
                        let _ = data_tx.send_blocking(data);
                    }

                    // Event Stream anfordern
                    if let Ok(Ok(Response::Handled)) = client.send(Request::EventStream) {
                        let mut read_event = client.read_events();
                        
                        // Hier blockiert der Thread, bis ein Event kommt
                        while let Ok(event) = read_event() {
                            match event {
                                Event::WorkspacesChanged { .. } | 
                                Event::WorkspaceActivated { .. } |
                                Event::WindowsChanged { .. } |
                                Event::WindowOpenedOrChanged { .. } |
                                Event::WindowClosed { .. } => {
                                    // Bei relevanten Änderungen neu pollen (über separaten Client)
                                    if let Ok(mut query_client) = Socket::connect() {
                                        if let Some(data) = Self::fetch_full_state(&mut query_client) {
                                            let _ = data_tx.send_blocking(data);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // Bei Verbindungsverlust kurz warten und neu versuchen
                thread::sleep(Duration::from_secs(1));
            }
        });

        data_rx
    }

    fn fetch_full_state(client: &mut Socket) -> Option<NiriData> {
        let ws = client.send(Request::Workspaces);
        let wins = client.send(Request::Windows);
        let outs = client.send(Request::Outputs);

        if let (
            Ok(Ok(Response::Workspaces(ws))),
            Ok(Ok(Response::Windows(wins))),
            Ok(Ok(Response::Outputs(outs))),
        ) = (ws, wins, outs)
        {
            Some(NiriData {
                workspaces: ws,
                windows: wins,
                outputs: outs,
            })
        } else {
            None
        }
    }
}
