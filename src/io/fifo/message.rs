use {
    crate::ui::{
        UiComponent,
        UiState,
    },
};

#[derive(Debug)]
pub enum Message {
    UiSelect(UiComponent),
    UiState(UiState),
}
