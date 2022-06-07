use {
    crate::{
        io::{
            utils as io_utils,
            fifo::{
                Message,
                MessageSender,
            },
        },
        ui::region::NmdAppRegion,
    },
    eframe::egui::*,
    eframe::egui::text::LayoutJob,
};

pub struct NmdAppHomeRegion {
    message_sender: Option<MessageSender>,
}

impl NmdAppHomeRegion {
    pub fn new(message_sender: &MessageSender) -> Self {
        Self {
            message_sender: Some(message_sender.to_owned()),
        }
    }
}

impl NmdAppRegion for NmdAppHomeRegion {
    fn message_sender(&self) -> Option<&MessageSender> {
        self.message_sender.as_ref()
    }

    fn ui(&mut self, ctx: &Context) {
        CentralPanel::default()
            .show(ctx, |ui|
        {
            // Prevent panel from eclipsing menu at small size, mostly
            ui.set_min_size(Vec2 {
                x: 120.0,
                y: 120.0,
            });

            ui.with_layout(Layout::bottom_up(Align::Min), |ui| {
                // Manual spacing to enforce true center
                ui.add_space(25.0);

                ui.with_layout(Layout::centered_and_justified(Direction::TopDown), |ui| {
                    let mut prompt_text = LayoutJob::default();

                    prompt_text.append(
                        concat!(
                            "🗁"
                        ),
                        0.0, TextFormat { font_id: FontId::proportional(120.0),
                                          color: Color32::from_gray(0x8C), ..Default::default() });
                    prompt_text.append(
                        concat!(
                            "\nImport"
                        ),
                        0.0, TextFormat { color: Color32::from_gray(0x53), ..Default::default() });
                    prompt_text.append(
                        concat!(
                            " an NMD file or "
                        ),
                        0.0, TextFormat::default());
                    prompt_text.append(
                        concat!(
                            "open"
                        ),
                        0.0, TextFormat { color: Color32::from_gray(0x53), ..Default::default() });
                    prompt_text.append(
                        concat!(
                            " an existing project.\n\n"
                        ),
                        0.0, TextFormat::default());
                    prompt_text.append(
                        concat!(
                            "You can also drag and drop here."
                        ),
                        0.0, TextFormat::default());

                    ui.label(prompt_text);
                });
            });
        });
    }
}

