use {
    crate::{
        io::{
            fifo::{
                Message,
                MessageSender,
            },
            nmd::{
                anatomy::{NmdFileBone, NmdFileBoneFlag},
                data::NmdFileData,
                data::tree::{
                    *,
                    NmdFileBoneTreeNode as Tree,
                },
            },
        },
        ui::{
            region,
            region::NmdAppRegion,
            UiComponent,
            UiColor,
            UiStyle,
            UiState,
        },
        utils::filter::*,
        utils::iter,
    },
    std::{
        cell::{
            Ref,
            RefCell,
            RefMut,
        },
        cmp::Ordering,
        collections::{
            BTreeMap,
            BTreeSet,
            HashMap,
            HashSet,
            VecDeque,
        },
        iter::from_fn as iter_from,
        fmt::{self, Display, Formatter},
        mem,
        rc::Rc,
    },
    eframe::{
        egui::{
            *,
            collapsing_header::CollapsingState,
            output::CursorIcon,
            text::LayoutJob,
        },
        epaint::text::TextWrapping,
    },
    serde::{
        ser,
        Deserialize,
        Serialize,
    },
};

/*
 * TODO:
 * ~ Make context menus appear in bounds
 * ~ Spotlight logic: when to `cancel_spotlight()`, etc. (TODO FEAT:SPOTLIGHT)
 */

macro_rules! ui_id {
    ($region:ident, $node:ident) => {
        $region.uuid().with($node.id)
    };
}

type NodeFilterSet = BTreeSet<Rc<RefCell<Node>>>;
type NodeStateSet = BTreeSet<NodeState>;
type NodeTree = NmdFileBoneTree<NodeWrapper>;
type NodeTreeRoot = NmdFileBoneTreeRoot<NodeWrapper>;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct DimHighlight(bool);
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Edited(bool);
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct ModifiedFirst(bool);
#[derive(Debug, Clone, Serialize, Deserialize)]
struct NodeSummary(u16, String);
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
struct Recursed(bool);
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
struct Recursive(bool);

#[derive(Debug, Clone)]
enum NodeExpand {
    ToggleBeneath(u16, Option<bool>),
    Set(HashSet<u16>),
    Nil,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
enum NodePasteMode {
    Copy(Recursive, usize),
    Cut,
    Nil,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
enum NodeSortMode {
    Id(ModifiedFirst),
    Name(ModifiedFirst),
    Type(ModifiedFirst),
    Nil,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
enum NodeState {
    Filtered,
    FilteredAncestor,
    Modified,
    CopyPasted,
    CutPasted,
    ModifiedAncestor(u16),
    CopyPastedAncestor(u16),
    CutPastedAncestor(u16),
    Nil,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum NodeStateSetOperation {
    Difference,
    Union,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum NodeView {
    List,
    Tree,
}

#[derive(Serialize, Deserialize)]
pub struct NmdAppTreeRegion {
    #[serde(skip)]
    message_sender: Option<MessageSender>,
    state: NmdAppTreeProjectState,
    #[serde(skip)]
    transient_state: NmdAppTreeTransientState,
    uuid_source: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct NmdAppTreeProjectState {
    #[serde(skip)]
    filter: NodeFilter,
    filter_text: String,
    filter_visit_cache: HashMap<u16, bool>,
    // This doesn't really need to be an array
    edit_status: [bool; 2],
    edit_status_frozen: bool,
    hiding_ids: bool,
    history: VecDeque<NodeSummary>,
    ids: BTreeSet<u16>,
    #[serde(skip)]
    list: Vec<NodeWrapper>,
    natural_root: Option<u16>,
    paste: Option<NodeSummary>,
    paste_mode: NodePasteMode,
    pins: Vec<NodePin>,
    tree: NodeTreeRoot,
    roots: Vec<u16>,
    selection: Option<u16>,
    sort_mode: NodeSortMode,
    spotlight: Option<NodeSummary>,
    view: NodeView,
}

#[derive(Default)]
struct NmdAppTreeTransientState {
    expand: NodeExpand,
    rooted_cursor: Option<Pos2>,
    scroll_id: Option<u16>,
    scroll_initialized: bool,
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct Node {
    id: u16,
    flag: NmdFileBoneFlag,
    name: String,
    normalized_id: String,
    normalized_flag: String,
    normalized_name: String,
}

#[derive(Default)]
struct NodeFilter {
    collection: NodeFilterSet,
}

#[derive(Default, Serialize, Deserialize)]
struct NodeMetadata {
    copied: bool,
    cut: bool,
    hidden: bool,
    filtered: NodeStateSet,
    modified: NodeStateSet,
}

#[derive(Default, Serialize, Deserialize)]
struct NodePin {
    id: u16,
    display_name: String,
    name: String,
    path: Vec<u16>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct NodeWrapper {
    node: Rc<RefCell<Node>>,
    metadata: Rc<RefCell<NodeMetadata>>,
}

impl NmdAppTreeRegion {
    const MIN_WIDTH: f32 = 230.0;
    const MAX_WIDTH: f32 = 400.0;

    pub fn new(message_sender: &MessageSender, data: &NmdFileData) -> Self {
        Self {
            message_sender: Some(message_sender.to_owned()),
            state: NmdAppTreeProjectState::from(data),
            transient_state: Default::default(),
            uuid_source: region::generate_uuid_source(),
        }
    }

    fn edited(&self) -> bool {
        self.state.status() == Edited(true)
    }

    fn emit_clear_filter(&self) {
        self.emit(Message::UiSelect(UiComponent::TreeFilterClear));
    }

    fn emit_edit_status(&self, edited: bool) {
        self.emit(Message::UiState(UiState::TreeEditStatus(edited)));
    }

    fn emit_copy(&self, node_id: u16, node_name: &String) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeCopy(node_id, node_name.to_owned())));
    }

    fn emit_copy_single(&self, node_id: u16, node_name: &String) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeCopySingle(node_id, node_name.to_owned())));
    }

