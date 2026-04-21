use std::time::Instant;
use log::info;

use super::{
    ContinuityInner, SharingState, VIRTUAL_POS_BUFFER,
};
use super::connection::ConnectionProvider;
use super::input::{EvdevCapture, InputCapture};
use super::protocol;

impl ContinuityInner {
    pub(super) fn remote_screen(&self) -> (i32, i32) {
        self.data.remote_screen.unwrap_or((self.data.screen_width, self.data.screen_height))
    }

    pub(super) fn init_virtual_pos(entry_side: super::Side, edge_pos: f64, remote_w: i32, remote_h: i32) -> (f64, f64) {
        let (rw, rh) = (remote_w as f64, remote_h as f64);
        let buffer = VIRTUAL_POS_BUFFER;
        match entry_side {
            super::Side::Right => (buffer, edge_pos.clamp(0.0, rh)),
            super::Side::Left => (rw - buffer, edge_pos.clamp(0.0, rh)),
            super::Side::Bottom => (edge_pos.clamp(0.0, rw), buffer),
            super::Side::Top => (edge_pos.clamp(0.0, rw), rh - buffer),
        }
    }

    pub(super) async fn handle_input_capture_event(
        &mut self,
        event: super::input::InputEvent,
        connection: &super::connection::TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        if !matches!(self.data.sharing_state, SharingState::Sharing { .. }) {
            return;
        }
        match event {
            super::input::InputEvent::CursorMove { dx, dy } => {
                let (rw, rh) = self.remote_screen();
                let rw = rw as f64;
                let rh = rh as f64;

                let SharingState::Sharing { entry_side, virtual_pos: mut vpos } = self.data.sharing_state.clone() else { return };

                vpos.0 += dx;
                vpos.1 += dy;
                vpos.0 = vpos.0.clamp(-100.0, rw + 100.0);
                vpos.1 = vpos.1.clamp(-100.0, rh + 100.0);

                let should_return = match entry_side {
                    super::Side::Left if vpos.0 > rw => true,
                    super::Side::Right if vpos.0 < 0.0 => true,
                    super::Side::Top if vpos.1 > rh => true,
                    super::Side::Bottom if vpos.1 < 0.0 => true,
                    _ => false,
                };

                if should_return {
                    info!("[continuity] return transition at vpos=({:.0},{:.0})", vpos.0, vpos.1);
                    self.data.sharing_state = SharingState::Idle;
                    self.last_transition_at = Instant::now();
                    capture.stop();
                    connection.send_message(protocol::Message::TransitionCancel);
                    self.push();
                    let _ = capture.prepare();
                    return;
                }

                self.data.sharing_state = SharingState::Sharing { entry_side, virtual_pos: vpos };
                connection.send_message(protocol::Message::CursorMove { dx, dy });
            }
            super::input::InputEvent::KeyPress { key, state } => {
                connection.send_message(protocol::Message::KeyPress { key, state });
            }
            super::input::InputEvent::KeyRelease { key } => {
                connection.send_message(protocol::Message::KeyRelease { key });
            }
            super::input::InputEvent::PointerButton { button, state } => {
                connection.send_message(protocol::Message::PointerButton { button, state });
            }
            super::input::InputEvent::PointerAxis { dx, dy } => {
                connection.send_message(protocol::Message::PointerAxis { dx, dy });
            }
            super::input::InputEvent::EmergencyExit => {
                info!("[continuity] kernel emergency exit requested");
                self.data.sharing_state = SharingState::Idle;
                capture.stop();
                connection.send_message(protocol::Message::TransitionCancel);
                self.push();
                let _ = capture.prepare();
            }
        };
    }
}
