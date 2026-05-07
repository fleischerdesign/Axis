pub struct ThemeDimensions {
    pub bar_bottom_margin: i32,
    pub bar_peek_pixels: i32,
    pub bar_autohide_delay_ms: u64,
    pub qs_popup_bottom_margin: i32,
    pub qs_popup_right_margin: i32,
    pub qs_popup_default_width: i32,
    pub launcher_height: i32,
    pub launcher_width_collapsed: i32,
    pub launcher_width_expanded: i32,
    pub osd_hide_timeout_ms: u64,
    pub osd_show_timeout_ms: u64,
    pub album_art_size: i32,
    pub toast_timeout_secs: u32,
    pub task_list_width: i32,
    pub calendar_grid_width: i32,
    pub wallpaper_blur_radius: f64,
    pub swipe_dismiss_threshold: f64,
}

impl Default for ThemeDimensions {
    fn default() -> Self {
        Self {
            bar_bottom_margin: 54,
            bar_peek_pixels: 1,
            bar_autohide_delay_ms: 500,
            qs_popup_bottom_margin: 64,
            qs_popup_right_margin: 10,
            qs_popup_default_width: 320,
            launcher_height: 400,
            launcher_width_collapsed: 380,
            launcher_width_expanded: 680,
            osd_hide_timeout_ms: 300,
            osd_show_timeout_ms: 2000,
            album_art_size: 200,
            toast_timeout_secs: 5,
            task_list_width: 280,
            calendar_grid_width: 240,
            wallpaper_blur_radius: 30.0,
            swipe_dismiss_threshold: 100.0,
        }
    }
}

pub const THEME: ThemeDimensions = ThemeDimensions {
    bar_bottom_margin: 54,
    bar_peek_pixels: 1,
    bar_autohide_delay_ms: 500,
    qs_popup_bottom_margin: 64,
    qs_popup_right_margin: 10,
    qs_popup_default_width: 320,
    launcher_height: 400,
    launcher_width_collapsed: 380,
    launcher_width_expanded: 680,
    osd_hide_timeout_ms: 300,
    osd_show_timeout_ms: 2000,
    album_art_size: 200,
    toast_timeout_secs: 5,
    task_list_width: 280,
    calendar_grid_width: 240,
    wallpaper_blur_radius: 30.0,
    swipe_dismiss_threshold: 100.0,
};
