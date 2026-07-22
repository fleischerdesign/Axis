use axis_domain::models::continuity::{Message, SharingState, Side};
use log::info;

use super::super::connection::{ConnectionProvider, TcpConnectionProvider};
use super::super::input::{EvdevCapture, InputCapture, InternalInputEvent};
use super::ContinuityInner;

impl ContinuityInner {
    pub(crate) async fn handle_input_capture_event(
        &mut self,
        event: InternalInputEvent,
        connection: &TcpConnectionProvider,
        capture: &mut EvdevCapture,
    ) {
        if !matches!(self.status.sharing_state, SharingState::Sharing { .. }) {
            return;
        }
        match event {
            InternalInputEvent::CursorMove { dx, dy } => {
                let (rw, rh) = self.remote_screen();
                let rw_f = rw as f64;
                let rh_f = rh as f64;

                let SharingState::Sharing {
                    entry_side,
                    virtual_pos: mut vpos,
                } = self.status.sharing_state.clone()
                else {
                    return;
                };

                vpos.0 += dx;
                vpos.1 += dy;
                vpos.0 = vpos.0.clamp(-100.0, rw_f + 100.0);
                vpos.1 = vpos.1.clamp(-100.0, rh_f + 100.0);

                let should_return = match entry_side {
                    Side::Left if vpos.0 > rw_f => true,
                    Side::Right if vpos.0 < 0.0 => true,
                    Side::Top if vpos.1 > rh_f => true,
                    Side::Bottom if vpos.1 < 0.0 => true,
                    _ => false,
                };

                if should_return {
                    info!(
                        "[continuity] return transition at vpos=({:.0},{:.0})",
                        vpos.0, vpos.1
                    );
                    self.status.sharing_state = SharingState::Idle;
                    self.last_transition_at = std::time::Instant::now();
                    capture.stop();
                    connection.send_message(Message::TransitionCancel);
                    self.push();
                    let _ = capture.prepare();
                    return;
                }

                self.status.sharing_state = SharingState::Sharing {
                    entry_side,
                    virtual_pos: vpos,
                };
                connection.send_message(Message::CursorMove { dx, dy });
            }
            InternalInputEvent::KeyPress { key, state } => {
                connection.send_message(Message::KeyPress { key, state });
            }
            InternalInputEvent::KeyRelease { key } => {
                connection.send_message(Message::KeyRelease { key });
            }
            InternalInputEvent::PointerButton { button, state } => {
                connection.send_message(Message::PointerButton { button, state });
            }
            InternalInputEvent::PointerAxis { dx, dy } => {
                connection.send_message(Message::PointerAxis { dx, dy });
            }
            InternalInputEvent::EmergencyExit => {
                info!("[continuity] kernel emergency exit requested");
                self.status.sharing_state = SharingState::Idle;
                capture.stop();
                connection.send_message(Message::TransitionCancel);
                self.push();
                let _ = capture.prepare();
            }
        };
    }
}
