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
    let level = ((percentage / 10.0).round() * 10.0) as u32;
    let level = level.min(100);

    if charging && level >= 100 {
        "battery-level-100-charged-symbolic"
    } else if charging {
        match level {
            0 => "battery-level-0-charging-symbolic",
            10 => "battery-level-10-charging-symbolic",
            20 => "battery-level-20-charging-symbolic",
            30 => "battery-level-30-charging-symbolic",
            40 => "battery-level-40-charging-symbolic",
            50 => "battery-level-50-charging-symbolic",
            60 => "battery-level-60-charging-symbolic",
            70 => "battery-level-70-charging-symbolic",
            80 => "battery-level-80-charging-symbolic",
            90 => "battery-level-90-charging-symbolic",
            _ => "battery-level-100-charging-symbolic",
        }
    } else {
        match level {
            0 => "battery-level-0-symbolic",
            10 => "battery-level-10-symbolic",
            20 => "battery-level-20-symbolic",
            30 => "battery-level-30-symbolic",
            40 => "battery-level-40-symbolic",
            50 => "battery-level-50-symbolic",
            60 => "battery-level-60-symbolic",
            70 => "battery-level-70-symbolic",
            80 => "battery-level-80-symbolic",
            90 => "battery-level-90-symbolic",
            _ => "battery-level-100-symbolic",
        }
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
