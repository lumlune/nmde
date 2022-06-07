use eframe::egui::*;

use {
    crate::{
        io::{
            utils as io_utils,
            fifo::{
                Message,
                MessageSender,
            },
        },
        ui::{UiComponent, UiColor, UiState},
        ui::region::{NmdAppRegion, NodeView as MenuTabView},
    },
    std::cmp::Ordering,
    std::ffi::OsStr,
    std::path::PathBuf,
    eframe::egui::*,
    eframe::egui::text::LayoutJob,
};

pub enum MenuTab {
    File(MenuTabData),
    Project(MenuTabData),
}

pub struct NmdAppMenuRegion {
    message_sender: Option<MessageSender>,
    state: NmdAppMenuProjectState,
}

#[derive(Default)]
struct NmdAppMenuProjectState {
    tab_index_opt: Option<usize>,
    tabs: Vec<MenuTab>,
}

pub struct MenuTabData {
    edited: bool,
    hiding_ids: bool,
    name: String,
    path: PathBuf,
    view: MenuTabView,
}

impl NmdAppMenuRegion {
    pub fn new(message_sender: &MessageSender) -> Self {
        Self {
            message_sender: Some(message_sender.to_owned()),
            state: Default::default(),
        }
    }

    pub fn assign_tab_to_project(&mut self, path: &PathBuf) {
        if let Some(tab) = self.state.current_tab_mut() {
            *tab = tab.to_project_tab(path);
        }
    }

    fn emit_commit(&self) {
        self.emit(Message::UiSelect(UiComponent::MenuCommit));
    }

    fn emit_hide_ids(&self, hide: bool) {
        self.emit(Message::UiSelect(UiComponent::MenuHideListIds(hide)));
    }

    fn emit_save_as(&self, path: &PathBuf) {
        self.emit(Message::UiSelect(UiComponent::MenuProjectSaveAs(path.to_owned())));
    }

    fn emit_tab(&self, index: usize) {
        self.emit(Message::UiSelect(UiComponent::MenuTab(index)));
    }

    fn emit_tab_close(&self, index: usize) {
        self.emit(Message::UiSelect(UiComponent::MenuTabClose(index)));
    }

    fn handle_keys(&self, ctx: &Context) {
        let mut input_state = ctx.input_mut();

        // Just translate keys to clicks here
        if input_state.consume_key(Modifiers::CTRL, Key::S) {
            if let Some(tab) = self.state.current_tab() {
                if tab.is_project() {
                    self.on_clicked_save();
                } else {
                    self.on_clicked_save_as();
                }
            }
        } else if input_state.consume_key(Modifiers::CTRL, Key::O) {
            self.on_clicked_open();
        }
    }

    fn in_project_tab(&self) -> bool {
        self.state.in_project_tab()
    }

    fn on_clicked_export(&self) {
        if let Some(message_sender) = self.message_sender().cloned() {
            io_utils::save_file("/", &[("NMD File", &["nmd"])], move |path| {
                message_sender.send(Message::UiSelect(UiComponent::MenuExport(path)));
            });
        }
    }

    fn on_clicked_import(&self) {
        if let Some(message_sender) = self.message_sender().cloned() {
            io_utils::open_file("/", &[("NMD File", &["nmd"])], move |path| {
                message_sender.send(Message::UiSelect(UiComponent::MenuImport(path)));
            });
        }
    }

    fn on_clicked_open(&self) {
        if let Some(message_sender) = self.message_sender().cloned() {
            io_utils::open_file("/", &[("NMD Project File", &["nmde"])], move |path| {
                message_sender.send(Message::UiSelect(UiComponent::MenuProjectOpen(path)));
            });
        }
    }

    fn on_clicked_save_as(&self) {
        if let Some(message_sender) = self.message_sender().cloned() {
            io_utils::save_file("/", &[("NMD Project File", &["nmde"])], move |path| {
                message_sender.send(Message::UiSelect(UiComponent::MenuProjectSaveAs(path)));
            });
        }
    }