    fn emit_cut(&self, node_id: u16, node_name: &String) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeCut(node_id, node_name.to_owned())));
    }

    fn emit_delete(&self, node_id: u16) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeDelete(node_id)));
    }

    fn emit_deleted(&self, root_id: u16, node_ids: HashSet<u16>) {
        self.emit(Message::UiState(UiState::TreeNodeDelete(root_id, node_ids)));
    }

    fn emit_expand_node(&self, node_id: u16) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeExpansion(node_id)));
    }

    fn emit_filter_to(&self, node_id: u16) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeFilterTo(node_id)));
    }

    fn emit_focus(&self, node_id: u16, node_name: &String) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeFocus(node_id, node_name.to_owned())));
    }

    fn emit_paste(&self, node_id: u16) {
        self.emit(Message::UiSelect(UiComponent::TreeNodePaste(node_id)));
    }

    fn emit_paste_after(&self, node_id: u16) {
        self.emit(Message::UiSelect(UiComponent::TreeNodePasteAfter(node_id)));
    }

    fn emit_pin(&self, pin_id: u16, pin_name: &String) {
        self.emit(Message::UiSelect(UiComponent::TreeNodePin(pin_id, pin_name.to_owned())));
    }

    fn emit_remove_pin(&self, pin_id: u16) {
        self.emit(Message::UiSelect(UiComponent::TreeNodePinRemove(pin_id)));
    }

    fn emit_root(&self, node_id: u16) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeRoot(node_id)));
    }

    fn emit_scroll(&self, node_id: u16) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeScroll(node_id)));
    }

    fn emit_scroll_done(&self) {
        self.emit(Message::UiState(UiState::TreeNodeScrollDone));
    }

    fn emit_select_node(&self, node_id: u16, node_name: &String) {
        self.emit(Message::UiSelect(UiComponent::TreeNode(node_id, node_name.to_owned())));
    }

    fn emit_spotlight(&self, node_id: u16, node_name: &String) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeSpotlight(node_id, node_name.to_owned())));
    }

    fn emit_unroot(&self) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeUnroot));
    }

    fn emit_unroot_all(&self) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeUnrootAll));
    }

    fn emit_view(&self, node_view: NodeView) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeView(node_view)));
    }

    fn emit_view_changed(&self, node_view: NodeView) {
        self.emit(Message::UiState(UiState::TreeNodeViewChanged(node_view)));
    }

    pub fn emit_with(&mut self, message_sender: &MessageSender) {
        self.message_sender = Some(message_sender.to_owned());
    }

    fn expand(&mut self, node_id: u16) {
        self.transient_state.expand = NodeExpand::ToggleBeneath(node_id, None);
    }

    fn handle_keys(&mut self, ctx: &Context) {
        if ctx.memory().focus().is_none() {
            if ctx.input().key_released(Key::H) {
                // TODO FEAT:SPOTLIGHT
                self.state.cancel_spotlight();
            } else if ctx.input().key_released(Key::G) {
                self.scroll_to_selection();
            } else if ctx.input().key_released(Key::V) {
                self.toggle_view();

                if ctx.input().modifiers.shift {
                    self.scroll_to_selection();
                }
            }
        }
    }

    pub fn hiding_ids(&self) -> bool {
        self.state.hiding_ids
    }

    fn focus(&mut self, node_id: u16, node_name: &String) {
        self.scroll_to(node_id);

        // TODO FEAT:SPOTLIGHT
        if !self.state.selected(node_id) {
            self.state.spotlight(node_id, node_name);
        }
    }

    pub fn modified(&self) -> bool {
        self.state.modified()
    }

    fn on_pasted(&mut self, node_id: u16, pasted_state: UiState) {
        self.scroll_to(node_id);
        self.emit(Message::UiState(pasted_state));
    }

    pub fn on_serialized(&mut self) {
        self.state.finalize();
        self.scroll_to_selection();
    }

    fn scroll_to(&mut self, node_id: u16) {
        self.transient_state.scroll_id = Some(node_id);
        self.transient_state.scroll_initialized = true;
    }

    fn scroll_to_selection(&mut self) {
        if let Some(node_id) = self.state.selection {
            self.scroll_to(node_id);
        }
    }

    fn set_view(&mut self, node_view: NodeView) {
        self.state.view = node_view;
        self.emit_view_changed(node_view);
    }

    fn toggle_view(&mut self) {
        self.state.view = match self.state.view {
            NodeView::Tree => NodeView::List,
            NodeView::List => NodeView::Tree,
        };

        self.emit_view_changed(self.state.view);
    }

    fn ui_body(&mut self, ui: &mut Ui) {
        ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
            self.ui_node_status(ui);

            ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
                match self.state.view {
                    NodeView::Tree => {
                        ScrollArea::both()
                            .id_source("region$tree$scroll")
                            .show(ui, |ui|
                        {
                            self.ui_body_style(ui);
                            self.ui_root(ui);
                            self.ui_tree(ui);
                        });
                    }
                    NodeView::List => {
                        ScrollArea::both()
                            .id_source("region$list$scroll")
                            .show(ui, |ui|
                        {
                            self.ui_body_style(ui);
                            self.ui_list(ui);
                        });
                    }
                }
            });
        });
    }

    fn ui_body_style(&self, ui: &mut Ui) {
        ui.visuals_mut().widgets = (*UiStyle).interactive_text.to_owned();
        ui.set_min_size(ui.available_size());
    }

    fn ui_expand(&mut self, collapser: &mut CollapsingState, node: &Ref<Node>) {
        match &mut self.transient_state.expand {
            NodeExpand::Nil => {}
            NodeExpand::ToggleBeneath(_, Some(open)) => {
                collapser.set_open(*open);
            }
            NodeExpand::ToggleBeneath(node_id, None) => {
                if *node_id == node.id {
                    let open = !collapser.is_open();

                    collapser.set_open(open);
                    self.transient_state.expand = NodeExpand::ToggleBeneath(*node_id, Some(open));
                }
            }
            NodeExpand::Set(open_set) => {
                if open_set.remove(&node.id) {
                    collapser.set_open(true);
                }
            }
        }
    }

    fn ui_expand_undo(&mut self, node: &Ref<Node>) {
        match &self.transient_state.expand {
            NodeExpand::ToggleBeneath(node_id, _) => {
                if *node_id == node.id {
                    self.transient_state.expand = NodeExpand::Nil;
                }
            }
            _ => {}
        }
    }

    fn ui_filter(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.set_width(ui.available_width());
            ui.label("Filter:");

            ui.with_layout(Layout::right_to_left(), |ui| {
                if ui.button("ｘ").clicked() {
                    self.state.clear_filter();
                }

                if ui.add(
                    TextEdit::singleline(&mut self.state.filter_text)
                        .desired_width(ui.available_width())
                        .hint_text("Filter text")
                ).changed() {
                    self.state.filter();
                }
            });
        });
    }

    fn ui_highlight(&self, ui: &mut Ui, node: &Ref<Node>, metadata: &Ref<NodeMetadata>, DimHighlight(use_dims): DimHighlight) {
        use NodeState::*;

        let stroke = if self.state.spotlighted(node.id) {
            (*UiColor).tree.spotlighted.normal_stroke()
        } else {
            let state_set = if self.state.filtered() {
                &metadata.filtered
            } else {
                &metadata.modified
            };

            match state_set
                .iter()
                .next() // First in an ordered set
                .unwrap_or(&NodeState::Nil)
            {
                Filtered                => { (*UiColor).tree.filtered.normal_stroke() }
                FilteredAncestor
                    if use_dims         => { (*UiColor).common.near_weak_gray.normal_stroke() }
                _
                    if metadata.copied  => { (*UiColor).tree.copied.normal_stroke() }
                _
                    if metadata.cut     => { (*UiColor).common.near_weak_gray.normal_stroke() }
                Modified                => { (*UiColor).tree.modified.normal_stroke() }
                ModifiedAncestor(_)
                    if use_dims         => { (*UiColor).tree.modified.dim_stroke() }
                CopyPasted              => { (*UiColor).tree.copy_pasted.normal_stroke() }
                CopyPastedAncestor(_)
                    if use_dims
                                        => { (*UiColor).tree.copy_pasted.dim_stroke() }
                CutPasted               => { (*UiColor).tree.cut_pasted.normal_stroke() }
                CutPastedAncestor(_)
                    if use_dims
                                        => { (*UiColor).tree.cut_pasted.dim_stroke() }
                _                       => { (*UiColor).common.light_gray.normal_stroke() }
            }
        };

        let widget_style = &mut ui.style_mut().visuals.widgets;

        widget_style.inactive.fg_stroke = stroke;
        
        if self.state.selected(node.id) {
            widget_style.active
                .bg_fill = (*UiColor).tree.selected.normal();
            widget_style.inactive
                .bg_fill = (*UiColor).tree.selected.normal();
            widget_style.hovered
                .bg_fill = (*UiColor).tree.selected.normal();
        }
    }

    fn ui_history(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let prev_button = ui.add_enabled(self.state.history.len() > 1, Button::new("Previous"))
                .on_hover_cursor(CursorIcon::PointingHand);

            if let Some(NodeSummary(node_id, node_name)) = self.state.history.get(1) {
                if prev_button.clicked() {
                    self.emit_select_node(*node_id, node_name);
                } else if prev_button.secondary_clicked() {
                    self.emit_focus(*node_id, node_name);
                }

                prev_button.on_hover_text_at_pointer(node_name);
            }

            ComboBox::from_id_source("region$tree$history")
                .selected_text("History")
                .width(ui.available_width() - (ui.spacing().item_spacing.x + 0.5))
                .show_ui(ui, |ui|
            {
                if self.state.has_history() {
                    let mut selected = f32::NAN;

                    for (i, NodeSummary(node_id, node_name)) in self.state.history.iter().enumerate() {
                        let entry;
                        let mut entry_text = LayoutJob::single_section(
                            node_name.to_owned(),
                            TextFormat {
                                // Normal label text as opposed to the
                                // slightly-lighter job text
                                color: (*UiColor).common.light_gray.normal(),
                                ..Default::default()
                            }
                        );

                        ui.style_mut().wrap = Some(true);

                        // Without `break_anywhere` can be unpredictable
                        entry_text.wrap.break_anywhere = true;
                        entry_text.wrap.max_rows = 1;

                        if i == 1 {
                            ui.separator();
                        }

                        // Reflexive non-equal value `NAN` to subvert highlighting
                        entry = ui.selectable_value(&mut selected, f32::NAN, entry_text)
                            .on_hover_text_at_pointer(node_name);

                        if entry.clicked() {
                            self.emit_select_node(*node_id, node_name);
                        } else if entry.secondary_clicked() {
                            self.emit_focus(*node_id, node_name);
                        }
                    }
                } else {
                    ui.label(RichText::new("Nothing here").weak());
                }
            });
        });
    }

    fn ui_list(&self, ui: &mut Ui) {
        let column_count = if self.state.hiding_ids {
            2
        } else {
            3
        };

        Grid::new("region$tree$list")
            .striped(true)
            .num_columns(column_count)
            .show(ui, |ui|
        {
            for node_wrapper in &self.state.list {
                let (node, metadata) = node_wrapper.as_tuple();

                if self.state.expect_in_view(&metadata) {
                    if !self.state.hiding_ids {
                        ui.label(&node.normalized_id);
                    }

                    ui.label(&node.normalized_flag);

                    self.ui_list_node(ui, &node, &metadata);

                    ui.end_row();
                }
            }
        });
    }

    fn ui_list_node(&self, ui: &mut Ui, node: &Ref<Node>, metadata: &Ref<NodeMetadata>) {
        ui.scope(|ui| {
            let node_button;

            self.ui_highlight(ui, node, metadata, DimHighlight(false));

            node_button = ui.button(&node.name)
                .on_hover_cursor(CursorIcon::PointingHand)
                .context_menu(|ui|
            {
                self.ui_list_node_menu(ui, node);
            });

            if node_button.clicked() {
                self.emit_select_node(node.id, &node.name);
            }

            if self.transient_state.pending_scroll(node.id) {
                let mut rect = node_button.rect;

                // Describe an out-of-bounds rect to get the scroll to snap
                // left, since unsure where we are in the scroll area
                rect.min.x = -10000.0;

                ui.scroll_to_rect(rect, Some(Align::Center));

                self.emit_scroll_done();
            }
        });
    }

    fn ui_list_node_menu(&self, ui: &mut Ui, node: &Ref<Node>) {
        if ui.button("Pin").clicked() {
            self.emit_pin(node.id, &node.name);

            ui.close_menu();
        }

        ui.separator();

        if ui.button("View in tree").clicked() {
            self.emit_focus(node.id, &node.name);
            self.emit_view(NodeView::Tree);

            ui.close_menu();
        }
    }

    fn ui_list_options(&mut self, ui: &mut Ui) {
        let mut prepending = self.state.sort_mode.prepending();

        if ui.button("⬆")
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text("Bring modified to top")
            .clicked()
        {
            self.state.sort_mode.prepend(true);
            self.state.sort();
        }

        ComboBox::from_id_source("Sort")
            .selected_text("Sort by…")
            .width(ui.available_width() - (ui.spacing().item_spacing.x + 0.5))
            .show_ui(ui, |ui|
        {
            let mut sort_mode = self.state.sort_mode;
            let mut sort = false;

            sort |= ui.selectable_value(&mut sort_mode, NodeSortMode::Type(ModifiedFirst(prepending)), "Bone type")
                .clicked();
            sort |= ui.selectable_value(&mut sort_mode, NodeSortMode::Id(ModifiedFirst(prepending)), "ID")
                .clicked();
            sort |= ui.selectable_value(&mut sort_mode, NodeSortMode::Name(ModifiedFirst(prepending)), "Name")
                .clicked();

            if sort {
                self.state.sort_by(sort_mode);
            }
        });
    }

    fn ui_node_status(&mut self, ui: &mut Ui) {
        let mut on_very_bottom = true;

        match &self.state.spotlight {
            Some(node_summary)
                => {
                    if self.ui_node_status_line(ui, "Spotlight:", node_summary.to_owned(), true).changed() {
                        self.state.cancel_spotlight();
                    }

                    on_very_bottom = false;

                    ui.separator();
            }
            _ => {}
        }

        match (self.state.paste_mode,
               &self.state.paste)
        {
            (paste_mode,
             Some(node_summary))
                => {
                    let status_text = format!("{paste_mode:}:");

                    if self.ui_node_status_line(ui, status_text.as_str(), node_summary.to_owned(), on_very_bottom).changed() {
                        self.state.cancel_paste();
                    }

                    ui.separator();
                }
            _ => {}
        }
    }

    fn ui_node_status_line(&mut self, ui: &mut Ui, status_text: &str, node_summary: NodeSummary, on_very_bottom: bool) -> Response {
        if on_very_bottom {
            ui.add_space(4.0);
        }

        ui.horizontal(|ui| {
            let inner_response;
            let mut text = LayoutJob::single_section(
                node_summary.1.to_owned(),
                TextFormat {
                    color: ui.visuals().weak_text_color(),
                    ..Default::default()
                }
            );

            text.wrap.break_anywhere = true;
            text.wrap.max_rows = 1;

            ui.spacing_mut().item_spacing.x = 4.0;
            ui.style_mut().wrap = Some(true);

            inner_response = ui.checkbox(&mut true, status_text);

            ui.with_layout(Layout::right_to_left(), |ui| {
                if ui.add(
                    Label::new(text)
                        .sense(Sense::click())
                ).on_hover_cursor(CursorIcon::PointingHand)
                 .on_hover_text_at_pointer(&node_summary.1)
                 .clicked()
                {
                    self.scroll_to(node_summary.0);
                }
            });

            inner_response
        }).inner
    }

    fn ui_pin(&mut self, ui: &mut Ui, pin: &NodePin) {
        let scrollable = self.state.visitable(pin.id, &pin.path);
        let pin_button = ui.selectable_value(&mut self.state.selection, Some(pin.id), &pin.display_name)
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text_at_pointer(&pin.name)
            .context_menu(|ui|
        {
            if ui.button("Unpin").clicked() {
                self.emit_remove_pin(pin.id);

                ui.close_menu();
            }

            ui.separator();

            if ui.add_enabled(scrollable, Button::new("Scroll to here")).clicked() {
                self.focus(pin.id, &pin.name);

                ui.close_menu();
            }
        });

        if pin_button.middle_clicked() {
            self.emit_remove_pin(pin.id);
        } else if pin_button.double_clicked() && scrollable {
            self.scroll_to(pin.id);
        } else if pin_button.clicked() {
            self.emit_select_node(pin.id, &pin.name);
        }
    }

    fn ui_pins(&mut self, ui: &mut Ui) {
        ScrollArea::horizontal()
            .show(ui, |ui|
        {
            ui.set_width(ui.available_width());

            self.ui_pins_style(ui);

            ui.horizontal(|ui| {
                ui.label("Pins:");

                if self.state.has_pins() {
                    let pins = mem::take(&mut self.state.pins);

                    for pin in &pins {
                        self.ui_pin(ui, pin);
                    }

                    self.state.pins = pins;
                } else {
                    ui.label(RichText::new("None…").weak());
                }
            });
        });
    }

    fn ui_pins_style(&self, ui: &mut Ui) {
        ui.visuals_mut().selection.bg_fill = (*UiColor).tree.pinned.normal();
    }

    fn ui_root(&mut self, ui: &mut Ui) {
        if self.state.rooted() {
            let mut rooted_label = ui.add(
                Label::new(RichText::new("Rooted…").weak())
                    .sense(Sense::click())
            ).on_hover_cursor(CursorIcon::Help);

            if rooted_label.hover_pos() != self.transient_state.rooted_cursor {
                rooted_label = rooted_label.on_hover_ui(|ui| {
                    let mut hover_text = LayoutJob::default();

                    hover_text.append("Tree has been", 0.0, TextFormat::default());
                    hover_text.append(" rooted ", 0.0, TextFormat { color: (*UiColor).common.weak_gray.normal(), ..Default::default() });
                    hover_text.append("under a parent node.\nClick to restore previous", 0.0, TextFormat::default());
                    hover_text.append(" root.", 0.0, TextFormat { color: (*UiColor).common.weak_gray.normal(), ..Default::default() });

                    ui.label(hover_text);

                    self.transient_state.rooted_cursor = None;
                });
            }

            if rooted_label.clicked() {
                self.state.pop_root();

                // If we click and we're still rooted, remember where the cursor
                // is to hide the hover text (until the cursor moves)
                self.transient_state.rooted_cursor = if self.state.rooted() {
                    rooted_label.hover_pos()
                } else {
                    None
                }
            } else if !rooted_label.hovered() {
                self.transient_state.rooted_cursor = None;
            }
        }
    }

    fn ui_subtree(&mut self, ui: &mut Ui, subtree: &NodeTree, visibility: f32) {
        let (node, metadata) = subtree.data().as_tuple();
        let (expect_node, expect_subnodes) = self.state.inspect_subtree(subtree, &metadata);

        if expect_node {
            if metadata.hidden {
                for child in &subtree.children {
                    self.ui_subtree(ui, child, visibility);
                }
            } else {
                if expect_subnodes {
                    let mut collapser = CollapsingState::load_with_default_open(ui.ctx(), ui_id!(self, node), false);
                    let subvisibility = visibility.min(collapser.openness(ui.ctx()));

                    self.ui_expand(&mut collapser, &node);

                    if collapser.show_header(ui, |ui| {
                        self.ui_subtree_node(ui, &node, &metadata, visibility);
                    }).body(|ui| {
                        for child in &subtree.children {
                            self.ui_subtree(ui, child, subvisibility);
                        }
                    }).0.secondary_clicked() {
                        self.emit_expand_node(node.id);
                    }

                    self.ui_expand_undo(&node);
                } else {
                    ui.horizontal(|ui| {
                        ui.add_space(18.0);

                        self.ui_subtree_node(ui, &node, &metadata, visibility);
                    });
                }
            }
        }
    }

    fn ui_subtree_node(&self, ui: &mut Ui, node: &Ref<Node>, metadata: &Ref<NodeMetadata>, visibility: f32) {
        ui.scope(|ui| {
            let node_button;

            self.ui_highlight(ui, node, metadata, DimHighlight(true));

            node_button = ui.button(&node.name)
                .on_hover_cursor(CursorIcon::PointingHand)
                .context_menu(|ui|
            {
                self.ui_subtree_node_menu(ui, node, metadata);
            });

            if node_button.clicked() {
                self.emit_select_node(node.id, &node.name);
            }

            if self.transient_state.pending_scroll(node.id) && visibility == 1.0 {
                node_button.scroll_to_me(Some(Align::Center));

                self.emit_scroll_done();
            }
        });
    }

    fn ui_subtree_node_menu(&self, ui: &mut Ui, node: &Ref<Node>, metadata: &Ref<NodeMetadata>) {
        if ui.button("Pin").clicked() {
            self.emit_pin(node.id, &node.name);

            ui.close_menu();
        }

        if !self.state.rooted_at_top(node.id) {
            if !self.state.rooted_at(node.id) {
                if ui.button("Root").clicked() {
                    self.emit_root(node.id);

                    ui.close_menu();
                }
            } else {
                if ui.button("Unroot").clicked() {
                    self.emit_unroot();

                    ui.close_menu();
                }

                if self.state.root_nested() {
                    if ui.button("Unroot (to top)").clicked() {
                        self.emit_unroot_all();

                        ui.close_menu();
                    }
                }
            }
        }

        ui.separator();

        if self.state.filtered() {
            if ui.button("View (without filter)").clicked() {
                self.emit_clear_filter();
                self.emit_focus(node.id, &node.name);

                ui.close_menu();
            }
        }

        if ui.button("View in list").clicked() {
            self.emit_focus(node.id, &node.name);
            self.emit_view(NodeView::List);

            ui.close_menu();
        }

        if !self.state.filtered_to(node.id) {
            ui.separator();

            if ui.button("Filter path to here").clicked() {
                self.emit_filter_to(node.id);

                ui.close_menu();
            }
        }

        if !self.state.filtered() {
            let enable_paste = (self.state.cutting() && !metadata.cut)
                                    || self.state.copying();

            ui.separator();

            if ui.button("Cut").clicked() {
                self.emit_cut(node.id, &node.name);

                ui.close_menu();
            }

            if ui.button("Copy").clicked() {
                self.emit_copy(node.id, &node.name);

                ui.close_menu();
            }

            if ui.button("Copy (without children)").clicked() {
                self.emit_copy_single(node.id, &node.name);

                ui.close_menu();
            }

            if ui.add_enabled(enable_paste, Button::new("Paste")).clicked() {
                self.emit_paste(node.id);

                ui.close_menu();
            }

            if ui.add_enabled(enable_paste, Button::new("Paste (after)")).clicked() {
                self.emit_paste_after(node.id);

                ui.close_menu();
            }

            ui.separator();

            ui.add_enabled(false, Button::new("New"));

            if ui.button("Delete").clicked() {
                self.emit_delete(node.id);

                ui.close_menu();
            }
        }
    }

    fn ui_tree(&mut self, ui: &mut Ui) {
        if let Some(node_id) = self.transient_state.start_scroll() {
            self.transient_state.expand = NodeExpand::Set(self.state.path_to(node_id));
        }

        self.with_subtrees(|region, subtrees| {
            for subtree in subtrees {
                region.ui_subtree(ui, subtree, 1.0);
            }
        });

        self.transient_state.expand = NodeExpand::Nil;
    }

    fn ui_views(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.add(RadioButton::new(self.state.view == NodeView::Tree, "Tree view"))
                .clicked()
            {
                if self.state.view == NodeView::List {
                    self.set_view(NodeView::Tree);
                }
            }

            if ui.add(RadioButton::new(self.state.view == NodeView::List, "List view"))
                .clicked()
            {
                if self.state.view == NodeView::Tree {
                    self.set_view(NodeView::List);
                }
            }

            ui.scope(|ui| {
                ui.set_enabled(self.state.view == NodeView::List);

                self.ui_list_options(ui);
            });
        });
    }

    pub fn view(&self) -> NodeView {
        self.state.view
    }

    fn with_subtrees(&mut self, mut routine: impl FnMut(&mut Self, &Vec<NodeTree>)) {
        let mut subtrees = self.state.tree.take();

        routine(self, &mut subtrees);

        self.state.tree.give(subtrees);
    }
}

