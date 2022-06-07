use {
    eframe::egui::Vec2,
};

pub const INITIAL_WINDOW_SIZE: Option<Vec2> = Some(Vec2 {
    x: 902.0,
    y: 644.0,
});

pub const ICON_PATH: &str = "resource/icon.png";
pub const MONOSPACE_FONT_PATH: &[&str] = &[
    "resource/mono.otf",
    "resource/mono.ttf",
];
