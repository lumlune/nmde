use {
    crate::{
        io::{
            fifo::{
                Message,
                MessageSender,
            },
            nmd::{
                data::NmdFileData,
                NmdFile,
            },
        },
        ui::{
            *,
            region::*,
        },
    },
    std::{
        cmp::Ordering,
        fs::File,
        io::{Error, ErrorKind, Result, Write},
        path::PathBuf,
    },
    eframe::egui::{Context, Id},
    serde::{
        Deserialize,
        Serialize,
    },
};

pub struct NmdAppView {
    message_sender: MessageSender,
    regions: NmdAppSubRegions,
    view_index_opt: Option<usize>,
}

struct NmdAppSubRegions {
    data: Vec<NmdAppProjectView>,
    home: NmdAppHomeRegion,
    menu: NmdAppMenuRegion,
}

#[derive(Serialize, Deserialize)]
struct NmdAppProjectView {
    regions: NmdAppProjectSubRegions,
    state: NmdAppDataProjectState,
}

#[derive(Serialize, Deserialize)]
struct NmdAppProjectSubRegions {
    editor: NmdAppEditorRegion,
    tree: NmdAppTreeRegion,
}

#[derive(Serialize, Deserialize)]
struct NmdAppDataProjectState {
    file_data: NmdFileData,
    project_path_opt: Option<PathBuf>,
}

impl NmdAppView {
    pub fn new(message_sender: &MessageSender) -> Self {
        Self {
            message_sender: message_sender.to_owned(),
            regions: NmdAppSubRegions {
                data: vec![],
                home: NmdAppHomeRegion::new(message_sender),
                menu: NmdAppMenuRegion::new(message_sender),
            },
            view_index_opt: None,
        }
    }

    fn conform_menu_to_project_view(menu: &mut NmdAppMenuRegion, project_view: &NmdAppProjectView) {
        if let Some(tab) = menu.most_recent_tab_mut() {
            tab.set_edited(project_view.regions.tree.modified());
            tab.set_hiding_ids(project_view.regions.tree.hiding_ids());
            tab.set_view(project_view.regions.tree.view());
        }
    }

    fn current_project_view(&self) -> Option<&NmdAppProjectView> {
        self.regions.data.get(self.view_index_opt?)
    }

    fn current_project_view_mut(&mut self) -> Option<&mut NmdAppProjectView> {
        self.regions.data.get_mut(self.view_index_opt?)
    }

    fn push_project_view(&mut self, project_path: &PathBuf, project_view: NmdAppProjectView) {
        self.regions.menu.push_project_tab(project_path);
        Self::conform_menu_to_project_view(&mut self.regions.menu, &project_view);
        self.regions.data.push(project_view);
    }

    fn remove_project_view(&mut self, index: usize) {
        if index < self.regions.data.len() {
            self.regions.data.remove(index);
            self.regions.menu.remove_tab(index);

            if let Some(current_index) = self.view_index_opt {
                match index.cmp(&current_index) {
                    Ordering::Less
                        => self.view_index_opt = current_index.checked_sub(1),
                    Ordering::Equal
                        if index == self.regions.data.len()
                        => self.view_index_opt = current_index.checked_sub(1),
                    _   => {}
                }
            }
        }
    }

    pub fn show_newest(&mut self) {
        self.set_view_index(self.regions.data.len().wrapping_sub(1));
    }

    fn set_view_index(&mut self, index: usize) {
        if index < self.regions.data.len() {
            self.view_index_opt = Some(index);

            self.regions.menu.select_tab(index);
        }
    }

    pub fn try_export(&mut self, path: &PathBuf) -> Result<()> {
        if let Some(project_view) = self.current_project_view_mut() {
            project_view.regions.editor
                .try_export(path, &project_view.state.file_data)
        } else {
            Err(Error::from(ErrorKind::Other))
        }
    }

    pub fn try_import(&mut self, path: &PathBuf) -> Result<()> {
        let data = NmdFile::try_from(path)?.data;

        self.regions.data.push(NmdAppProjectView::new(&self.message_sender, data));
        self.regions.menu.push_tab(path);

        Ok(())
    }