    fn on_clicked_save(&self) {
        // Confirm we're in a project to not overwrite an imported file
        if self.state.in_project_tab() {
            if let Some(project_path) = self.state.current_tab_path() {
                self.emit_save_as(project_path);
            }
        }
    }

    pub fn most_recent_tab(&self) -> Option<&MenuTab> {
        self.state.most_recent_tab()
    }

    pub fn most_recent_tab_mut(&mut self) -> Option<&mut MenuTab> {
        self.state.most_recent_tab_mut()
    }

    pub fn push_project_tab(&mut self, path: &PathBuf) {
        self.state.tabs.push(MenuTab::for_project(path));
    }

    pub fn push_tab(&mut self, path: &PathBuf) {
        self.state.tabs.push(MenuTab::for_file(path));
    }

    pub fn select_tab(&mut self, index: usize) {
        if index < self.state.tabs.len() {
            self.state.tab_index_opt = Some(index);
        }
    }

    pub fn remove_tab(&mut self, index: usize) {
        self.state.remove_tab(index);
    }

    fn ui_menu(&mut self, ui: &mut Ui) {
        self.ui_menu_button_file(ui);
        self.ui_menu_button_project(ui);
        self.ui_menu_button_view(ui);
        self.ui_menu_button_help(ui);
    }

