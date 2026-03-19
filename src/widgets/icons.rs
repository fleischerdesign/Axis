pub fn wifi_signal_icon(strength: u8) -> &'static str {
    if strength > 75 {
        "network-wireless-signal-excellent-symbolic"
    } else if strength > 50 {
        "network-wireless-signal-good-symbolic"
    } else if strength > 25 {
        "network-wireless-signal-ok-symbolic"
    } else {
        "network-wireless-signal-weak-symbolic"
    }
}

pub fn battery_icon(percentage: f64, charging: bool) -> &'static str {
    if charging {
        "battery-full-charging-symbolic"
    } else if percentage < 10.0 {
        "battery-empty-symbolic"
    } else if percentage < 30.0 {
        "battery-low-symbolic"
    } else if percentage < 60.0 {
        "battery-good-symbolic"
    } else {
        "battery-full-symbolic"
    }
}

pub fn volume_icon(volume: f64, muted: bool) -> &'static str {
    if muted || volume <= 0.01 {
        "audio-volume-muted-symbolic"
    } else if volume < 0.33 {
        "audio-volume-low-symbolic"
    } else if volume < 0.66 {
        "audio-volume-medium-symbolic"
    } else {
        "audio-volume-high-symbolic"
    }
}