impl NmdAppRegion for NmdAppTreeRegion {
    fn message_sender(&self) -> Option<&MessageSender> {
        self.message_sender.as_ref()
    }

    fn receive_message(&mut self, message: &Message) {
        match message {
            Message::UiState(UiState::BoneData(bone_id, modified))
                => { self.state.mark_modified(*bone_id, *modified); }
            Message::UiState(UiState::BoneFlag(bone_id, flag))
                => { self.state.update_flag(*bone_id, *flag); }
            Message::UiState(UiState::BoneName(bone_id, name))
                => { self.state.update_name(*bone_id, name); }
            Message::UiState(UiState::TreeNodeScrollDone)
                => { self.transient_state.scroll_id = None; }
            _   => {}
        }
    }

    fn select(&mut self, ui_component: &UiComponent) {
        match ui_component {
            UiComponent::MenuCommit
                => { self.state.clear_modified_state(); }
            UiComponent::MenuHideListIds(hide)
                => { self.state.hiding_ids = *hide; }
            UiComponent::TreeFilterClear
                => { self.state.clear_filter(); }
            UiComponent::TreeNode(node_id, node_name)
                => { self.state.select(*node_id, node_name); }
            UiComponent::TreeNodeCopy(node_id, node_name)
                => { self.state.copy(Recursive(true), *node_id, node_name); }
            UiComponent::TreeNodeCopySingle(node_id, node_name)
                => { self.state.copy(Recursive(false), *node_id, node_name); }
            UiComponent::TreeNodeCut(node_id, node_name)
                => { self.state.cut(*node_id, node_name); }
            UiComponent::TreeNodeDelete(node_id)
                => { if let Some(ids) = self.state.delete(*node_id) { self.emit_deleted(*node_id, ids); } }
            UiComponent::TreeNodeExpansion(node_id)
                => { self.expand(*node_id); }
            UiComponent::TreeNodeFilterTo(node_id)
                => { self.state.filter_to(*node_id); }
            UiComponent::TreeNodeFocus(node_id, node_name)
                => { self.focus(*node_id, node_name); }
            UiComponent::TreeNodePaste(parent_id)
                => { if let Some((node_id, ui_state)) = self.state.paste(*parent_id, None)  { self.on_pasted(node_id, ui_state); } }
            UiComponent::TreeNodePasteAfter(sibling_id)
                => { if let Some((node_id, ui_state)) = self.state.paste_after(*sibling_id) { self.on_pasted(node_id, ui_state); } }
            UiComponent::TreeNodePin(pin_id, pin_name)
                => { self.state.insert_pin(*pin_id, pin_name); }
            UiComponent::TreeNodePinRemove(pin_id)
                => { self.state.remove_pin(*pin_id); }
            UiComponent::TreeNodeRoot(node_id)
                => { self.state.push_root(*node_id); }
            UiComponent::TreeNodeScroll(node_id)
                => { self.scroll_to(*node_id); }
            UiComponent::TreeNodeSpotlight(node_id, node_name)
                => { self.state.spotlight(*node_id, node_name); }
            UiComponent::TreeNodeUnroot
                => { self.state.pop_root(); }
            UiComponent::TreeNodeUnrootAll
                => { self.state.clear_roots(); }
            UiComponent::TreeNodeView(node_view)
                => { self.set_view(*node_view); }
            _   => {}
        }
    }