    #[inline]
    fn ui_menu_button_file(&mut self, ui: &mut Ui) {
        ui.menu_button("File", |ui| {
            if ui.button("Import…").clicked() {
                ui.close_menu();

                self.on_clicked_import();
            }

            ui.separator();

            ui.scope(|ui| {
                ui.set_enabled(self.state.in_tab());

                ui.menu_button("Export as NMD", |ui| {
                    ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                        ui.add_space(3.0);
                        ui.label("Format…");
                        ui.separator();
                    });

                    ui.scope(|ui| {
                        ui.set_enabled(false /* WIP */);

                        if ui.button("SCIV").clicked() {
                            ui.close_menu();
                        }

                        if ui.button("SCV").clicked() {
                            ui.close_menu();
                        }
                    });

                    if ui.button("SCVI").clicked() {
                        self.on_clicked_export();

                        ui.close_menu();
                    }
                });
            });

            ui.add_space(2.0);
        });
    }

    #[inline]
    fn ui_menu_button_help(&mut self, ui: &mut Ui) {
        ui.menu_button("Help", |ui| {
            ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                ui.add_space(3.0);
                ui.label("Tips…");
            });

            ui.separator();

            ui.button("Collapsers")
                .on_hover_cursor(CursorIcon::Help)
                .on_hover_ui(|ui|
            {
                let mut text = LayoutJob::default();

                text.append(
                    concat!(
                        "Right-click a tree collapser to recursively expand or collapse it."
                    ),
                    0.0, TextFormat::default());

                ui.label(text);
            });

            ui.button("Filters")
                .on_hover_cursor(CursorIcon::Help)
                .on_hover_ui(|ui|
            {
                let mut text = LayoutJob::default();

                text.append(
                    concat!(
                        "Filter expressions are by default reductive, e.g."
                    ),
                    0.0, TextFormat::default());
                text.append(
                    concat!(
                        "\n\n",
                        "    A B         = CONTAINS(A) AND CONTAINS(B)"
                    ),
                    0.0, TextFormat { font_id: FontId::monospace(14.0), ..Default::default() });
                text.append(
                    concat!(
                        "\n\n",
                        "In addition, an OR operator exists (also showcasing parentheses):"
                    ),
                    0.0, TextFormat::default());
                text.append(
                    concat!(
                        "\n\n",
                        "    A | B       = CONTAINS(A) OR CONTAINS(B)\n",
                        "    A | (B C)   = CONTAINS(A) OR (CONTAINS(B) AND CONTAINS(C))"
                    ),
                    0.0, TextFormat { font_id: FontId::monospace(14.0), ..Default::default() });
                text.append(
                    concat!(
                        "\n\n",
                        "Some additional syntax exists for property searching:"
                    ),
                    0.0, TextFormat::default());
                text.append(
                    concat!(
                        "\n\n",
                        "    $rot        = TYPE_CONTAINS(rot)\n",
                        "    #123        = ID_EQUALS(123)\n",
                        "    A $sw       = CONTAINS(A) AND TYPE_CONTAINS(sw)"
                    ),
                    0.0, TextFormat { font_id: FontId::monospace(14.0), ..Default::default() });

                ui.label(text);
            });

            ui.button("Input fields")
                .on_hover_cursor(CursorIcon::Help)
                .on_hover_ui(|ui|
            {
                let mut text = LayoutJob::default();

                text.append(
                    concat!(
                        "Submit an empty input field to revert it to its original value.",
                    ),
                    0.0, TextFormat::default());

                ui.label(text);
            });

            ui.button("Roots")
                .on_hover_cursor(CursorIcon::Help)
                .on_hover_ui(|ui|
            {
                let mut text = LayoutJob::default();

                text.append(
                    concat!(
                        "Right-click and",
                        r#" select "root" on"#,
                        " a bone in the tree to treat it temporarily as though",
                        " it's the root of the tree.",
                        "\n\nThis can have the effect of reducing visual clutter, but otherwise does nothing."
                    ),
                    0.0, TextFormat::default());

                ui.label(text);
            });

            ui.separator();

            ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                ui.label("Key bindings…");
            });

            ui.separator();

            ui.button("Navigation")
                .on_hover_cursor(CursorIcon::Help)
                .on_hover_ui(|ui|
            {
                let mut text = LayoutJob::default();

                text.append(
                    concat!(
                        r#"G           … Scroll to current"#, "\n",
                        r#"H           … Clear "spotlight""#, "\n",
                        r#"B           … Select previous in chain"#, "\n",
                        r#"N           … Select next in chain"#, "\n",
                        r#"V           … Toggle view"#, "\n",
                        r#"Shift-V     … Toggle view and scroll to current"#
                    ),
                    0.0, TextFormat { font_id: FontId::monospace(14.0), ..Default::default() });

                ui.label(text);
            });

            ui.button("Project")
                .on_hover_cursor(CursorIcon::Help)
                .on_hover_ui(|ui|
            {
                let mut text = LayoutJob::default();

                text.append(
                    concat!(
                        r#"Ctrl-O      … Open"#, "\n",
                        r#"Ctrl-S      … Save (as…)"#,
                    ),
                    0.0, TextFormat { font_id: FontId::monospace(14.0), ..Default::default() });

                ui.label(text);
            });

            ui.separator();

            ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
                ui.hyperlink_to("nmde v0.1", "https://github.com/lumlune/nmde");
                ui.add_space(1.0);
            });
        });
    }

    #[inline]
    fn ui_menu_button_project(&mut self, ui: &mut Ui) {
        ui.menu_button("Project", |ui| {
            let in_project_tab = self.state.in_project_tab();
            let mut save_button;

            if ui.button("Open…").clicked() {
                self.on_clicked_open();

                ui.close_menu();
            }

            ui.separator();

            save_button = ui.add_enabled(
                in_project_tab,
                Button::new("Save"));

            if in_project_tab {
                if let Some(path) = self.state.current_tab_path() {
                    save_button = save_button
                        .on_hover_cursor(CursorIcon::Help)
                        .on_hover_ui_at_pointer(|ui|
                    {
                        let mut hover_text = LayoutJob::default();

                        hover_text.append("Target: ",
                            0.0, TextFormat::default());
                        hover_text.append(&path_to_string(path),
                            0.0, TextFormat { color: (*UiColor).common.weak_gray.normal(), ..Default::default() });

                        ui.label(hover_text);
                    });
                }

                if save_button.clicked() {
                    self.on_clicked_save();

                    ui.close_menu();
                }
            }

            if ui.add_enabled(
                self.state.in_tab(),
                Button::new("Save as…")
            ).clicked() {
                self.on_clicked_save_as();

                ui.close_menu();
            }

            ui.separator();

            if ui.add_enabled(
                self.state.in_edited_tab(),
                Button::new("Unmark edits")
            ).on_hover_cursor(CursorIcon::Help)
             .on_hover_text("Remove the color from edited elements, making them appear original.")
             .clicked() {
                self.emit_commit();

                ui.close_menu();
            }

            ui.add_space(1.0);
        });
    }

    #[inline]
    fn ui_menu_button_view(&mut self, ui: &mut Ui) {
        ui.menu_button("View", |ui| {
            let (tab_hiding_ids, tab_is_list) = if let Some(tab) = self.state.current_tab() {
                (tab.hiding_ids(), tab.view() == MenuTabView::List)
            } else {
                (false, false)
            };

            if ui.add_enabled(
                tab_is_list,
                Button::new(if tab_hiding_ids {
                    "Show list IDs" 
                } else {
                    "Hide list IDs"
                })
            ).clicked() {
                if let Some(mut tab) = self.state.current_tab_mut() {
                    tab.set_hiding_ids(!tab_hiding_ids);
                    self.emit_hide_ids(!tab_hiding_ids);
                }

                ui.close_menu();
            }
        });
    }

    fn ui_menu_style(&self, ui: &mut Ui) {
        ui.visuals_mut().widgets.inactive.bg_fill = Color32::TRANSPARENT;
    }

    fn ui_tabs(&mut self, ui: &mut Ui) {
        if let Some(mut tab_index) = self.state.tab_index_opt.to_owned() {
            let mut i = 0;

            for tab in &self.state.tabs {
                let tab_button;

                if i > 0 {
                    ui.separator();
                }

                tab_button = ui.selectable_value(&mut tab_index, i, tab.name())
                    .on_hover_cursor(CursorIcon::PointingHand)
                    .on_hover_ui_at_pointer(|ui| { ui.label(&path_to_string(tab.path())); })
                    .context_menu(|ui|
                {
                    if ui.button("Close").clicked() {
                        self.emit_tab_close(i);

                        ui.close_menu();
                    }
                });

                if tab_button.clicked() {
                    self.emit_tab(i);
                }

                i += 1;
            }
        }
    }

    fn ui_tabs_style(&self, ui: &mut Ui) {
        ui.spacing_mut().button_padding.y = 6.0;
        ui.visuals_mut().selection.bg_fill = (*UiColor).menu.selected_tab.normal();
    }
}

