use {
    eframe::egui::*,
    eframe::egui::style::*,
    lazy_static::lazy_static,
};

pub struct Styles {
    pub interactive_text: Widgets,
}

lazy_static! {
    pub static ref UI_STYLES: Styles = {
        Styles {
            interactive_text: {
                let mut visuals = Widgets::dark();

                visuals.active
                    .bg_fill    = Color32::TRANSPARENT;
                visuals.active
                    .bg_stroke  = Stroke::none();
                visuals.hovered
                    .bg_fill    = Color32::TRANSPARENT;
                visuals.hovered
                    .bg_stroke  = Stroke::none();
                visuals.inactive
                    .bg_fill    = Color32::TRANSPARENT;

                visuals
            },
        }
    };
}

