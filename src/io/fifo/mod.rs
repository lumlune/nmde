use {
    std::sync::{
        mpsc::{
            Sender,
            Receiver,
        }
    }
};

mod message;

pub type MessageSender = Sender<Message>;
pub type MessageReceiver = Receiver<Message>;
pub type MessageChannel = (MessageSender, MessageReceiver);

pub use {
    message::Message,
};
