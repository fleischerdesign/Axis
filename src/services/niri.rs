use async_channel::{bounded, Receiver};
use niri_ipc::{
    socket::Socket, Action, Event, Output, Request, Response, Window, Workspace,
    WorkspaceReferenceArg,
};
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
        if self.workspaces.len() != other.workspaces.len()
            || self.windows.len() != other.windows.len()
        {
            return false;
        }
        // Vergleich der Workspaces
        let ws_eq = self
            .workspaces
            .iter()
            .zip(&other.workspaces)
            .all(|(a, b)| a.id == b.id && a.is_active == b.is_active);
        // Vergleich der Fenster (IDs und Fokus reichen für UI-Update-Trigger)
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
            let mut reconnect_delay = Duration::from_secs(1);
            let max_delay = Duration::from_secs(30);

            loop {
                let client_event = Socket::connect();
                let client_query = Socket::connect();

                if let (Ok(mut events_sock), Ok(mut query_sock)) = (client_event, client_query) {
                    reconnect_delay = Duration::from_secs(1);
                    println!("[NiriService] Connected");

                    if let Some(data) = Self::fetch_full_state(&mut query_sock) {
                        let _ = data_tx.send_blocking(data);
                    }

                    if let Ok(Ok(Response::Handled)) = events_sock.send(Request::EventStream) {
                        let mut read_event = events_sock.read_events();

                        while let Ok(event) = read_event() {
                            match event {
                                Event::WorkspacesChanged { .. }
                                | Event::WorkspaceActivated { .. }
                                | Event::WindowsChanged { .. }
                                | Event::WindowOpenedOrChanged { .. }
                                | Event::WindowClosed { .. } => {
                                    if let Some(data) = Self::fetch_full_state(&mut query_sock) {
                                        let _ = data_tx.send_blocking(data);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    eprintln!("[NiriService] Connection lost, reconnecting...");
                }

                thread::sleep(reconnect_delay);
                reconnect_delay = (reconnect_delay * 2).min(max_delay);
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

    pub fn switch_to_workspace(ws_id: u64) {
        thread::spawn(move || {
            if let Ok(mut sock) = Socket::connect() {
                let _ = sock.send(Request::Action(Action::FocusWorkspace {
                    reference: WorkspaceReferenceArg::Id(ws_id),
                }));
            }
        });
    }
}
