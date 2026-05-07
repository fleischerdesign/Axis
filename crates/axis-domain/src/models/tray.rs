use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct TrayItem {
    pub bus_name: String,
    pub id: String,
    pub title: String,
    pub icon_name: String,
    pub attention_icon_name: String,
    pub overlay_icon_name: String,
    pub icon_pixmap: Vec<IconPixmap>,
    pub attention_icon_pixmap: Vec<IconPixmap>,
    pub status: TrayItemStatus,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub enum TrayItemStatus {
    #[default]
    Active,
    Passive,
    NeedsAttention,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct TrayStatus {
    pub items: Vec<TrayItem>,
}