    pub fn try_open(&mut self, project_path: &PathBuf) -> Result<()> {
        match File::open(project_path) {
            Ok(project_file) => {
                match serde_json::from_reader::<_, NmdAppProjectView>(&project_file) {
                    Ok(mut project_view) => {
                        project_view.emit_with(&self.message_sender);
                        project_view.on_serialized(project_path);

                        self.push_project_view(project_path, project_view);

                        Ok(())
                    }
                    Err(serde_error) => Err(Error::new(ErrorKind::Other, serde_error)),
                }
            }
            Err(error) => Err(error)
        }
    }

    pub fn try_save_as(&mut self, project_path: &PathBuf) -> Result<()> {
        if let Some(project_view) = self.current_project_view_mut() {
            match File::create(project_path) {
                Ok(project_file) => {
                    match serde_json::to_writer(&project_file, &project_view) {
                        Err(serde_error) => Err(Error::new(ErrorKind::Other, serde_error)),
                        _ => Ok(())
                    }
                }
                Err(error) => Err(error)
            }
        } else {
            Err(Error::new(ErrorKind::Other, "Tried to save an empty view"))
        }
    }
}

impl NmdAppRegion for NmdAppView {
    fn receive_message(&mut self, message: &Message) {
        self.regions.menu.receive_message(message);

        if let Some(project_view) = self.current_project_view_mut() {
            project_view.receive_message(message);
        }
    }

    fn select(&mut self, ui_component: &UiComponent) {
        match ui_component {
            UiComponent::MenuExport(path)
                => { self.try_export(path); }
            UiComponent::MenuImport(path)
                => { if self.try_import(path).is_ok()  { self.show_newest(); } }
            UiComponent::MenuProjectOpen(path)
                => { if self.try_open(path).is_ok()    { self.show_newest(); } }
            UiComponent::MenuProjectSaveAs(path)
                => { if self.try_save_as(path).is_ok() { self.regions.menu.assign_tab_to_project(path); }; }
            UiComponent::MenuTab(index)
                => { self.set_view_index(*index); }
            UiComponent::MenuTabClose(index)
                => { self.remove_project_view(*index); }
            _   => {}
        }

        self.regions.menu.select(ui_component);

        if let Some(project_view) = self.current_project_view_mut() {
            project_view.select(ui_component);
        }
    }

    fn ui(&mut self, ctx: &Context) {
        self.regions.menu.ui(ctx);

        if let Some(project_view) = self.current_project_view_mut() {
            project_view.ui(ctx);
        } else {
            self.regions.home.ui(ctx);
        }
    }
}

impl NmdAppProjectView {
    fn new(message_sender: &MessageSender, data: NmdFileData) -> Self {
        Self {
            regions: NmdAppProjectSubRegions::new(message_sender, &data),
            state: NmdAppDataProjectState::from(data),
        }
    }

    fn emit_with(&mut self, message_sender: &MessageSender) {
        self.regions.tree.emit_with(message_sender);
        self.regions.editor.emit_with(message_sender);
    }

    fn on_serialized(&mut self, project_path: &PathBuf) {
        self.regions.tree.on_serialized();

        self.state.project_path_opt = Some(project_path.to_owned());
    }
}

impl NmdAppRegion for NmdAppProjectView {
    fn receive_message(&mut self, message: &Message) {
        self.regions.tree.receive_message(message);
        self.regions.editor.receive_message(message);
    }

    fn select(&mut self, ui_component: &UiComponent) {
        self.regions.tree.select(ui_component);
        self.regions.editor.select(ui_component);
    }

    fn ui(&mut self, ctx: &Context) {
        self.regions.tree.ui(ctx);
        self.regions.editor.ui(ctx);
    }
}

impl NmdAppProjectSubRegions {
    fn new(message_sender: &MessageSender, data: &NmdFileData) -> Self {
        Self {
            editor: NmdAppEditorRegion::new(message_sender, data),
            tree: NmdAppTreeRegion::new(message_sender, data),
        }
    }
}

impl From<NmdFileData> for NmdAppDataProjectState {
    fn from(data: NmdFileData) -> Self {
        Self {
            file_data: data,
            project_path_opt: None,
        }
    }
}