    fn ui(&mut self, ctx: &Context) {
        self.handle_keys(ctx);

        SidePanel::left("region$tree")
            .width_range(Self::MIN_WIDTH..=Self::MAX_WIDTH)
            .show(ctx, |ui|
        {
            ui.add_space(6.0);
            self.ui_pins(ui);
            ui.separator();
            self.ui_history(ui);
            ui.separator();
            self.ui_views(ui);
            ui.separator();
            self.ui_filter(ui);
            ui.separator();
            self.ui_body(ui);
        });

        if let Some(Edited(status)) = self.state.consume_edit_status() {
            self.emit_edit_status(status);
        }
    }

    fn uuid_source(&self) -> u64 {
        self.uuid_source
    }
}

impl NmdAppTreeProjectState {
    const HISTORY_SIZE: usize = 10;

    /// Supply a state set that should affect ancestors of a moved or deleted
    /// node.
    fn ancestor_state_set(subtree: &NodeTree, recursive: Recursive) -> NodeStateSet {
        let mut state_set = NodeStateSet::new(); 

        Self::ancestor_state_set_internal(subtree, recursive, &mut state_set);

        state_set
    }

    fn ancestor_state_set_internal(subtree: &NodeTree, recursive: Recursive, state_set: &mut NodeStateSet) {
        let modified_state_set = &subtree.data().metadata.borrow().modified;

        state_set.extend(
            modified_state_set
                .iter()
                .filter_map(|node_state| node_state.to_ancestor(subtree.id()))
        );

        if recursive.0 {
            for child in subtree.children() {
                Self::ancestor_state_set_internal(child, recursive, state_set);
            }
        }
    }

    fn cancel_paste(&mut self) {
        if let Some(NodeSummary(node_id, _)) = self.paste {
            self.mark_no_cut_copy(node_id);
            self.paste = None;
        }

        self.paste_mode = NodePasteMode::Nil;
    }

    fn cancel_spotlight(&mut self) {
        self.spotlight = None;
    }

