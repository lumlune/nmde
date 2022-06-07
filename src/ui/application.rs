use {
    crate::{
        io::fifo::*,
        ui::{
            region::{
                NmdAppRegion,
            },
            NmdAppView,
            UiComponent,
            options,
        },
    },
    std::{
        io,
        fs,
        path::PathBuf,
        sync::mpsc,
    },
    eframe::{
        egui::*,
        egui::util::History,
    },
};

pub struct NmdApp {
    // frame_history: History<f32>,
    message_channel: MessageChannel,
    view: NmdAppView,
}

impl NmdApp {
    fn font_data(paths: &[&str]) -> Option<Vec<u8>> {
        for path in paths {
            if let Ok(font_data) = fs::read(path) {
                return Some(font_data);
            }
        }

        None
    }

    fn icon_data(path: &str) -> Option<eframe::IconData> {
        let rgba = image::open(path).ok()?.to_rgba8();
        let (width, height) = rgba.dimensions();

        Some(eframe::IconData {
            rgba: rgba.into_raw(),
            width: width,
            height: height,
        })
    }

    fn message_receiver(&self) -> &MessageReceiver {
        &self.message_channel.1
    }

    fn message_sender(&self) -> &MessageSender {
        &self.message_channel.0
    }

    fn open(&mut self, path: &PathBuf) {
        if let Some(extension) = path.extension() {
            let result = match extension
                .to_ascii_lowercase()
                .to_string_lossy()
                .as_ref()
            {
                "nmd"
                    => self.view.try_import(path),
                "nmde"
                    => self.view.try_open(path),
                _   => { return; }
            };

            if result.is_ok() {
                self.view.show_newest();
            }
        }
    }

    pub(crate) fn run() {
        use options::*;

        let app_options = eframe::NativeOptions {
            drag_and_drop_support: true,
            icon_data: Self::icon_data(ICON_PATH),
            initial_window_size: INITIAL_WINDOW_SIZE,
            ..Default::default()
        };

        eframe::run_native(
            "nmde",
            app_options,
            Box::new(|creation_context| Box::new(Self::from(creation_context)))
        );
    }

    fn try_receive_message(&self) -> Result<Message, mpsc::TryRecvError> {
        self.message_receiver().try_recv()
    }
}

impl eframe::App for NmdApp {
    fn update(&mut self, ctx: &Context, frame: &mut eframe::Frame) {
        loop {
            use Message::*;

            match self.try_receive_message() {
                Ok(UiSelect(ui_component))  => { self.view.select(&ui_component); }
                Ok(message)                 => { self.view.receive_message(&message); }
                _                           => { break; }
            }
        }

        for dropped_file in &ctx.input().raw.dropped_files {
            if let Some(path) = &dropped_file.path {
                self.open(path);
            }
        }

        // self.diagnose(ctx, frame);

        self.view.ui(ctx);
    }
}

impl Default for NmdApp {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();

        Self {
            // frame_history: History::new(0..300, 1.0),
            view: NmdAppView::new(&sender),
            message_channel: (sender, receiver),
        }
    }
}

impl From<&eframe::CreationContext<'_>> for NmdApp {
    fn from(creation_context: &eframe::CreationContext) -> Self {
        use options::*;

        let mut app = Self::default();

        if let Some(font_bytes) = Self::font_data(MONOSPACE_FONT_PATH) {
            let mut app_fonts = FontDefinitions::default();
            let monospace_font_name = "custom_mono";

            app_fonts.font_data
                .insert(monospace_font_name.to_string(), FontData::from_owned(font_bytes));
            app_fonts.families.get_mut(&FontFamily::Monospace)
                .unwrap()
                .insert(0, monospace_font_name.to_string());

            creation_context.egui_ctx.set_fonts(app_fonts);
        }

        creation_context.egui_ctx.set_visuals(Visuals::dark());

        app
    }
}

// impl NmdApp {
//     fn diagnose(&mut self, ctx: &Context, frame: &eframe::Frame) {
//         TopBottomPanel::top("region$debug")
//             .show(ctx, |ui|
//         {
//             let now = ctx.input().time;
//             let cpu = frame.info().cpu_usage;

//             let prev_frame_time = cpu.unwrap_or_default();

//             if let Some(latest) = self.frame_history.latest_mut() {
//                 *latest = prev_frame_time;
//             }

//             self.frame_history.add(now, prev_frame_time);

//             ui.add_space(4.0);
//             ui.horizontal(|ui| {
//                 let frc = self.frame_history.total_count();
//                 let fps = 1.0 / self.frame_history.mean_time_interval().unwrap_or_default();
//                 let mft = self.frame_history.average().unwrap_or_default();

//                 ui.label(format!("Frames: {}", frc));
//                 ui.separator();
//                 ui.label(format!("FPS: {}", fps));
//                 ui.separator();
//                 ui.label(format!("CPU: {:.2} ms / frame", 1000.0 * mft));
//             });
//         });
//     }
// }
