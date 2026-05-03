use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Default)]
pub struct MprisStatus {
    pub players: Vec<MprisPlayer>,
    pub active_player_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MprisPlayer {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub art_url: Option<String>,
    pub playback: PlaybackState,
    pub position_us: i64,
    pub length_us: i64,
    pub can_play: bool,
    pub can_pause: bool,
    pub can_go_next: bool,
    pub can_go_previous: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
}

impl MprisStatus {
    pub fn active_player(&self) -> Option<&MprisPlayer> {
        self.active_player_id
            .as_ref()
            .and_then(|id| self.players.iter().find(|p| &p.id == id))
    }
}