    fn change_path_state(&mut self, path: &VecDeque<u16>, set_operation: NodeStateSetOperation, state_set: &NodeStateSet) {
        self.tree.for_path_mut(path, |mut subtree| {
            let metadata = &mut subtree.data_mut().metadata.borrow_mut();

            metadata.modified = match set_operation {
                NodeStateSetOperation::Difference => &metadata.modified - state_set,
                NodeStateSetOperation::Union => &metadata.modified | state_set,
            }
        });

        self.on_modified();
    }

    fn clear_filter(&mut self) {
        self.filter_text.clear();
        self.filter_visit_cache.clear();
    }

    fn clear_modified_state(&mut self) {
        for node_wrapper in &mut self.list {
            node_wrapper.metadata.borrow_mut().modified.clear();
        }

        self.on_modified();
    }

    fn clear_roots(&mut self) {
        if let Some(popped_root_id) = self.roots.pop() {
            self.roots.clear();

            Self::mark_hidden(&mut self.tree, Some(popped_root_id), false);
        }
    }

    /// Retain only objects referencing an ID in the state ID set.
    fn clean(&mut self) {
        self.clean_filter();
        self.clean_history();
        self.clean_list();
        self.clean_paste();
        self.clean_pins();
        self.clean_roots();
        self.clean_selection();
        self.clean_spotlight();
        self.on_clean(); // Side effect(s)
    }

    #[inline]
    fn clean_filter(&mut self) {
        self.filter.retain(|node| self.ids.contains(&node.borrow().id));
    }

    #[inline]
    fn clean_history(&mut self) {
        self.history.retain(|NodeSummary(id, _)| self.ids.contains(id));
    }

    #[inline]
    fn clean_list(&mut self) {
        self.list.retain(|node_wrapper| self.ids.contains(&node_wrapper.node.borrow().id));
    }

    #[inline]
    fn clean_paste(&mut self) {
        if self.dangling_paste() {
            self.cancel_paste();
        }
    }

    #[inline]
    fn clean_pins(&mut self) {
        self.pins.retain(|node_pin| self.ids.contains(&node_pin.id));
    }

    #[inline]
    fn clean_roots(&mut self) {
        // This is inefficient if done repeatedly but not expected to loop more
        // than once
        while let Some(root_id) = self.roots.last() {
            if !self.ids.contains(&root_id) {
                self.pop_root()
            } else {
                break;
            }
        }
    }

    #[inline]
    fn clean_selection(&mut self) {
        if self.dangling_selection() {
            self.selection = None; 
        }
    }

    #[inline]
    fn clean_spotlight(&mut self) {
        if self.dangling_spotlight() {
            self.cancel_spotlight();
        }
    }

    fn consume_edit_status(&mut self) -> Option<Edited> {
        let status = self.observe_edit_status();

        if status.is_some() {
            self.unfreeze_edit_status();
        }

        status
    }

    fn copy(&mut self, recursive: Recursive, node_id: u16, node_name: &String) {
        self.cancel_paste();
        self.paste = Some(NodeSummary(node_id, node_name.to_owned()));
        self.paste_mode = NodePasteMode::Copy(recursive, 0);

        if self.mark_copied(node_id, recursive, true) == Recursed(false) && recursive.0 {
            // Don't allow `paste_mode` to specify `recursive` if there's
            // nothing to recurse into (just a cosmetic thing)
            self.paste_mode.recurse(false);
        }

        // TODO FEAT:SPOTLIGHT
        self.cancel_spotlight();
    }

    fn copying(&self) -> bool {
        matches!(self.paste_mode, NodePasteMode::Copy(_, _))
    }

    fn cut(&mut self, node_id: u16, node_name: &String) {
        self.cancel_paste();
        self.paste = Some(NodeSummary(node_id, node_name.to_owned()));
        self.paste_mode = NodePasteMode::Cut;
        self.mark_cut(node_id, true);
        // TODO FEAT:SPOTLIGHT
        self.cancel_spotlight();
    }

    fn cutting(&self) -> bool {
        self.paste_mode == NodePasteMode::Cut
    }

    fn dangling_paste(&self) -> bool {
        matches!(self.paste, Some(NodeSummary(id, _)) if !self.ids.contains(&id))
    }

    fn dangling_selection(&self) -> bool {
        matches!(self.selection, Some(id) if !self.ids.contains(&id))
    }

    fn dangling_spotlight(&self) -> bool {
        matches!(self.spotlight, Some(NodeSummary(id, _)) if !self.ids.contains(&id))
    }

    fn delete(&mut self, node_id: u16) -> Option<HashSet<u16>> {
        if let (path_to_parent, Some(mut parent)) = self.tree.path_to_parent_mut(node_id) {
            if let Some(target) = parent.take_child(node_id) {
                let ancestor_state = Self::ancestor_state_set(&target, Recursive(true));

                self.change_path_state(&path_to_parent, NodeStateSetOperation::Difference, &ancestor_state);
                self.disintegrate(&target);

                return Some(target.iter().map(|subtree| subtree.id()).collect());
            }
        }

        None
    }

    fn disintegrate(&mut self, removed_subtree: &NodeTree) { 
        for subtree in removed_subtree {
            self.ids.remove(&subtree.id());
        }

        self.clean();
    }

    /// Return whether this metadata indicates a node that should appear,
    /// with no consideration to descendent state.
    fn expect_in_view(&self, metadata: &Ref<NodeMetadata>) -> bool {
        !metadata.hidden
            && (!self.filtered()
                    || metadata.filtered.contains(&NodeState::Filtered))
    }

    fn filter(&mut self) {
        if let Ok(expression) = Expression::try_from(&self.filter_text.to_lowercase()) {
            self.mark_filtered(expression);
        }
    }

    fn filter_to(&mut self, node_id: u16) {
        self.filter_with(&format!("#{:x}", node_id));
    }

    fn filter_with(&mut self, filter_text: &String) {
        self.filter_text = filter_text.to_owned();
        self.filter();
    }

    #[inline]
    fn filtered(&self) -> bool {
        !self.filter_text.is_empty()
    }

    #[inline]
    fn filtered_to(&self, node_id: u16) -> bool {
        self.filter_text == format!("#{:x}", node_id)
    }

    /// Fill in reference-counted fields and sort list contents. This has to be
    /// called after serializing to complete the object.
    fn finalize(&mut self) {
        self.finalize_filter();
        self.finalize_list();
    }

    #[inline]
    fn finalize_filter(&mut self) {
        self.filter = NodeFilter::from(&self.tree);
    }

    #[inline]
    fn finalize_list(&mut self) {
        self.list = self.tree.iter().map(|subtree| subtree.data().to_owned()).collect();
        self.sort();
    }
    
    fn get_summary(&self, node_id: u16) -> Option<NodeSummary> {
        Some(
            self.iter_nodes()
            .find(|node| node.id == node_id)?
            .to_summary()
        )
    }

    fn has_history(&self) -> bool {
        !self.history.is_empty()
    }

    fn has_pin(&self, pin_id: u16) -> bool {
        self.pins.iter().any(|pin| pin.id == pin_id)
    }

    fn has_pins(&self) -> bool {
        !self.pins.is_empty()
    }

    fn insert_pin(&mut self, pin_id: u16, pin_name: &String) {
        if !self.has_pin(pin_id) {
            let pin_path = self.tree.path_to(pin_id).0.into();

            self.pins.push(NodePin::new(pin_id, pin_name, pin_path));
        }
    }

    /// Return a tuple representing `(expect_node, expect_subnodes)` for a
    /// subtree. Does not consider whether the subtree is under a temporary
    /// root.
    fn inspect_subtree(&self, subtree: &NodeTree, metadata: &Ref<NodeMetadata>) -> (bool, bool) {
        if self.filtered() {
            if metadata.filtered.is_empty() {
                (false, false)
            } else {
                (true, metadata.filtered.contains(&NodeState::FilteredAncestor))
            }
        } else {
            (true, !subtree.children.is_empty())
        }
    }

    fn integrate(new_subtree: &NodeTree, filter: &mut NodeFilter, id_set: &mut BTreeSet<u16>, list: &mut Vec<NodeWrapper>) {
        for subtree in new_subtree {
            filter.insert(subtree.data().node.to_owned());
            id_set.insert(subtree.id());
            list.push(subtree.data().to_owned());
        }
    }

