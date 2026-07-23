#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use async_channel::bounded;
    use axis_domain::models::continuity::{
        AudioStreamDirection, ContinuityStatus, Message, PeerArrangement, PeerConfig, Side,
    };

    use crate::adapters::continuity::drag_drop::DragDropManager;
    use crate::adapters::continuity::inner::{CmdContext, ConfigSyncArgs, ContinuityInner};
    use crate::adapters::continuity::testing::{
        MockAudio, MockCapture, MockClipboard, MockDiscovery, MockInjection, MockNetwork,
    };

    fn make_inner() -> (ContinuityInner, tokio::sync::watch::Receiver<ContinuityStatus>) {
        let (tx, rx) = tokio::sync::watch::channel(ContinuityStatus::default());
        (ContinuityInner::new(tx), rx)
    }

    fn setup_trusted_peer(inner: &mut ContinuityInner, peer_id: &str) {
        inner.status.peer_configs.insert(
            peer_id.to_string(),
            PeerConfig {
                trusted: true,
                clipboard: true,
                audio: false,
                audio_direction: AudioStreamDirection::Off,
                drag_drop: false,
                auto_connect: false,
                ..Default::default()
            },
        );
    }

    fn fake_peer() -> String {
        "550e8400-e29b-41d4-a716-446655440000".to_string()
    }

    #[tokio::test]
    async fn test_configsync_arrangement_mirroring() {
        let (mut inner, status_rx) = make_inner();
        let peer_id = fake_peer();

        setup_trusted_peer(&mut inner, &peer_id);
        inner.status.active_connection = Some(
            axis_domain::models::continuity::ActiveConnectionInfo {
                peer_id: peer_id.clone(),
                peer_name: "test-peer".to_string(),
                connected_secs: 0,
            },
        );
        inner.connected_at = Some(std::time::Instant::now());

        let mut network = MockNetwork::new();
        let mut capture = MockCapture::new();
        let mut injection = MockInjection::new();
        let mut clipboard = MockClipboard::new();
        let mut discovery = MockDiscovery::new();
        let audio = MockAudio::new();
        let drag_drop = DragDropManager::new();
        let (dtx, _) = bounded(4);
        let (ctx, _) = bounded(4);
        let (cltx, _) = bounded(4);
        let (itx, _) = bounded(4);

        let mut cmd_ctx = CmdContext {
            network: &mut network,
            capture: &mut capture,
            injection: &mut injection,
            clipboard: &mut clipboard,
            discovery: &mut discovery,
            audio: &audio,
            drag_drop_mgr: &drag_drop,
            discovery_tx: &dtx,
            conn_tx: &ctx,
            clipboard_tx: &cltx,
            input_tx: &itx,
        };

        inner
            .handle_config_sync(
                ConfigSyncArgs {
                    arrangement: Side::Right,
                    offset: 42,
                    clipboard: true,
                    audio: true,
                    audio_direction: AudioStreamDirection::SendToPeer,
                    drag_drop: true,
                    version: 1,
                },
                &mut cmd_ctx,
            )
            .await;

        let status = status_rx.borrow().clone();
        let config = status.peer_configs.get(&peer_id).expect("config should exist");

        assert_eq!(config.arrangement.side, Side::Left, "side should be mirrored");
        assert_eq!(config.arrangement.offset, -42, "offset should be negated");
        assert_eq!(
            config.audio_direction,
            AudioStreamDirection::ReceiveFromPeer,
            "audio direction should be mirrored"
        );
        assert!(config.clipboard);
        assert!(config.drag_drop);
    }

    #[tokio::test]
    async fn test_configsync_ignores_older_version() {
        let (mut inner, status_rx) = make_inner();
        let peer_id = fake_peer();

        setup_trusted_peer(&mut inner, &peer_id);
        inner.status.active_connection = Some(
            axis_domain::models::continuity::ActiveConnectionInfo {
                peer_id: peer_id.clone(),
                peer_name: "test-peer".to_string(),
                connected_secs: 0,
            },
        );
        inner.connected_at = Some(std::time::Instant::now());

        let config = inner.status.peer_configs.get_mut(&peer_id).unwrap();
        config.version = 5;
        config.arrangement = PeerArrangement {
            side: Side::Left,
            offset: 50,
        };
        config.audio_direction = AudioStreamDirection::ReceiveFromPeer;

        let mut network = MockNetwork::new();
        let mut capture = MockCapture::new();
        let mut injection = MockInjection::new();
        let mut clipboard = MockClipboard::new();
        let mut discovery = MockDiscovery::new();
        let audio = MockAudio::new();
        let drag_drop = DragDropManager::new();
        let (dtx, _) = bounded(4);
        let (ctx, _) = bounded(4);
        let (cltx, _) = bounded(4);
        let (itx, _) = bounded(4);

        let mut cmd_ctx = CmdContext {
            network: &mut network,
            capture: &mut capture,
            injection: &mut injection,
            clipboard: &mut clipboard,
            discovery: &mut discovery,
            audio: &audio,
            drag_drop_mgr: &drag_drop,
            discovery_tx: &dtx,
            conn_tx: &ctx,
            clipboard_tx: &cltx,
            input_tx: &itx,
        };

        inner
            .handle_config_sync(
                ConfigSyncArgs {
                    arrangement: Side::Right,
                    offset: 100,
                    clipboard: false,
                    audio: false,
                    audio_direction: AudioStreamDirection::Off,
                    drag_drop: false,
                    version: 3,
                },
                &mut cmd_ctx,
            )
            .await;

        let status = status_rx.borrow().clone();
        let config = match status.peer_configs.get(&peer_id) {
            Some(c) => c,
            None => {
                // Config was ignored because version 3 < 5, which is correct
                // Status was not pushed, so read from inner state directly
                let c = inner.status.peer_configs.get(&peer_id).unwrap();
                assert_eq!(c.version, 5, "version should not be downgraded");
                assert_eq!(c.arrangement.side, Side::Left);
                return;
            }
        };
        assert_eq!(config.version, 5, "version should not be downgraded");
        assert_eq!(config.arrangement.side, Side::Left, "arrangement should not change");
    }

    #[tokio::test]
    async fn test_set_peer_arrangement_sends_configsync() {
        let (mut inner, _status_rx) = make_inner();
        let peer_id = fake_peer();

        setup_trusted_peer(&mut inner, &peer_id);
        inner.status.active_connection = Some(
            axis_domain::models::continuity::ActiveConnectionInfo {
                peer_id: peer_id.clone(),
                peer_name: "test-peer".to_string(),
                connected_secs: 0,
            },
        );
        inner.connected_at = Some(std::time::Instant::now());

        let mut network = MockNetwork::new();
        let mut capture = MockCapture::new();
        let mut injection = MockInjection::new();
        let mut clipboard = MockClipboard::new();
        let mut discovery = MockDiscovery::new();
        let audio = MockAudio::new();
        let drag_drop = DragDropManager::new();
        let (dtx, _) = bounded(4);
        let (ctx, _) = bounded(4);
        let (cltx, _) = bounded(4);
        let (itx, _) = bounded(4);

        let mut cmd_ctx = CmdContext {
            network: &mut network,
            capture: &mut capture,
            injection: &mut injection,
            clipboard: &mut clipboard,
            discovery: &mut discovery,
            audio: &audio,
            drag_drop_mgr: &drag_drop,
            discovery_tx: &dtx,
            conn_tx: &ctx,
            clipboard_tx: &cltx,
            input_tx: &itx,
        };

        inner
            .handle_set_peer_arrangement(
                PeerArrangement {
                    side: Side::Bottom,
                    offset: 200,
                },
                cmd_ctx.network,
            )
            .await;

        let sent = network.drain_sent();
        assert_eq!(sent.len(), 1, "should send one ConfigSync");
        assert!(matches!(
            &sent[0],
            Message::ConfigSync {
                arrangement: Side::Bottom,
                offset: 200,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_update_peer_configs_with_matching_peer_id_sends_configsync() {
        let (mut inner, _status_rx) = make_inner();
        let peer_id = fake_peer();

        setup_trusted_peer(&mut inner, &peer_id);
        inner.status.active_connection = Some(
            axis_domain::models::continuity::ActiveConnectionInfo {
                peer_id: peer_id.clone(),
                peer_name: "test-peer".to_string(),
                connected_secs: 0,
            },
        );
        inner.connected_at = Some(std::time::Instant::now());

        let mut network = MockNetwork::new();
        let mut capture = MockCapture::new();
        let mut injection = MockInjection::new();
        let mut clipboard = MockClipboard::new();
        let mut discovery = MockDiscovery::new();
        let audio = MockAudio::new();
        let drag_drop = DragDropManager::new();
        let (dtx, _) = bounded(4);
        let (ctx, _) = bounded(4);
        let (cltx, _) = bounded(4);
        let (itx, _) = bounded(4);

        let mut cmd_ctx = CmdContext {
            network: &mut network,
            capture: &mut capture,
            injection: &mut injection,
            clipboard: &mut clipboard,
            discovery: &mut discovery,
            audio: &audio,
            drag_drop_mgr: &drag_drop,
            discovery_tx: &dtx,
            conn_tx: &ctx,
            clipboard_tx: &cltx,
            input_tx: &itx,
        };

        let mut configs = HashMap::new();
        configs.insert(
            peer_id.clone(),
            PeerConfig {
                clipboard: false,
                audio: true,
                audio_direction: AudioStreamDirection::SendToPeer,
                ..Default::default()
            },
        );

        inner
            .handle_update_peer_configs(configs, &mut cmd_ctx)
            .await;

        let sent = network.drain_sent();
        assert!(!sent.is_empty(), "should send ConfigSync for active peer");
        assert!(matches!(
            &sent[0],
            Message::ConfigSync {
                clipboard: false,
                audio: true,
                audio_direction: AudioStreamDirection::SendToPeer,
                ..
            }
        ));
    }

    #[tokio::test]
    async fn test_handle_hello_trusted_sets_active_connection() {
        let (mut inner, status_rx) = make_inner();
        let peer_id = fake_peer();

        setup_trusted_peer(&mut inner, &peer_id);
        inner.is_initiating = true;

        let mut network = MockNetwork::new();
        let mut capture = MockCapture::new();
        let mut injection = MockInjection::new();
        let mut clipboard = MockClipboard::new();
        let mut discovery = MockDiscovery::new();
        let audio = MockAudio::new();
        let drag_drop = DragDropManager::new();
        let (dtx, _) = bounded(4);
        let (ctx, _) = bounded(4);
        let (cltx, _) = bounded(4);
        let (itx, _) = bounded(4);

        let mut cmd_ctx = CmdContext {
            network: &mut network,
            capture: &mut capture,
            injection: &mut injection,
            clipboard: &mut clipboard,
            discovery: &mut discovery,
            audio: &audio,
            drag_drop_mgr: &drag_drop,
            discovery_tx: &dtx,
            conn_tx: &ctx,
            clipboard_tx: &cltx,
            input_tx: &itx,
        };

        inner
            .handle_hello(peer_id.clone(), "test-peer".to_string(), 1, &mut cmd_ctx)
            .await;

        let status = status_rx.borrow().clone();
        assert!(status.active_connection.is_some(), "should have active connection");
        assert_eq!(status.active_connection.as_ref().unwrap().peer_id, peer_id);
        assert!(status.pending_pin.is_none(), "trusted peer should skip PIN");
    }

    #[tokio::test]
    async fn test_configsync_both_v0_initiator_ignores() {
        let (mut inner, status_rx) = make_inner();
        let peer_id = fake_peer();

        setup_trusted_peer(&mut inner, &peer_id);
        inner.status.active_connection = Some(
            axis_domain::models::continuity::ActiveConnectionInfo {
                peer_id: peer_id.clone(),
                peer_name: "test-peer".to_string(),
                connected_secs: 0,
            },
        );
        inner.connected_at = Some(std::time::Instant::now());

        let mut network = MockNetwork::new();
        let mut capture = MockCapture::new();
        let mut injection = MockInjection::new();
        let mut clipboard = MockClipboard::new();
        let mut discovery = MockDiscovery::new();
        let audio = MockAudio::new();
        let drag_drop = DragDropManager::new();
        let (dtx, _) = bounded(4);
        let (ctx, _) = bounded(4);
        let (cltx, _) = bounded(4);
        let (itx, _) = bounded(4);

        let mut cmd_ctx = CmdContext {
            network: &mut network,
            capture: &mut capture,
            injection: &mut injection,
            clipboard: &mut clipboard,
            discovery: &mut discovery,
            audio: &audio,
            drag_drop_mgr: &drag_drop,
            discovery_tx: &dtx,
            conn_tx: &ctx,
            clipboard_tx: &cltx,
            input_tx: &itx,
        };

        inner.is_initiating = true;

        inner
            .handle_config_sync(
                ConfigSyncArgs {
                    arrangement: Side::Right,
                    offset: 0,
                    clipboard: true,
                    audio: true,
                    audio_direction: AudioStreamDirection::SendToPeer,
                    drag_drop: false,
                    version: 0,
                },
                &mut cmd_ctx,
            )
            .await;

        let config = inner.status.peer_configs.get(&peer_id).unwrap();
        assert_eq!(config.version, 0, "initiator at v0 ignores peer v0");
    }

    #[tokio::test]
    async fn test_configsync_both_v0_non_initiator_adopts() {
        let (mut inner, status_rx) = make_inner();
        let peer_id = fake_peer();

        setup_trusted_peer(&mut inner, &peer_id);
        inner.status.active_connection = Some(
            axis_domain::models::continuity::ActiveConnectionInfo {
                peer_id: peer_id.clone(),
                peer_name: "test-peer".to_string(),
                connected_secs: 0,
            },
        );
        inner.connected_at = Some(std::time::Instant::now());

        let mut network = MockNetwork::new();
        let mut capture = MockCapture::new();
        let mut injection = MockInjection::new();
        let mut clipboard = MockClipboard::new();
        let mut discovery = MockDiscovery::new();
        let audio = MockAudio::new();
        let drag_drop = DragDropManager::new();
        let (dtx, _) = bounded(4);
        let (ctx, _) = bounded(4);
        let (cltx, _) = bounded(4);
        let (itx, _) = bounded(4);

        let mut cmd_ctx = CmdContext {
            network: &mut network,
            capture: &mut capture,
            injection: &mut injection,
            clipboard: &mut clipboard,
            discovery: &mut discovery,
            audio: &audio,
            drag_drop_mgr: &drag_drop,
            discovery_tx: &dtx,
            conn_tx: &ctx,
            clipboard_tx: &cltx,
            input_tx: &itx,
        };

        inner.is_initiating = false;

        inner
            .handle_config_sync(
                ConfigSyncArgs {
                    arrangement: Side::Right,
                    offset: 50,
                    clipboard: true,
                    audio: true,
                    audio_direction: AudioStreamDirection::SendToPeer,
                    drag_drop: false,
                    version: 0,
                },
                &mut cmd_ctx,
            )
            .await;

        let status = status_rx.borrow().clone();
        let config = status.peer_configs.get(&peer_id).unwrap();

        assert_eq!(config.version, 0, "non-initiator adopts through is_initial_adopt");
        assert_eq!(config.arrangement.side, Side::Left, "side mirrored");
        assert_eq!(config.arrangement.offset, -50, "offset negated");
    }
}
