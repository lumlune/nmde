use {
    crate::{
        io::fifo::{
            Message,
            MessageSender,
        },
        ui::{
            UiComponent,
        },
    },
    std::time::{Duration, SystemTime, UNIX_EPOCH},
    std::sync::Once,
    std::sync::atomic::{AtomicU64, Ordering},
    eframe::egui::{Context, Id},
};

// TODO: Use `SyncLazy`?
static UUID_INIT: Once = Once::new();
static UUID_SOURCE: AtomicU64 = AtomicU64::new(0);

pub trait NmdAppRegion {
    fn emit(&self, message: Message) -> Result<(), &'static str> {
        self.message_sender().ok_or("Message sender absent")?.send(message);

        Ok(())
    }

    fn message_sender(&self) -> Option<&MessageSender> {
        None
    }

    fn receive_message(&mut self, message: &Message) {
        // Pass
    }

    fn select(&mut self, ui_component: &UiComponent) {
        // Pass
    }

    fn ui(&mut self, ctx: &Context) {
        // Pass
    }

    fn uuid(&self) -> Id {
        Id::new(self.uuid_source())
    }

    fn uuid_source(&self) -> u64 {
        0
    }
}

pub fn generate_uuid_source() -> u64 {
    UUID_INIT.call_once(|| {
        UUID_SOURCE.fetch_add(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::ZERO)
                .as_millis() as u64,
            Ordering::Relaxed
        );
    });

    UUID_SOURCE.fetch_add(1, Ordering::Relaxed)
}