    fn iter_nodes<'a>(&'a self) -> impl Iterator<Item = Ref<Node>> + 'a {
        // The filter iterates over all nodes directly so it's suitable
        let mut iter = self.filter.iter();

        iter_from(move || {
            if let Some(node) = iter.next() {
                Some(node.borrow())
            } else {
                None
            }
        })
    }

    fn iter_nodes_mut<'a>(&'a mut self) -> impl Iterator<Item = RefMut<Node>> + 'a {
        // The filter iterates over all nodes directly so it's suitable
        let mut iter = self.filter.iter();

        iter_from(move || {
            if let Some(node) = iter.next() {
                Some(node.borrow_mut())
            } else {
                None
            }
        })
    }

    fn path_to(&mut self, node_id: u16) -> HashSet<u16> {
        self.tree.path_to_except(node_id).0.into_iter().collect()
    }

    fn pop_root(&mut self) {
        if let Some(popped_root_id) = self.roots.pop() {
            if let Some(root) = self.root_mut(0) {
                Self::mark_hidden(root, Some(popped_root_id), false);
            }
        }
    }

    fn push_root(&mut self, root_id: u16) {
        self.roots.push(root_id);

        if let Some(old_root) = self.root_mut(1) {
            Self::mark_hidden(old_root, Some(root_id), true);
        }
    }

    fn mark_copied(&mut self, node_id: u16, recursive: Recursive, copied: bool) -> Recursed {
        if let Some(subtree) = self.tree.find_mut(node_id) {
            Self::mark_copied_internal(subtree, recursive, copied);

            Recursed(recursive.0 && subtree.children.len() > 0)
        } else {
            Recursed(false)
        }
    }

    fn mark_copied_internal(subtree: &mut NodeTree, recursive: Recursive, copied: bool) {
        subtree.data_mut().metadata.borrow_mut().copied = copied;

        if recursive.0 {
            for child in subtree.children_mut() {
                Self::mark_copied_internal(child, recursive, copied);
            }
        }
    }

    fn mark_cut(&mut self, node_id: u16, cut: bool) {
        if let Some(subtree) = self.tree.find_mut(node_id) {
            Self::mark_cut_internal(subtree, cut);
        }
    }

    fn mark_cut_internal(subtree: &mut NodeTree, cut: bool) {
        subtree.data_mut().metadata.borrow_mut().cut = cut;

        for child in subtree.children_mut() {
            Self::mark_cut_internal(child, cut);
        }
    }

    fn mark_cut_pasted(subtree: &mut NodeTree, cut_pasted: bool) {
        if cut_pasted {
            subtree.data_mut().metadata.borrow_mut().modified.insert(NodeState::CutPasted);
        } else {
            subtree.data_mut().metadata.borrow_mut().modified.remove(&NodeState::CutPasted);
        }

        for child in subtree.children_mut() {
            Self::mark_cut_pasted(child, cut_pasted);
        }
    }

    fn mark_filtered(&mut self, expression: Expression) {
        if let Some(mut filtered_set) = self.filter.query(&expression) {
            Self::mark_filtered_internal(&mut self.tree, &mut filtered_set);
        }
    }

    fn mark_filtered_internal(subtree: &mut impl Tree<Data = NodeWrapper>, filtered_set: &mut NodeFilterSet) -> bool {
        let mut in_path = false;

        for child in subtree.children_mut() {
            child.data_mut().metadata.borrow_mut().filtered.clear();

            if filtered_set.remove(&child.data().node) {
                child.data_mut().metadata.borrow_mut().filtered.insert(NodeState::Filtered);

                in_path = true;
            }

            if Self::mark_filtered_internal(child, filtered_set) {
                child.data_mut().metadata.borrow_mut().filtered.insert(NodeState::FilteredAncestor);

                in_path = true;
            }
        }

        in_path
    }

    fn mark_hidden(subtree: &mut dyn Tree<Data = NodeWrapper>, stop_id: Option<u16>, hidden: bool) {
        if Some(subtree.id()) != stop_id {
            if let Some(node_wrapper) = subtree.data_mut_opt() {
                node_wrapper.metadata.borrow_mut().hidden = hidden;
            }

            for child in subtree.children_mut() {
                Self::mark_hidden(child, stop_id, hidden);
            }
        }
    }

    fn mark_modified(&mut self, target_id: u16, modified: bool) {
        Self::mark_modified_internal(&mut self.tree, target_id, modified);

        self.on_modified();
    }

    fn mark_modified_internal(subtree: &mut impl Tree<Data = NodeWrapper>, target_id: u16, modified: bool) -> bool {
        let state_change;

        for child in subtree.children_mut() {
            if child.id() == target_id {
                state_change = NodeState::Modified;
            } else if Self::mark_modified_internal(child, target_id, modified) {
                state_change = NodeState::ModifiedAncestor(target_id);
            } else {
                continue;
            }

            if modified {
                child.data_mut().metadata.borrow_mut().modified.insert(state_change);
            } else {
                child.data_mut().metadata.borrow_mut().modified.remove(&state_change);
            }

            return true;
        }

        false
    }

    fn mark_no_cut_copy(&mut self, node_id: u16) {
        if let Some(subtree) = self.tree.find_mut(node_id) {
            Self::mark_no_cut_copy_internal(subtree);
        }
    }

    fn mark_no_cut_copy_internal(subtree: &mut NodeTree) {
        {
            let mut metadata = subtree.data_mut().metadata.borrow_mut();
            
            metadata.copied = false;
            metadata.cut = false;
        }

        for child in subtree.children_mut() {
            Self::mark_no_cut_copy_internal(child);
        }
    }

    fn modified(&self) -> bool {
        self.tree.children()
            .iter()
            .any(|child| !child.data().metadata.borrow().modified.is_empty())
    }

    fn observe_edit_status(&self) -> Option<Edited> {
        if self.edit_status[0] != self.edit_status[1] {
            Some(Edited(self.edit_status[1]))
        } else {
            None
        }
    }

    fn on_clean(&mut self) {
        self.on_modified();
    }

    fn on_modified(&mut self) {
        self.update_edit_status();
    }

    fn on_paste_copy(&mut self) {
        self.on_modified();
    }

    fn on_paste_cut(&mut self) {
        self.on_modified();
    }

    fn paste(&mut self, target_parent_id: u16, predecessor_id_opt: Option<u16>) -> Option<(u16, UiState)> {
        match (self.paste_mode,
               self.paste.clone())
        {
            (NodePasteMode::Copy(recursive, consecutive_count),
             Some(NodeSummary(node_id, _)))
                => self.paste_copy(node_id, target_parent_id, predecessor_id_opt, recursive, consecutive_count),
            (NodePasteMode::Cut,
             Some(NodeSummary(node_id, _)))
                => self.paste_cut(node_id, target_parent_id, predecessor_id_opt),
            _   => None,
        }
    }

    fn paste_after(&mut self, predecessor_id: u16) -> Option<(u16, UiState)> {
        if let Some(node_id) = self.paste_id() {
            // Some redundancy in that we traverse to `target_parent` here & later
            if let Some(target_parent) = self.tree.find_parent(predecessor_id) {
                let target_parent_id = target_parent.id();

                match (target_parent.child_index(predecessor_id),
                       target_parent.child_index(node_id))
                {
                    (Some(i), Some(j))
                        if self.paste_mode == NodePasteMode::Cut
                            && i + 1 == j
                        => { self.cancel_paste(); }
                    _   => { return self.paste(target_parent_id, Some(predecessor_id)); }
                }
            } else {
                self.cancel_paste();
            }
        }

        None
    }

    fn paste_copy(&mut self, target_id: u16, new_parent_id: u16, predecessor_id_opt: Option<u16>, recursive: Recursive, consecutive_count: usize) -> Option<(u16, UiState)> {
        if let Some(target) = self.tree.find_mut(target_id) {
            let mut id_copies = HashMap::<u16, (u16, String)>::new();

            // Could supply `recursive = true` to the transform function no
            // matter what, since closure rejects on `copied = false`
            if let Some(target_copy) = target.transformed(recursive.0, &self.ids, |(old_id, new_id), node_wrapper| {
                (node_wrapper.metadata.borrow().copied)
                    .then(||
                {
                    let node = node_wrapper.node.borrow().to_copy(new_id, consecutive_count);
                    let metadata = {
                        let mut metadata = NodeMetadata::default();

                        // We forego a `mark_copy_pasted` later by setting this here
                        metadata.modified.insert(NodeState::CopyPasted);
                        metadata
                    };

                    id_copies.insert(old_id, (new_id, node.name.to_owned()));

                    NodeWrapper::new(node, metadata)
                })
            }) {
                match self.paste_copy_internal(target_copy, new_parent_id, predecessor_id_opt, recursive) {
                    Ok(path_to_copy) => {
                        if let Some(copy) = self.tree.at_path(&path_to_copy) {
                            let copy_id = copy.id();

                            Self::integrate(copy, &mut self.filter, &mut self.ids, &mut self.list);

                            self.on_paste_copy();
                            self.paste_mode.increment();
                            self.sort();

                            return Some((copy_id, UiState::TreeNodeCopyPaste(id_copies, new_parent_id)));
                        }
                    }
                    Err(unclaimed_copy) => { /* Unexpected */ }
                }
            }
        }

        self.cancel_paste();

        None
    }

    fn paste_copy_internal(&mut self, mut target: NodeTree, new_parent_id: u16, predecessor_id_opt: Option<u16>, recursive: Recursive) -> Result<VecDeque<u16>, NodeTree> {
        if let (path_to_new_parent, Some(mut copy_parent)) = self.tree.path_to_mut(new_parent_id) {
            let ancestor_state = Self::ancestor_state_set(&target, recursive);
            let target_id = target.id();

            if let Some(predecessor_id) = predecessor_id_opt {
                copy_parent.give_child_at_index(target, copy_parent.child_index(predecessor_id).and_then(|i| Some(i + 1)).unwrap_or(0));
            } else {
                copy_parent.give_child_at_index(target, 0);
            }

            self.change_path_state(&path_to_new_parent, NodeStateSetOperation::Union, &ancestor_state);

            {
                let mut path_to_copy = path_to_new_parent;
                path_to_copy.push_back(target_id);
                
                Ok(path_to_copy)
            }
        } else {
            Err(target)
        }
    }

    fn paste_cut(&mut self, target_id: u16, new_parent_id: u16, predecessor_id_opt: Option<u16>) -> Option<(u16, UiState)> {
        if let (path_to_parent, Some(mut parent)) = self.tree.path_to_parent_mut(target_id) {
            if parent.id() == new_parent_id
                && predecessor_id_opt.is_none()
                && parent.child_index(target_id) == Some(0) {
                // Pass
            } else if let Some(mut child) = parent.take_child(target_id) {
                match self.paste_cut_internal(&path_to_parent, child, new_parent_id, predecessor_id_opt) {
                    Ok(_) => {
                        self.cancel_paste();
                        self.on_paste_cut();

                        return Some((target_id, UiState::TreeNodeCutPaste(target_id, new_parent_id)));
                    }
                    Err(unclaimed_child) => {
                        // Paste failed, so restore the child (unexpected)
                        self.tree.insert_at_path(&path_to_parent, unclaimed_child);
                    }
                }
            }
        }
        
        self.cancel_paste();

        None
    }

    fn paste_cut_internal(&mut self, path_to_parent: &VecDeque<u16>, mut target: NodeTree, new_parent_id: u16, predecessor_id_opt: Option<u16>) -> Result<VecDeque<u16>, NodeTree> {
        if let (path_to_new_parent, Some(mut new_parent)) = self.tree.path_to_mut(new_parent_id) {
            let ancestor_state;
            let target_id = target.id();

            if target.parent_id() == new_parent_id {
                ancestor_state = NodeStateSet::new();
            } else {
                // Mark before generating the ancestor state, which will include
                // `NodeState::CutPastedAncestor(...)`
                Self::mark_cut_pasted(&mut target, true);

                ancestor_state = Self::ancestor_state_set(&target, Recursive(true));
            }

            if let Some(predecessor_id) = predecessor_id_opt {
                new_parent.give_child_at_index(target, new_parent.child_index(predecessor_id).and_then(|i| Some(i + 1)).unwrap_or(0));
            } else {
                new_parent.give_child_at_index(target, 0);
            }

            self.change_path_state(path_to_parent, NodeStateSetOperation::Difference, &ancestor_state);
            self.change_path_state(&path_to_new_parent, NodeStateSetOperation::Union, &ancestor_state);

            {
                let mut path_to_target = path_to_new_parent;
                path_to_target.push_back(target_id);

                Ok(path_to_target)
            }
        } else {
            Err(target)
        }
    }

    fn paste_id(&self) -> Option<u16> {
        self.paste.as_ref().and_then(|NodeSummary(node_id, _)| Some(*node_id))
    }

    fn prepend_history(&mut self, node_id: u16, node_name: &String) {
        if let Some(index) = self.history.iter().position(|NodeSummary(id, _)| *id == node_id) {
            self.history.remove(index);
        }

        self.history.push_front(NodeSummary(node_id, node_name.to_owned()));

        if self.history.len() > Self::HISTORY_SIZE {
            self.history.pop_back();
        }
    }

    fn prepend_modified(&mut self) {
        let (mut modified, not): (Vec<_>, Vec<_>) = mem::take(&mut self.list)
            .into_iter()
            .partition(|a| a.metadata.borrow().modified.contains(&NodeState::Modified));

        modified.extend(not);

        self.list = modified;
    }

    fn remove_pin(&mut self, pin_id: u16) {
        self.pins.retain(|pin| pin.id != pin_id);
    }

    fn root_mut(&mut self, distance_up: usize) -> Option<&mut dyn Tree<Data = NodeWrapper>> {
        if let Some(root_id) = self.roots.get(self.roots.len().wrapping_sub(distance_up + 1)) {
            // It can't infer the type here...
            self.tree.find_mut(*root_id)
                .map(|subtree| subtree as &mut dyn Tree<Data = NodeWrapper>)
        } else {
            Some(&mut self.tree)
        }
    }

    fn root_nested(&self) -> bool {
        self.roots.len() > 1
    }

    fn rooted(&self) -> bool {
        self.roots.len() > 0
    }

    fn rooted_at(&self, node_id: u16) -> bool {
        self.roots.last() == Some(&node_id)
    }

    fn rooted_at_top(&self, node_id: u16) -> bool {
        self.natural_root == Some(node_id)
    }

    fn select(&mut self, node_id: u16, node_name: &String) {
        self.selection = Some(node_id);
        self.prepend_history(node_id, node_name);
    }

    fn selected(&self, node_id: u16) -> bool {
        matches!(self.selection, Some(id) if id == node_id)
    }

    fn spotlight(&mut self, node_id: u16, node_name: &String) {
        self.spotlight = Some(NodeSummary(node_id, node_name.to_owned()));
        // TODO FEAT:SPOTLIGHT
        self.cancel_paste();
    }

    fn spotlighted(&self, node_id: u16) -> bool {
        matches!(self.spotlight, Some(NodeSummary(id, _)) if id == node_id)
    }

    fn sort(&mut self) {
        let then_prepend = self.sort_mode.prepending();

        match self.sort_mode {
            NodeSortMode::Id(_) => self.sort_by_id(),
            NodeSortMode::Name(_) => self.sort_by_name(),
            NodeSortMode::Type(_) => self.sort_by_type(),
            _ => {}
        }

        if then_prepend {
            self.prepend_modified();
            // For now, toggle off prepend after doing it once
            self.sort_mode.prepend(false);
        }
    }

    fn sort_by(&mut self, sort_mode: NodeSortMode) {
        self.sort_mode = sort_mode;
        self.sort();
    }

    fn sort_by_id(&mut self) {
        self.list.sort_by(|a, b| a.node.borrow().id.cmp(&b.node.borrow().id));
    }

    fn sort_by_name(&mut self) {
        self.list.sort_by(|a, b| a.node.borrow().name.cmp(&b.node.borrow().name));
    }

    fn sort_by_type(&mut self) {
        // Sort by type -> group by type -> sort groups by name -> flatten
        self.list = iter::sort_then_group_by(mem::take(&mut self.list), |a, b| a.node.borrow().normalized_flag.cmp(&b.node.borrow().normalized_flag))
            .into_iter()
            .map(|mut group| {
                group.sort_by(|a, b| a.node.borrow().name.cmp(&b.node.borrow().name));
                group
            })
            .flatten()
            .collect();
    }

    fn status(&self) -> Edited {
        Edited(self.edit_status[1])
    }

    fn unfreeze_edit_status(&mut self) {
        self.edit_status = [self.edit_status[1], self.edit_status[1]];
        self.edit_status_frozen = false;
    }

    fn update_edit_status(&mut self) {
        if !self.edit_status_frozen {
            self.edit_status = [self.edit_status[1], self.modified()];

            if self.observe_edit_status().is_some() {
                // Freeze status so that it isn't overwritten in a successive
                // operation, for example. Can be observed & unfrozen at once
                // with `consume_edit_status()`
                self.edit_status_frozen = true;
            }
        }
    }

    fn update_flag(&mut self, node_id: u16, flag: NmdFileBoneFlag) {
        self.update_flag_for_node(node_id, flag);
    }

    #[inline]
    fn update_flag_for_node(&mut self, node_id: u16, flag: NmdFileBoneFlag) {
        for mut node in self.iter_nodes_mut() {
            if node.id == node_id {
                node.set_flag(flag);
                break;
            }
        }
    }

    fn update_name(&mut self, node_id: u16, name: &String) {
        let name_space;
        let name_str = if name.chars().all(|c| c.is_whitespace()) {
            name_space = format!("({})", " ".repeat(name.len()));
            name_space.as_str()
        } else {
            name.as_str() 
        };

        self.update_name_for_node(node_id, name_str);
        self.update_name_for_pin(node_id, name_str);

        Self::update_name_for_summary(self.history.iter_mut(), node_id, name_str);
        Self::update_name_for_summary(self.paste.iter_mut(), node_id, name_str);
        Self::update_name_for_summary(self.spotlight.iter_mut(), node_id, name_str);
    }

    #[inline]
    fn update_name_for_node(&mut self, node_id: u16, name: &str) {
        for mut node in self.iter_nodes_mut() {
            if node.id == node_id {
                node.set_name(name);
                break;
            }
        }
    }

    #[inline]
    fn update_name_for_pin(&mut self, node_id: u16, name: &str) {
        for node_pin in &mut self.pins {
            if node_pin.id == node_id {
                node_pin.name = name.to_owned();
            }
        }
    }

    #[inline]
    fn update_name_for_summary<'a>(mut iter: impl Iterator<Item = &'a mut NodeSummary>, node_id: u16, name: &str) {
        for x in iter {
            if x.0 == node_id {
                x.1 = name.to_owned();
                break;
            }
        }
    }

    /// Return whether a node was visited after attempting to walk to it in the
    /// curent filter.
    fn visit(tree: &NodeTree, node_id: u16) -> bool {
        if tree.data().metadata.borrow().filtered.is_empty() {
            false
        } else {
            if tree.id() == node_id {
                true
            } else {
                for tree in tree.children() {
                    if Self::visit(tree, node_id) {
                        return true;
                    }
                }

                false
            }
        }
    }

    fn visitable(&mut self, node_id: u16, node_path: &Vec<u16>) -> bool {
        if let Some(root_id) = self.roots.last() {
            if !node_path.contains(&root_id) {
                return false;
            }
        }

        if self.filtered() {
            if let Some(value) = self.filter_visit_cache.get(&node_id) {
                return *value;
            } else {
                self.filter_visit_cache.insert(node_id, false);

                for child in &self.tree.children {
                    if Self::visit(child, node_id) {
                        self.filter_visit_cache.insert(node_id, true);

                        return true;
                    }
                }
            }

            false
        } else {
            true
        }
    }
}

