mod input;
mod input_numeric;
mod input_memory;
mod input_ui;

pub type InputFieldProxy<'a, T> = Box<dyn FnMut(Option<T>) -> T + 'a>;

pub mod input_mem_utils {
    pub use super::input_memory::remove;
}

pub use self::{
    input::InputField,
    input_memory::{
        InputFieldMemory,
        InputFieldMemoryData,
    },
    input_ui::{
        InputFieldDisplay,
        InputFieldInnerResponse,
        InputFieldResponse,
    },
    input_numeric::{
        NumericDefault,
        NumericInputField
    },
};
