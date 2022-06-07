use {
    super::InputField,
    eframe::egui::*,
};

pub(in super) trait WidgetUtils {
    fn set_min_button_width(ui: &mut Ui, width: f32, text: &String) {
        let pad_x = (width - text_width(ui, text)) / 2.0;

        ui.spacing_mut().button_padding.x = f32::max(pad_x, ui.spacing().button_padding.x);
    }
}

impl<'a, I> WidgetUtils for I
    where I: InputField<'a>
{}

fn text_width(ui: &Ui, text: &String) -> f32 {
    let font_id = TextStyle::Monospace.resolve(ui.style());
    let mut width = 0.0;

    for character in text.chars() {
        width += ui.fonts().glyph_width(&font_id, character);
    }

    width
}