impl From<&NmdFileData> for NmdAppTreeProjectState {
    fn from(data: &NmdFileData) -> Self {
        let tree = data.tree_with(|bone_data| NodeWrapper::from(bone_data));
        let mut state = Self {
            natural_root: (tree.children.len() == 1).then(|| tree.children.first().unwrap().id()),
            sort_mode: NodeSortMode::Type(ModifiedFirst(false)),
            ids: tree.iter().map(|subtree| subtree.id()).collect(),
            tree: tree,
            ..Default::default()
        };

        state.finalize();
        state
    }
}

impl NmdAppTreeTransientState {
    fn pending_scroll(&self, node_id: u16) -> bool {
        self.scroll_id == Some(node_id)
    }

    fn start_scroll(&mut self) -> Option<u16> {
        if self.scroll_initialized {
            self.scroll_initialized = false;
            self.scroll_id
        } else {
            None
        }
    }
}

impl Default for NodeExpand {
    fn default() -> Self {
        NodeExpand::Nil
    }
}

impl NodePasteMode {
    fn increment(&mut self) {
        *self = match self {
            NodePasteMode::Copy(recursive, consecutive_count)
                => NodePasteMode::Copy(*recursive, *consecutive_count + 1),
            _   => *self
        }
    }

    fn recurse(&mut self, recurse: bool) {
        *self = match self {
            NodePasteMode::Copy(_, consecutive_count)
                => NodePasteMode::Copy(Recursive(recurse), *consecutive_count),
            _   => *self
        }
    }

