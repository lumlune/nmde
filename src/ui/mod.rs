pub mod color;
pub mod region;
pub mod style;
pub mod widget;

mod application;
mod component; // TODO: move/rename
mod options;
mod view;

pub use {
    application::NmdApp,
    color::UI_COLORS as UiColor,
    component::{
        UiComponent,
        UiState,
    },
    style::UI_STYLES as UiStyle,
    view::NmdAppView,
};