impl NmdAppRegion for NmdAppMenuRegion {
    fn message_sender(&self) -> Option<&MessageSender> {
        self.message_sender.as_ref()
    }

    fn receive_message(&mut self, message: &Message) {
        match message {
            Message::UiState(UiState::TreeEditStatus(edited))
                => { self.state.set_current_tab_edited(*edited); }
            Message::UiState(UiState::TreeNodeViewChanged(node_view))
                => { self.state.set_current_tab_view(*node_view); }
            _   => {}
        }
    }

    fn ui(&mut self, ctx: &Context) {
        self.handle_keys(ctx);

        TopBottomPanel::top("region$top")
            .show(ctx, |ui|
        {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                self.ui_menu_style(ui);
                self.ui_menu(ui);
            });

            if !self.state.tabs.is_empty() {
                ui.separator();
                ui.horizontal(|ui| {
                    self.ui_tabs_style(ui);
                    self.ui_tabs(ui);
                });
            }

            ui.add_space(1.0);
        });
    }
}

impl NmdAppMenuProjectState {
    fn current_tab(&self) -> Option<&MenuTab> {
        self.tabs.get(self.tab_index_opt?)
    }

    fn current_tab_mut(&mut self) -> Option<&mut MenuTab> {
        self.tabs.get_mut(self.tab_index_opt?)
    }

