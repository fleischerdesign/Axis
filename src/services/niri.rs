use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use futures_channel::mpsc;
use niri_ipc::{Response, Request, Workspace, Window, Output, socket::Socket};

pub struct NiriData {
    pub workspaces: Vec<Workspace>,
    pub windows: Vec<Window>,
    pub outputs: HashMap<String, Output>,
}

pub struct NiriService;

impl NiriService {
    pub fn spawn() -> mpsc::UnboundedReceiver<NiriData> {
        let (tx, rx) = mpsc::unbounded();

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
                            
                            let data = NiriData {
                                workspaces: ws,
                                windows: wins,
                                outputs: outs,
                            };

                            if tx.unbounded_send(data).is_err() {
                                return; // Kanal geschlossen
                            }
                        }
                        thread::sleep(Duration::from_millis(250));
                    }
                }
                thread::sleep(Duration::from_secs(1)); // Retry connection
            }
        });

        rx
    }
}
