use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct MprisStatus {
    pub players: Vec<MprisPlayer>,
    pub active_player_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_player_returns_matching_player() {
        let status = MprisStatus {
            players: vec![MprisPlayer {
                id: "spotify".into(),
                playback: PlaybackState::Playing,
                ..Default::default()
            }],
            active_player_id: Some("spotify".into()),
        };
        let p = status.active_player();
        assert!(p.is_some());
        assert_eq!(p.unwrap().id, "spotify");
    }

    #[test]
    fn active_player_returns_none_when_no_match() {
        let status = MprisStatus {
            players: vec![],
            active_player_id: Some("missing".into()),
        };
        assert!(status.active_player().is_none());
    }

    #[test]
    fn playback_state_default_is_stopped() {
        assert_eq!(PlaybackState::default(), PlaybackState::Stopped);
    }

    #[test]
    fn playback_state_serde_roundtrip() {
        let states = vec![
            PlaybackState::Stopped,
            PlaybackState::Playing,
            PlaybackState::Paused,
        ];
        for s in states {
            let json = serde_json::to_string(&s).unwrap();
            let back: PlaybackState = serde_json::from_str(&json).unwrap();
            assert_eq!(s, back);
        }
    }
}