    fn current_tab_name(&self) -> Option<&String> {
        Some(self.current_tab()?.name())
    }

    fn current_tab_path(&self) -> Option<&PathBuf> {
        Some(self.current_tab()?.path())
    }

    fn in_edited_tab(&self) -> bool {
        if let Some(tab) = self.current_tab() {
            tab.edited()
        } else {
            false
        }
    }

    fn in_project_tab(&self) -> bool {
        if let Some(tab) = self.current_tab() {
            tab.is_project()
        } else {
            false
        }
    }

    fn in_tab(&self) -> bool {
        !self.tabs.is_empty()
    }

    fn set_current_tab_edited(&mut self, edited: bool) {
        if let Some(tab) = self.current_tab_mut() {
            tab.set_edited(edited);
        }
    }

    fn set_current_tab_view(&mut self, view: MenuTabView) {
        if let Some(tab) = self.current_tab_mut() {
            tab.set_view(view);
        }
    }

    fn most_recent_tab(&self) -> Option<&MenuTab> {
        self.tabs.last()
    }

    fn most_recent_tab_mut(&mut self) -> Option<&mut MenuTab> {
        self.tabs.last_mut()
    }

    fn remove_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);

            if let Some(current_index) = self.tab_index_opt {
                match index.cmp(&current_index) {
                    Ordering::Less
                        => self.tab_index_opt = current_index.checked_sub(1),
                    Ordering::Equal
                        if index == self.tabs.len()
                        => self.tab_index_opt = current_index.checked_sub(1),
                    _   => {}
                }
            }
        }
    }
}

impl MenuTab {
    fn data(&self) -> &MenuTabData {
        match self {
            MenuTab::File(tab_data) => tab_data,
            MenuTab::Project(tab_data) => tab_data,
        }
    }

    fn data_mut(&mut self) -> &mut MenuTabData {
        match self {
            MenuTab::File(tab_data) => tab_data,
            MenuTab::Project(tab_data) => tab_data,
        }
    }

    fn edited(&self) -> bool {
        self.data().edited
    }

    fn hiding_ids(&self) -> bool {
        self.data().hiding_ids
    }

    fn for_file(path: &PathBuf) -> Self {
        MenuTab::File(MenuTabData::from(path))
    }

    fn for_project(path: &PathBuf) -> Self {
        MenuTab::Project(MenuTabData::from(path))
    }

    fn to_project_tab(&self, path: &PathBuf) -> Self {
        let mut tab = Self::for_project(path);

        tab.set_edited(self.edited());
        tab.set_hiding_ids(self.hiding_ids());
        tab.set_view(self.view());
        tab
    }

    fn is_project(&self) -> bool {
        matches!(self, MenuTab::Project(_))
    }

    fn name(&self) -> &String {
        &self.data().name
    }

    fn path(&self) -> &PathBuf {
        &self.data().path
    }

    pub fn set_edited(&mut self, edited: bool) {
        self.data_mut().edited = edited;
    }

    pub fn set_hiding_ids(&mut self, hiding_ids: bool) {
        self.data_mut().hiding_ids = hiding_ids;
    }

    pub fn set_view(&mut self, view: MenuTabView) {
        self.data_mut().view = view;
    }

    fn view(&self) -> MenuTabView {
        self.data().view
    }
}

impl From<&PathBuf> for MenuTabData {
    fn from(path: &PathBuf) -> Self {
        Self {
            edited: false,
            hiding_ids: false,
            name: path_to_file_name(path),
            path: path.to_owned(),
            view: MenuTabView::Tree,
        }
    }
}

fn path_to_file_name(path: &PathBuf) -> String {
    path.file_name()
        .unwrap_or_else(|| OsStr::new("?"))
        .to_string_lossy()
        .into_owned()
}

fn path_to_string(path: &PathBuf) -> String {
    path.to_string_lossy()
        .into_owned()
}