    fn recursive(&self) -> bool {
        matches!(self, NodePasteMode::Cut | NodePasteMode::Copy(Recursive(true), _))
    }
}

impl Default for NodePasteMode {
    fn default() -> Self {
        NodePasteMode::Nil
    }
}

impl Display for NodePasteMode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match self {
            NodePasteMode::Copy(Recursive(true), _) => "Copy (Recursive)",
            NodePasteMode::Copy(_, _) => "Copy",
            NodePasteMode::Cut => "Cut",
            NodePasteMode::Nil => "Nil",
        })
    }
}

impl NodeSortMode {
    fn prepend(&mut self, prepend: bool) {
        *self = match self {
            Self::Id(_) => Self::Id(ModifiedFirst(prepend)),
            Self::Name(_) => Self::Name(ModifiedFirst(prepend)),
            Self::Type(_) => Self::Type(ModifiedFirst(prepend)),
            Self::Nil => *self,
        }
    }

    fn prepending(&self) -> bool {
        match self {
            Self::Id(ModifiedFirst(prepending)) => *prepending,
            Self::Name(ModifiedFirst(prepending)) => *prepending,
            Self::Type(ModifiedFirst(prepending)) => *prepending,
            Self::Nil => false,
        }
    }
}

impl Default for NodeSortMode {
    fn default() -> Self {
        NodeSortMode::Nil
    }
}

impl NodeState {
    fn to_ancestor(&self, node_id: u16) -> Option<Self> {
        match self {
            Self::CopyPasted => Some(Self::CopyPastedAncestor(node_id)),
            Self::CutPasted => Some(Self::CutPastedAncestor(node_id)),
            Self::Filtered => Some(Self::FilteredAncestor),
            Self::Modified => Some(Self::ModifiedAncestor(node_id)),
            _ => None
        }
    }
}

impl Default for NodeState {
    fn default() -> Self {
        NodeState::Nil
    }
}

impl Default for NodeView {
    fn default() -> Self {
        NodeView::Tree
    }
}

impl Node {
    const COPY_SUFFIX: &'static str = "_copy";

    fn new(id: u16, name: &String, flag: NmdFileBoneFlag) -> Self {
        Self {
            id: id,
            flag: flag,
            name: name.to_owned(),
            normalized_id: Self::normalize_id(id),
            normalized_flag: Self::normalize_flag(flag),
            normalized_name: Self::normalize_name(name.as_str()),
        }
    }

    fn increment_name(name: &String, count: usize) -> String {
        match count {
            0 => name.to_owned() + Self::COPY_SUFFIX,
            n => name.to_owned() + Self::COPY_SUFFIX + &(n + 1).to_string()
        }
    }

    fn normalize_flag(flag: NmdFileBoneFlag) -> String {
        flag.to_string().to_lowercase()
    }

    fn normalize_id(id: u16) -> String {
        format!("{:x}", id)
    }

    fn normalize_name(name: &str) -> String {
        name.to_lowercase()
    }

    fn set_flag(&mut self, flag: NmdFileBoneFlag) {
        self.flag = flag;
        self.normalized_flag = Self::normalize_flag(flag);
    }

    fn set_name(&mut self, name: &str) {
        self.name = name.to_owned();
        self.normalized_name = Self::normalize_name(name);
    }

    fn to_copy(&self, id: u16, copy_count: usize) -> Self {
        Self::new(id, &Self::increment_name(&self.name, copy_count), self.flag)
    }

    fn to_summary(&self) -> NodeSummary {
        NodeSummary(self.id, self.name.to_owned())
    }
}

impl From<&NmdFileBone> for Node {
    fn from(bone_data: &NmdFileBone) -> Self {
        Self::new(bone_data.id, &bone_data.name, bone_data.flag)
    }
}

impl Eq for Node {}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl NodeFilter {
    fn insert(&mut self, node: Rc<RefCell<Node>>) {
        self.collection.insert(node);
    }

    fn iter<'a>(&'a self) -> impl Iterator<Item = &Rc<RefCell<Node>>> {
        self.collection.iter()
    }

    fn remove(&mut self, node: &Rc<RefCell<Node>>) {
        self.collection.remove(node);
    }

    fn retain(&mut self, predicate: impl Fn(&Rc<RefCell<Node>>) -> bool) {
        self.collection.retain(predicate);
    }
}

impl Filter for NodeFilter {
    type FilterSet = NodeFilterSet;

    fn collection(&self) -> &Self::FilterSet {
        &self.collection
    }

    fn search(&self, token: &String, nodes: &Self::FilterSet) -> Self::FilterSet {
        let mut result = Self::FilterSet::default();

        for node in nodes.iter() {
            if match token.chars().nth(0) {
                Some('#') => node.borrow().normalized_id == &token[1..token.len()],
                Some('$') => node.borrow().normalized_flag.contains(&token[1..token.len()]),
                _         => node.borrow().normalized_name.contains(token)
            } {
                result.insert(node.to_owned());
            }
        }

        result
    }
}

impl From<&NodeTreeRoot> for NodeFilter {
    fn from(tree_root: &NodeTreeRoot) -> Self {
        Self {
            collection: tree_root
                .into_iter()
                .map(|subtree| subtree.data().node.to_owned())
                .collect()
        }
    }
}

impl NodePin {
    fn new(id: u16, name: &String, path: Vec<u16>) -> Self {
        Self {
            id: id,
            display_name: format!("{:#05X}", id),
            name: name.to_owned(),
            path: path,
        }
    }
}

impl NodeWrapper {
    fn new(node: Node, metadata: NodeMetadata) -> Self {
        Self {
            node: Rc::new(RefCell::new(node)),
            metadata: Rc::new(RefCell::new(metadata)),
        }
    }

    fn as_tuple(&self) -> (Ref<Node>, Ref<NodeMetadata>) {
        (self.node.borrow(), self.metadata.borrow())
    }
}

impl From<&NmdFileBone> for NodeWrapper {
    fn from(bone_data: &NmdFileBone) -> Self {
        Self::new(Node::from(bone_data), NodeMetadata::default())
    }
}

