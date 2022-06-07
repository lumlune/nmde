use {
    super::{
        InputField,
        InputFieldMemory,
        InputFieldMemoryData,
    },
    std::sync::{Arc, Mutex},
    eframe::egui::*,
};

pub enum InputFieldInnerResponse {
    Button(Response),
    TextEdit(Response),
}

pub struct InputFieldResponse {
    pub inner_response: InputFieldInnerResponse,
    pub memory: Arc<Mutex<InputFieldMemoryData>>,
}

pub trait InputFieldDisplay<'a, I>: private::InputFieldDisplayImpl<'a, I>
    where I: Clone + PartialEq + ToString,
{
    fn show(mut self, ui: &mut Ui) -> InputFieldResponse {
        let inner_response = self.show_impl(ui);

        InputFieldResponse {
            inner_response: inner_response,
            memory: self.memory(ui),
        }
    }
}

mod private {
    use {
        super::{
            super::{
                InputField,
                InputFieldMemory,
                InputFieldMemoryData,
            },
            InputFieldDisplay,
            InputFieldInnerResponse,
        },
        std::sync::{Arc, Mutex, MutexGuard},
        eframe::{
            egui::{
                *,
                widgets::text_edit::CCursorRange,
            },
            epaint::text::cursor::CCursor,
        },
    };

    pub trait InputFieldDisplayImpl<'a, I>: InputFieldMemory<'a, I>
        where I: Clone + PartialEq + ToString,
    {
        fn show_impl(&mut self, ui: &mut Ui) -> InputFieldInnerResponse {
            let memory_arc = self.memory(ui);
            let mut memory = memory_arc.lock()
                .unwrap();

            if ui.memory().has_focus(self.widget_id()) {
                self.show_text_edit(ui, &mut memory)
            } else {
                self.show_button(ui, &mut memory)
            }
        }

        fn show_button(&mut self, ui: &mut Ui, memory: &mut MutexGuard<InputFieldMemoryData>) -> InputFieldInnerResponse {
            let response;

            response = ui.add(
                Button::new(
                    RichText::new(format!("{:>16}", &memory.display_text))
                        .monospace()
                )
            ).on_hover_cursor(CursorIcon::Text);

            if self.was_editing(ui) {
                self.commit(memory);
                self.set_editing(ui, false);
            } else {
                // If memory was reverted previous frame, it should not be
                // marked so this frame
                memory.reverted = false;
            }

            if response.clicked() || response.gained_focus() {
                ui.memory().request_focus(self.widget_id());
            }

            InputFieldInnerResponse::Button(response)
        }

        fn show_text_edit(&mut self, ui: &mut Ui, memory: &mut MutexGuard<InputFieldMemoryData>) -> InputFieldInnerResponse {
            let mut output = TextEdit::singleline(&mut memory.value_text)
                .id(self.widget_id())
                .desired_width(140.0)
                .font(TextStyle::Monospace)
                .show(ui);

            if !self.was_editing(ui) {
                self.set_editing(ui, true);

                if !memory.deviates() {
                    output.state.set_ccursor_range(Some(CCursorRange {
                        primary: CCursor {
                            index: 0,
                            ..Default::default()
                        },
                        secondary: CCursor {
                            index: memory.value_text.len(),
                            ..Default::default()
                        },
                    }));

                    TextEdit::store_state(ui.ctx(), self.widget_id(), output.state);
                }
            }

            // (egui v0.18) An `output.response.lost_focus()` check is
            // unreliable here, since it won't trigger if an earlier
            // `InputField` is focused (also seems buggy in general: see name
            // field in editor region).

            InputFieldInnerResponse::TextEdit(output.response)
        }
    }

    impl<'a, I, T> InputFieldDisplayImpl<'a, I> for T
        where I: Clone + PartialEq + ToString,
              T: InputFieldDisplay<'a, I>,
    {}
}

