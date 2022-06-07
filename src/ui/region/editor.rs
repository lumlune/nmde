use {
    crate::{
        io::{
            fifo::{
                Message,
                MessageSender,
            },
            nmd::anatomy::NmdFileBone,
            nmd::anatomy::NmdFileBoneFlag,
            nmd::data::tree::NmdFileBoneTreeNode,
            nmd::data::NmdFileData,
            nmd::NmdFileWriter,
        },
        ui::{
            region,
            region::NmdAppRegion,
            widget::*,
            UiComponent,
            UiColor,
            UiState,
        },
    },
    std::{
        cell::Ref,
        collections::{BTreeMap, HashMap, HashSet},
        io,
        mem,
        path::PathBuf,
        rc::Rc,
    },
    eframe::egui::*,
    eframe::egui::text::LayoutJob,
    serde::{
        Deserialize,
        Serialize,
    },
};

/*
 * TODO
 * ~ There's no special protocol right now for forked bone chains, i.e. knowing
 * which is "next" when there are multiple nexts. Need to see examples to know
 * how to handle (TODO FEAT:FORKS)
 *
 * ~ Expand editor minimum size just a bit - encountered longer string -
 *      -0.0000000000000000000100796926
 */

#[derive(Serialize, Deserialize)]
struct ChainSnippet(Option<NmdFileBone>, NmdFileBone, Option<NmdFileBone>);

macro_rules! some_to_1 {
    ($opt:ident) => {
        if $opt.is_some() { 1.0 } else { 0.0 }
    };
}

macro_rules! ui_group {
    ($region:ident, $ui:ident, $heading:expr, { $($rest:tt)* }) => {
        let count = ui_group!(@count 0.0, $($rest)*);
        $ui.group(|ui| {
            ui.heading($heading);
            ui.separator();
            ui.horizontal(|ui| {
                ui_group!(@glomp $region, ui, count, {
                    $($rest)*
                } => {});
            });
        })
    };

    (@count $count:expr, [ $($l:tt)+ ], [ $($r:tt)+ ], $($rest:tt)*) => {
        ui_group!(@count $count + 1.0, $($rest)*)
    };

    (@count $count:expr, 'if_some: $data_opt:ident { [ $($l:tt)+ ], [ $($r:tt)+ ], }, $($rest:tt)*) => {
        ui_group!(@count $count + some_to_1!($data_opt), $($rest)*)
    };

    (@count $count:expr,) => {
        $count
    };

    (@glomp $region:ident, $ui:ident, $count:expr, {
        [ $heading:expr$(=> $to_response:ident !)?, $grid_id:expr ],
        [ $($args:tt)+ ],
        $($rest:tt)*
    } => {
        $($fold:tt)*
    }) => {
        ui_group!(@glomp $region, $ui, $count - 1.0, {
            $($rest)*
        } => {
            $($fold)*
            ui_subgroup!($region, $ui, 1.0 / ($count), $heading$(=> $to_response !)?, $grid_id, { $($args)+ });
        })
    };

    (@glomp $region:ident, $ui:ident, $count:expr, {
        'if_some: $data_opt:ident {
            [ $heading:expr$(=> $_punc:tt)?, $grid_id:expr ],
            [ $($args:tt)+ ],
        },
        $($rest:tt)*
    } => {
        $($fold:tt)*
    }) => {
        // Store `count` here to not propagate `data_opt` immutable reference
        let count = $count;

        ui_group!(@glomp $region, $ui, $count - some_to_1!($data_opt), {
            $($rest)*
        } => {
            $($fold)*
            if let Some(ref mut data) = $data_opt {
                ui_subgroup!($region, $ui, (1.0 / count), $heading$(=> data$_punc)?, $grid_id, {
                    'with_some: data {
                        $($args)+
                    }
                });
            }
        })
    };

    (@glomp $region:ident, $ui:ident, $count:expr, {} => { $($fold:tt)* }) => {
        $($fold)*
    };
}

macro_rules! ui_id {
    ($region:ident, $id:ident (#direct), $($salt:tt)*) => {
        $region.uuid().with($id).with(stringify!($($salt)*))
    };

    ($region:ident, $data:expr, $($salt:tt)*) => {
        $region.uuid().with($data.id).with(stringify!($($salt)*))
    };
}

macro_rules! ui_input {
    ($region:ident, $ui:ident, $label:expr, $data:expr, $field:ident $((#$disabled:ident))? $(, $($t:tt)*)?) => {
        $ui.horizontal(|ui| {
            ui.label($label);
            ui_input!(@dsble ui $((#$disabled))?);
            ui.with_layout(Layout::right_to_left(), |ui| {
                let response = ui_input!(@glomp
                    NumericInputField::new(ui_id!($region, $data, $field), &mut $data.$field)
                    $(, $($t)*)?
                ).max_precision(8)
                 .default(
                    // This is set every frame, but only read on memory
                    // instantiation+invalidation. Should handle this in the
                    // input response...? (Empty input protocol)
                    $region.state
                        .unedited($data.id, stringify!($field))
                        .map(|default| NumericDefault::RcString(default)))
                 .show(ui);

                $region.ui_numeric_input_response(ui, $data.id, stringify!($field), response);
            });
        })
    };

    (@dsble $ui:ident) => {};

    (@dsble $ui:ident (#disabled)) => {
        $ui.set_enabled(false);
    };

    (@glomp $field:expr, % 'degrees $(, $($t:tt)*)?) => {
        ui_input!(@glomp NumericInputField::clamp_range($field, -360.0..=360.0), % 'suffix: "°" $(, % $($t)*)?)
    };

    (@glomp $field:expr, % 'prefix: $prefix:expr $(, $($t:tt)*)?) => {
        ui_input!(@glomp NumericInputField::prefix($field, $prefix) $(, % $($t)*)?)
    };

    (@glomp $field:expr, % 'range: $range:expr $(, $($t:tt)*)?) => {
        ui_input!(@glomp NumericInputField::clamp_range($field, $range) $(, % $($t)*)?)
    };

    (@glomp $field:expr, % 'salt: ($($salt:tt)*) $(, $($t:tt)*)?) => {
        ui_input!(@glomp NumericInputField::salt($field, stringify!($($salt)*)) $(, % $($t)*)?)
    };

    (@glomp $field:expr, % 'sci $(, $($t:tt)*)?) => {
        ui_input!(@glomp NumericInputField::scientific($field) $(, % $($t)*)?)
    };

    (@glomp $field:expr, % 'signed $(, $($t:tt)*)?) => {
        ui_input!(@glomp NumericInputField::signed($field, true) $(, % $($t)*)?)
    };

    (@glomp $field:expr, % 'suffix: $suffix:expr $(, $($t:tt)*)?) => {
        ui_input!(@glomp NumericInputField::suffix($field, $suffix) $(, % $($t)*)?)
    };

    (@glomp $field:expr) => {
        $field
    };
}

macro_rules! ui_subgroup {
    ($region:ident, $ui:ident, $scale:expr, $heading:expr$(=> $to_response:ident$($_punc:tt)?)?, $grid_id:expr, {
        $($input_args:tt)*
    }) => {
        $ui.group(|ui| {
            // For simplicity, 20.0px = item spacing + (2 * window margin)
            ui.set_width((ui.available_width() - (20.0 * ((1.0 / $scale) - 1.0))) * $scale);
            ui.vertical(|ui| {
                ui_subgroup!(@hding $region, ui, $heading$(=> $to_response$($_punc)?)?);
                ui.separator();
                Grid::new($grid_id)
                    .num_columns(1)
                    .min_col_width(ui.available_width())
                    .striped(true)
                    .show(ui, |ui|
                {
                    ui_subgroup!(@glomp $region, ui, { $($input_args)* } => { });
                });
            });
        });
    };

    (@glomp $region:ident, $ui:ident, { [ $($input_args:tt)* ], $($rest:tt)* } => { $($fold:tt)* }) => {
        ui_subgroup!(@glomp $region, $ui, { $($rest)* } => {
            $($fold)*
            ui_input!($region, $ui, $($input_args)*);
            $ui.end_row();
        })
    };

    (@glomp $region:ident, $ui:ident, {
        'if_cond: ( $cond:expr ) {
            $($cond_args:tt)*
        },
        $($rest:tt)*
    } => {
        $($fold:tt)*
    }) => {
        if $cond {
            ui_subgroup!(@glomp $region, $ui, { $($cond_args)* $($rest)* } => { $($fold)* });
        } else {
            ui_subgroup!(@glomp $region, $ui, { $($rest)* } => { $($fold)* });
        }
    };

    (@glomp $region:ident, $ui:ident, {
        'with_some: $data:ident {
            [ $label:expr, *, $($input_args:tt)* ],
            $($rest:tt)*
        }
    } => {
        $($fold:tt)*
    }) => {
        ui_subgroup!(@glomp $region, $ui, {
            'with_some: $data {
                $($rest)*
            }
        } => {
            $($fold)*
            ui_input!($region, $ui, $label, $data, $($input_args)*);
            $ui.end_row();
        })
    };

    (@glomp $region:ident, $ui:ident, {
        'with_some: $data:ident {
            'if_cond: ( $cond:expr ) {
                $($cond_args:tt)*
            },
            $($rest:tt)*
        }
    } => {
        $($fold:tt)*
    }) => {
        if $cond {
            ui_subgroup!(@glomp $region, $ui, { 'with_some: $data { $($cond_args)* $($rest)* } } => { $($fold)* });
        } else {
            ui_subgroup!(@glomp $region, $ui, { 'with_some: $data { $($rest)* } } => { $($fold)* });
        }
    };

    (@glomp $region:ident, $ui:ident, { $('with_some: $data:ident { })? } => { $($fold:tt)* }) => {
        $($fold)*
    };

    (@hding $region:ident, $ui:ident, $heading:expr) => {
        $ui.heading($heading);
    };

    (@hding $region:ident, $ui:ident, $heading:expr => $to_response:ident$($_punc:tt)?) => {
        $ui.scope(|ui| {
            $region.ui_interactive_heading_style(ui);
            let heading_response = ui.add(Label::new(RichText::new($heading).heading()).sense(Sense::click()));
            $region.ui_interactive_heading_response(ui, $to_response, heading_response);
        });
    };
}

#[derive(Serialize, Deserialize)]
pub struct NmdAppEditorRegion {
    #[serde(skip)]
    message_sender: Option<MessageSender>,
    state: NmdAppEditorProjectState,
    #[serde(skip)]
    transient_state: NmdAppEditorTransientState,
    uuid_source: u64,
}

#[derive(Serialize, Deserialize)]
struct NmdAppEditorProjectState {
    chains: HashMap<u16, u16>,
    map: BTreeMap<u16, NmdFileBone>,
    // Would prefer memory field keys being `&str`
    memory: HashMap<u16, HashMap<String, Rc<String>>>,
    selected_id: Option<u16>,
    selection: Option<ChainSnippet>,
}

#[derive(Default)]
struct NmdAppEditorTransientState {
    input_start: String,
    input_focus: bool,
    input_was_default: bool,
    memory_wipe: Option<HashSet<u16>>,
}

impl NmdAppEditorTransientState {
    fn clear_input_state(&mut self) {
        self.input_focus = false;
        self.input_start.clear();
        self.input_was_default = false;
    }
}

impl NmdAppEditorRegion {
    pub fn new(message_sender: &MessageSender, data: &NmdFileData) -> Self {
        Self {
            message_sender: Some(message_sender.to_owned()),
            state: NmdAppEditorProjectState::from(data),
            transient_state: Default::default(),
            uuid_source: region::generate_uuid_source(),
        }
    }

    fn commit(&mut self) {
        let memory = mem::take(&mut self.state.memory);

        self.transient_state.memory_wipe = Some(memory.into_keys().collect());
    }

    fn emit_edit(&self, bone_id: u16, edited: bool) {
        self.emit(Message::UiState(UiState::BoneData(bone_id, edited)));
    }

    fn emit_flag(&self, bone_id: u16, flag: NmdFileBoneFlag) {
        self.emit(Message::UiState(UiState::BoneFlag(bone_id, flag)));
    }

    fn emit_focus(&self, bone_id: u16, name: &String) {
        self.emit(Message::UiSelect(UiComponent::TreeNodeFocus(bone_id, name.to_owned())));
    }

    fn emit_name(&self, bone_id: u16, name: &String) {
        self.emit(Message::UiState(UiState::BoneName(bone_id, name.to_owned())));
    }

    fn emit_select_node(&self, bone_id: u16, name: &String) {
        self.emit(Message::UiSelect(UiComponent::TreeNode(bone_id, name.to_owned())));
    }

    pub fn emit_with(&mut self, message_sender: &MessageSender) {
        self.message_sender = Some(message_sender.to_owned());
    }

    fn handle_keys(&mut self, ctx: &Context) {
        if ctx.memory().focus().is_none() {
            if ctx.input().key_released(Key::B) {
                if let Some(prev) = self.state.prev_in_selection() {
                    self.emit_select_node(prev.id, &prev.name);
                }
            } else if ctx.input().key_released(Key::N) {
                if let Some(next) = self.state.next_in_selection() {
                    self.emit_select_node(next.id, &next.name);
                }
            }
        }
    }

    fn insert_memory(&mut self, bone_id: u16, field: &'static str, field_memory: &String) {
        if !self.state.edited(bone_id) {
            self.emit_edit(bone_id, true);
        }

        self.state.insert_memory(bone_id, field, field_memory);
    }

    #[inline]
    fn name_input_changed(&mut self, bone_data: &NmdFileBone) {
        let mut input_is_default = bone_data.name.is_empty();

        if let Some(unedited_name) = self.state.unedited_text(bone_data.id, "name") {
            input_is_default |= bone_data.name == unedited_name;

            if input_is_default {
                self.emit_name(bone_data.id, &unedited_name);
            }
        } else {
            let unedited_name = self.transient_state.input_start.to_owned();

            // Can skip implicit `emit_edit` if input is empty
            self.insert_memory(bone_data.id, "name", &unedited_name);
        }

        if !input_is_default {
            self.emit_name(bone_data.id, &bone_data.name);
        }

        if !self.state.edited_except(bone_data.id, "name") {
            if input_is_default {
                // Pretend this field isn't edited if it's empty; or, e.g. if
                // the field was backspaced and retyped.
                self.emit_edit(bone_data.id, false);
            } else if self.transient_state.input_was_default {
                // Necessary to undo the above.
                self.emit_edit(bone_data.id, true);
            }
        }

        self.transient_state.input_was_default = input_is_default;
    }

    #[inline]
    fn name_input_submitted(&mut self, bone_data: &mut NmdFileBone) {
        if self.transient_state.input_was_default {
            if bone_data.name.is_empty() {
                if let Some(unedited_name) = self.state.unedited_text(bone_data.id, "name") {
                    bone_data.name = unedited_name;
                }
            }

            self.remove_memory(bone_data.id, "name");
        } else if Self::reject_bone_name(&bone_data.name) {
            bone_data.name = self.transient_state.input_start.to_owned();

            if let Some(unedited_name) = self.state.unedited_text(bone_data.id, "name") {
                if unedited_name == bone_data.name {
                    self.remove_memory(bone_data.id, "name");
                }
            }

            self.emit_name(bone_data.id, &bone_data.name);
        }
    }

    fn reject_bone_name(name: &String) -> bool {
        name.chars().any(|c| !(c == '_' || c.is_ascii_alphanumeric()))
    }

    fn remove_in_ui_memory(&mut self, ctx: &Context, ids: &HashSet<u16>) {
        for id in ids {
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), translation_x));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), translation_y));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), translation_z));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), rotation_x));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), rotation_y));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), rotation_z));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), physics_constraint_x_max));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), physics_constraint_x_min));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), physics_constraint_y_max));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), physics_constraint_y_min));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), gravity_x));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), gravity_y));
            input_mem_utils::remove(ctx, ui_id!(self, id (#direct), translation_x_next));
        }
    }

    fn remove_in_ui_memory_if_pending(&mut self, ctx: &Context) {
        if let Some(ids) = mem::take(&mut self.transient_state.memory_wipe) {
            self.remove_in_ui_memory(ctx, &ids);
        }
    }

    fn remove_memory(&mut self, bone_id: u16, field: &'static str) {
        if let Some(_) = self.state.remove_memory(bone_id, field) {
            if !self.state.edited(bone_id) {
                self.emit_edit(bone_id, false);
            }
        }
    }

    /// Set the flag for the bone with the given ID.
    //
    // ** Don't call this from a UI function, because during that time the
    // editor does not own the selection.
    fn set_flag(&mut self, bone_id: u16, flag: NmdFileBoneFlag) {
        self.without_selection(|region| {
            if let Some(bone_data) = region.state.get_mut(bone_id) {
                let replaced_flag = mem::replace(&mut bone_data.flag, flag);

                if region.state.memory_equals(bone_id, "flag", &flag.to_string()) {
                    region.remove_memory(bone_id, "flag");
                } else {
                    region.insert_memory(bone_id, "flag", &replaced_flag.to_string());
                }

                region.state.on_flag_changed(bone_id, replaced_flag, flag);
            }
        });
    }

    // ** Don't call this from a UI function, because during that time the
    // editor does not own the selection.
    pub fn try_export(&mut self, path: &PathBuf, data: &NmdFileData) -> io::Result<()> {
        self.without_selection(|region| {
            match NmdFileWriter::try_from(path) {
                Ok(writer) => writer.write_new(data, &region.state.map),
                Err(error) => Err(error),
            }
        })
    }

    fn ui_flag_input(&mut self, ui: &mut Ui, bone_data: &NmdFileBone) {
        ui.horizontal(|ui| {
            let mut flag = bone_data.flag;
            let flag_box_width;

            ui.label("Type:");

            if let Some(unedited_flag) = self.state.unedited_text(bone_data.id, "flag") {
                self.ui_input_edited_mark(ui, &unedited_flag);
            }

            // (egui v0.18) Combo boxes given all available width (or drawn in a
            // right-to-left layout) will run off the screen by about the item
            // spacing width.
            flag_box_width = ui.available_width() - (ui.spacing().item_spacing.x + 0.5);

            ComboBox::from_id_source(ui_id!(self, bone_data, #flag))
                .selected_text(bone_data.flag.to_string())
                .width(flag_box_width)
                .show_ui(ui, |ui|
            {
                for flag_entry in NmdFileBoneFlag::iter() {
                    if ui.selectable_value(&mut flag, flag_entry, flag_entry.to_string())
                        .clicked()
                    {
                        // Delay `set_flag` to next frame
                        self.emit_flag(bone_data.id, flag);
                    }
                }
            });
        });
    }

    fn ui_header(&mut self, ui: &mut Ui, bone_data: &mut NmdFileBone) {
        ui.horizontal(|ui| {
            self.ui_name_input(ui, bone_data);
            ui.separator();
            self.ui_id(ui, bone_data);
            ui.separator();
            self.ui_flag_input(ui, bone_data);
        });
    }

    fn ui_id(&self, ui: &mut Ui, bone_data: &NmdFileBone) {
        ui.scope(|ui| {
            // `LayoutJob` to not highlight the text as a button
            let id_text = LayoutJob::single_section(
                format!("ID: {:#04X}", bone_data.id),
                TextFormat {
                    color: ui.visuals().widgets.noninteractive.fg_stroke.color,
                    ..Default::default()
                }
            );

            if ui.add(
                Label::new(id_text)
                    .sense(Sense::click()))
                .on_hover_cursor(CursorIcon::PointingHand)
                .on_hover_text_at_pointer(&bone_data.name)
                .clicked()
            {
                self.emit_focus(bone_data.id, &bone_data.name);
            }
        });
    }

    fn ui_interactive_heading_response(&self, ui: &mut Ui, bone_data: &NmdFileBone, mut response: Response) {
        response = response
            .on_hover_cursor(CursorIcon::PointingHand)
            .on_hover_text_at_pointer(&bone_data.name);

        if response.clicked() {
            self.emit_select_node(bone_data.id, &bone_data.name);
        } else if response.secondary_clicked() {
            self.emit_focus(bone_data.id, &bone_data.name);
        }
    }

    fn ui_interactive_heading_style(&self, ui: &mut Ui) {
        ui.visuals_mut().widgets.active.fg_stroke = (*UiColor).common.gray.normal_stroke();
        ui.visuals_mut().widgets.hovered.fg_stroke = (*UiColor).common.gray.normal_stroke();
        ui.visuals_mut().widgets.inactive.fg_stroke = (*UiColor).common.gray.normal_stroke();
    }

    fn ui_input_edited_mark(&mut self, ui: &mut Ui, original_display_text: &String) {
        ui.add(
            Label::new(
                RichText::new("✱")
                    .small()
                    .color((*UiColor).editor.modified.normal()))
                // TODO: Want to make hover text appear on mouse down; can
                // achieve this by sensing clicks, but then element is
                // "interactable" therefore gets tab focus; don't want this...
                // .sense(Sense::click())
        ).on_hover_cursor(CursorIcon::Help)
         .on_hover_ui_at_pointer(|ui| {
            let mut text = LayoutJob::default();

            text.append("Modified from: ", 0.0, TextFormat::default());
            text.append(original_display_text, 0.0, TextFormat { color: (*UiColor).common.weak_gray.normal(), ..Default::default() });

            ui.label(text);
        }).surrender_focus();
    }

    fn ui_name_input(&mut self, ui: &mut Ui, bone_data: &mut NmdFileBone) {
        let name_input;

        ui.label("Name:");

        if self.transient_state.input_focus {
            if Self::reject_bone_name(&bone_data.name) {
                self.ui_name_rejected_mark(ui, &bone_data.name);
            }
        } else if let Some(unedited_name) = self.state.unedited_text(bone_data.id, "name") {
            self.ui_input_edited_mark(ui, &unedited_name);
        }

        name_input = ui.add(
            TextEdit::singleline(&mut bone_data.name) 
                .id(ui_id!(self, bone_data, #name_input))
                .desired_width(160.0));

        if name_input.gained_focus() {
            self.transient_state.input_focus = true;
            self.transient_state.input_start = bone_data.name.to_owned();
        } else if name_input.changed() {
            self.name_input_changed(bone_data);
        } else if !name_input.has_focus() && self.transient_state.input_focus {
            // (egui v0.18) There may be a bug where `lost_focus` can get jammed
            // and stop reporting; `input_focus` bool is a workaround
            self.name_input_submitted(bone_data);
            self.transient_state.clear_input_state();
        }
    }

    fn ui_name_rejected_mark(&self, ui: &mut Ui, bone_name: &String) {
        ui.label(RichText::new("！").color((*UiColor).editor.error.normal()))
            .on_hover_cursor(CursorIcon::Help)
            .on_hover_ui(|ui|
        {
            ui.set_width(240.0);
            ui.label("Only alphabetic, numeric and underscore characters are allowed here.");
        });
    }

    fn ui_numeric_inputs(&mut self, ui: &mut Ui, ChainSnippet (prev_opt, current, next_opt): &mut ChainSnippet) {
        // NOTE: Fields used here should have corresponding line in
        // `Self::remove_in_ui_memory`
        ui.with_layout(Layout::top_down_justified(Align::Center), |ui| {
            ui_group!(self, ui, "Placement", {
                ["Translation", "region$editor$translation"],
                [
                    ["x:", current, translation_x, % 'suffix: "cm"],
                    ["y:", current, translation_y, % 'suffix: "cm"],
                    ["z:", current, translation_z, % 'suffix: "cm"],
                ],
                ["Rotation", "region$editor$rotation"],
                [
                    ["x:", current, rotation_x, % 'degrees],
                    ["y:", current, rotation_y, % 'degrees],
                    ["z:", current, rotation_z, % 'degrees],
                ],
            });

            ui_group!(self, ui, "Parameters", {
                ["Constraints", "region$editor$constraints"],
                [
                    ["max x:", current, physics_constraint_x_max, % 'suffix: "°"],
                    ["min x:", current, physics_constraint_x_min, % 'suffix: "°"],
                    ["max y:", current, physics_constraint_y_max, % 'suffix: "°"],
                    ["min y:", current, physics_constraint_y_min, % 'suffix: "°"],
                ],
                ["Gravity", "region$editor$gravity"],
                [
                    ["x:", current, gravity_x],
                    ["y:", current, gravity_y],
                ],
            });

            if prev_opt.is_some() || next_opt.is_some() {
                ui_group!(self, ui, "Chain", {
                    'if_some: prev_opt {
                        ["Previous" => *, "editor$chain$prev"],
                        [
                            ["next x:", *, translation_x_next, % 'suffix: "cm"],
                        ],
                    },
                    ["Current" => current!, "editor$chain$curr"],
                    [
                        'if_cond: (prev_opt.is_some()) {
                            ["x:", current, translation_x (#disabled),
                             % 'salt: (#dupe), 'suffix: "cm"],
                        },
                        'if_cond: (next_opt.is_some()) {
                            ["next x:", current, translation_x_next, % 'suffix: "cm"],
                        },
                    ],
                    'if_some: next_opt {
                        ["Next" => *, "editor$chain$next"],
                        [
                            ["x:", *, translation_x, % 'suffix: "cm"],
                        ],
                    },
                });
            }
        });
    }

    fn ui_numeric_input_response(&mut self, ui: &mut Ui, bone_id: u16, field: &'static str, response: InputFieldResponse) {
        let memory = response.memory.lock()
            .unwrap();

        match response.inner_response {
            InputFieldInnerResponse::Button(inner_response) => {
                if memory.deviates() {
                    self.ui_input_edited_mark(ui, &memory.original_display_text);
                } else if memory.reverted() {
                    // Catch the case where some invalid input is rejected,
                    // and the field reverts to default value; this won't be
                    // caught during text edit frame.
                    self.remove_memory(bone_id, field);
                }
            }
            InputFieldInnerResponse::TextEdit(inner_response) => {
                if inner_response.changed() {
                    if memory.deviates() {
                        self.insert_memory(bone_id, field, &memory.original_value_text);
                    } else {
                        self.remove_memory(bone_id, field);
                    }
                }
            }
        }
    }

    fn with_selection<R>(&mut self, mut routine: impl FnMut(&mut Self, &mut ChainSnippet) -> R) -> Option<R> {
        let mut chain_snippet = self.state.take_selection()?;
        let result = Some(routine(self, &mut chain_snippet));

        self.state.give_selection(chain_snippet);

        result
    }

    fn without_selection<R>(&mut self, mut routine: impl FnMut(&mut Self) -> R) -> R {
        let selection_id_opt = self.state.unselect();
        let result = routine(self);
        
        if let Some(selection_id) = selection_id_opt {
            self.state.select(selection_id);
        }

        result
    }
}

impl NmdAppRegion for NmdAppEditorRegion {
    fn message_sender(&self) -> Option<&MessageSender> {
        self.message_sender.as_ref()
    }

    fn receive_message(&mut self, message: &Message) {
        match message {
            Message::UiState(UiState::BoneFlag(id, flag))
                => { self.set_flag(*id, *flag); }
            Message::UiState(UiState::TreeNodeCopyPaste(id_copy_map, parent_id))
                => { self.state.on_copy_paste(id_copy_map, *parent_id); }
            Message::UiState(UiState::TreeNodeCutPaste(id, new_parent_id))
                => { self.state.on_cut_paste(*id, *new_parent_id); }
            Message::UiState(UiState::TreeNodeDelete(root_id, removed_ids))
                => { self.state.on_deleted(*root_id, removed_ids); self.transient_state.memory_wipe = Some(removed_ids.to_owned()); }
            _   => {}
        }
    }

    fn select(&mut self, ui_component: &UiComponent) {
        use UiComponent::*;

        match ui_component {
            MenuCommit
                => { self.commit(); }
            TreeNode(bone_id, _)
                => { self.state.select(*bone_id); }
            _   => {}
        }
    }

    fn ui(&mut self, ctx: &Context) {
        self.handle_keys(ctx);
        self.remove_in_ui_memory_if_pending(ctx);

        if self.state.has_selection() {
            self.with_selection(|region, chain_snippet| {
                CentralPanel::default()
                    .show(ctx, |ui|
                {
                    ScrollArea::both()
                        .id_source("editor$body")
                        .show(ui, |ui|
                    {
                        ui.set_min_size(Vec2 {
                            x: 604.0,
                            y: ui.available_height()
                        });

                        region.ui_header(ui, chain_snippet.current_mut());
                        ui.separator();
                        region.ui_numeric_inputs(ui, chain_snippet);
                    });
                });
            });
        } else {
            CentralPanel::default()
                .show(ctx, |ui|
            {});
        }
    }

    fn uuid_source(&self) -> u64 {
        self.uuid_source
    }
}

impl NmdAppEditorProjectState {
    // ### Cat.: Lookup

    fn chain_eligible(&self, bone_id: u16) -> bool {
        if let Some(bone_data) = self.get(bone_id) {
            bone_data.is_phys() && !self.chained(bone_id)
        } else {
            false
        }
    }

    fn chained(&self, bone_id: u16) -> bool {
        self.chains.contains_key(&bone_id)
    }

    fn get(&self, bone_id: u16) -> Option<&NmdFileBone> {
        self.map.get(&bone_id)
    }

    fn get_mut(&mut self, bone_id: u16) -> Option<&mut NmdFileBone> {
        self.map.get_mut(&bone_id)
    }

    fn get_parent_id(&self, bone_id: u16) -> Option<u16> {
        if let Some(bone_data) = self.get(bone_id) {
            Some(bone_data.parent_id)
        } else {
            None
        }
    }

    /// Find a candidate for the next bone in a new chain. This is inefficient.
    fn next_for_chain(&self, bone_id: u16) -> Option<&NmdFileBone> {
        self.map
            .values()
            .find(|bone_data| bone_data.is_phys()
                              && bone_data.parent_id == bone_id)
    }

    fn next_in_chain(&self, bone_id: u16) -> Option<&NmdFileBone> {
        self.get(*self.chains.get(&bone_id)?)
    }

    fn next_in_selection(&self) -> Option<&NmdFileBone> {
        self.selection.as_ref()?.next()
    }

    /// Find a candidate for the previous bone in a new chain.
    fn prev_for_chain(&self, bone_id: u16) -> Option<&NmdFileBone> {
        if let Some(prev) = self.get(self.get(bone_id)?.parent_id) {
            if prev.is_phys() {
                return Some(prev);
            }
        }

        None
    }

    fn prev_in_chain(&self, bone_id: u16) -> Option<&NmdFileBone> {
        if let Some(prev) = self.get(self.get(bone_id)?.parent_id) {
            // May be redundant check but can't be sure with forked chains
            if self.chains.contains_key(&prev.id) {
                return Some(prev);
            }
        }

        None
    }

    fn prev_in_selection(&self) -> Option<&NmdFileBone> {
        self.selection.as_ref()?.prev()
    }

    // ### Cat.: Memory

    fn insert_memory(&mut self, bone_id: u16, field: &str, field_memory: &String) {
        // Non-overwriting insert
        self.memory.entry(bone_id)
            .or_default()
            .entry(field.to_string())
            .or_insert_with(|| Rc::new(field_memory.to_owned()));
    }

    fn edited(&self, bone_id: u16) -> bool {
        self.memory.contains_key(&bone_id)
    }

    fn edited_except(&self, bone_id: u16, field: &str) -> bool {
        if let Some(bone_memory) = self.memory.get(&bone_id) {
            bone_memory.len() > 1
                || (bone_memory.len() == 1 && !bone_memory.contains_key(field))
        } else {
            true
        }
    }

    fn edited_in(&self, bone_id: u16, field: &str) -> bool {
        if let Some(bone_memory) = self.memory.get(&bone_id) {
            bone_memory.contains_key(field)
        } else {
            false
        }
    }

    fn memory_equals(&self, bone_id: u16, field: &str, field_memory: &String) -> bool {
        self.unedited_text_ref(bone_id, field) == Some(field_memory)
    }

    fn remove_memory(&mut self, bone_id: u16, field: &str) -> Option<Rc<String>> {
        if let Some(mut bone_memory) = self.memory.get_mut(&bone_id) {
            let removed = bone_memory.remove(&field.to_string());

            if bone_memory.is_empty() {
                self.memory.remove(&bone_id);
            }

            removed
        } else {
            None
        }
    }

    fn unedited(&self, bone_id: u16, field: &str) -> Option<Rc<String>> {
        self.memory
            .get(&bone_id)
            .and_then(|bone_memory| bone_memory.get(&field.to_string()))
            .cloned()
    }

    fn unedited_text(&self, bone_id: u16, field: &str) -> Option<String> {
        self.unedited_text_ref(bone_id, field).cloned()
    }

    fn unedited_text_ref(&self, bone_id: u16, field: &str) -> Option<&String> {
        self.memory
            .get(&bone_id)
            .and_then(|bone_memory| bone_memory.get(&field.to_string()))
            .map(|rc_string| rc_string.as_ref())
    }

    // ### Cat.: Selection

    fn give(&mut self, bone_data: NmdFileBone) {
        self.map.insert(bone_data.id, bone_data);
    }

    fn give_opt(&mut self, bone_data_opt: Option<NmdFileBone>) {
        if let Some(bone_data) = bone_data_opt {
            self.give(bone_data);
        }
    }

    fn give_selection(&mut self, chain_snippet: ChainSnippet) {
        self.selection = Some(chain_snippet);
    }

    fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    fn select(&mut self, bone_id: u16) {
        if self.selected_id != Some(bone_id) {
            self.unselect();

            if let Some(current) = self.map.remove(&bone_id) {
                let (prev_opt, next_opt) = if current.is_phys() {
                    self.take_chained(&current)
                } else {
                    (None, None)
                };

                self.selected_id = Some(bone_id);
                self.selection = Some(ChainSnippet(prev_opt, current, next_opt));
            }
        }
    }

    fn take(&mut self, bone_id: u16) -> Option<NmdFileBone> {
        self.map.remove(&bone_id)
    }

    fn take_chained(&mut self, current: &NmdFileBone) -> (Option<NmdFileBone>, Option<NmdFileBone>) {
        let prev_opt = self.take_if_chain(current.parent_id, current.id);
        let next_opt = if let Some(next_id) = self.chains.get(&current.id) {
            self.take(*next_id)
        } else {
            None
        };

        (prev_opt, next_opt)
    }

    fn take_if_chain(&mut self, bone_id: u16, next_id: u16) -> Option<NmdFileBone> {
        if let Some(bone_data) = self.take(bone_id) {
            if self.chains.get(&bone_id) == Some(&next_id) {
                Some(bone_data)
            } else {
                self.map.insert(bone_id, bone_data);

                None
            }
        } else {
            None
        }
    }

    fn take_selection(&mut self) -> Option<ChainSnippet> {
        mem::take(&mut self.selection)
    }

    fn unselect(&mut self) -> Option<u16> {
        if let Some(chain_snippet) = mem::take(&mut self.selection) {
            let ChainSnippet (prev_opt, current, next_opt) = chain_snippet;

            self.give_opt(prev_opt);
            self.give(current);
            self.give_opt(next_opt);

            mem::take(&mut self.selected_id)
        } else {
            None
        }
    }

    fn without_selection<R>(&mut self, mut routine: impl FnMut(&mut Self) -> R) -> R {
        let selection_id_opt = self.unselect();
        let result = routine(self);

        if let Some(selection_id) = selection_id_opt {
            self.select(selection_id);
        }

        result
    }

    // ### Cat.: State response

    #[inline]
    fn delete_in_chains(&mut self, ids: &HashSet<u16>) {
        self.chains.retain(|id, next_id| !(ids.contains(id) || ids.contains(next_id)));
    }

    #[inline]
    fn delete_in_map(&mut self, ids: &HashSet<u16>) {
        self.map.retain(|id, _| !ids.contains(id));
    }

    #[inline]
    fn delete_in_memory(&mut self, ids: &HashSet<u16>) {
        self.memory.retain(|id, _| !ids.contains(id));
    }

    fn on_copy_paste(&mut self, id_copy_map: &HashMap<u16, (u16, String)>, parent_id: u16) {
        self.without_selection(|state| {
            state.on_copy_paste_internal(id_copy_map, parent_id);
        });
    }

    #[inline]
    fn on_copy_paste_internal(&mut self, id_copy_map: &HashMap<u16, (u16, String)>, parent_id: u16) {
         for (source_id, (target_id, name)) in id_copy_map {
            if let Some(source) = self.get(*source_id) {
                let mut target = source.to_owned();

                target.name = name.to_owned();
                target.id = *target_id;
                target.parent_id = *id_copy_map.get(&target.parent_id)
                    .and_then(|(id, _)| Some(id))
                    .unwrap_or(&parent_id);

                if target.parent_id == parent_id && target.is_phys() {
                    if let Some(parent) = self.get(parent_id) {
                        // I wish for trivial overrides
                        if self.chain_eligible(parent_id) {
                            self.chains.insert(parent_id, target.id);
                        }
                    }
                }

                self.map.insert(target.id, target);
            }

            if let Some(source_next_id) = self.chains.get(source_id) {
                if let Some((target_next_id, _)) = id_copy_map.get(source_next_id) {
                    self.chains.insert(*target_id, *target_next_id);
                }
            }
        }
    }

    fn on_cut_paste(&mut self, bone_id: u16, new_parent_id: u16) {
        self.without_selection(|state| {
            state.on_cut_paste_internal(bone_id, new_parent_id);
        });
    }

    #[inline]
    fn on_cut_paste_internal(&mut self, bone_id: u16, new_parent_id: u16) {
        if let Some(bone_data) = self.get_mut(bone_id) {
            let old_parent_id = mem::replace(&mut bone_data.parent_id, new_parent_id);

            if bone_data.is_phys() {
                if self.chains.get(&old_parent_id) == Some(&bone_id) {
                    self.chains.remove(&old_parent_id);
                    self.rechain_maybe(old_parent_id);
                }

                if self.chain_eligible(new_parent_id) {
                    self.chains.insert(new_parent_id, bone_id);
                }
            }
        }
    }

    fn on_deleted(&mut self, root_id: u16, ids: &HashSet<u16>) {
        self.without_selection(|state| {
            let parent_id_opt = state.get_parent_id(root_id);

            state.delete_in_chains(ids);
            state.delete_in_map(ids);
            state.delete_in_memory(ids);

            if let Some(parent_id) = parent_id_opt {
                state.rechain_maybe(parent_id);
            }
        });
    }

    fn on_flag_changed(&mut self, bone_id: u16, flag_from: NmdFileBoneFlag, flag_to: NmdFileBoneFlag) {
        match (flag_from.is_phys(), flag_to.is_phys()) {
            (true, false) => {
                self.chains.remove(&bone_id);

                if let Some(prev) = self.prev_in_chain(bone_id) {
                    let prev_id = prev.id;

                    self.chains.remove(&prev_id);
                    self.rechain_maybe(prev_id);
                }
            }
            (false, true) => {
                if let Some(prev) = self.prev_for_chain(bone_id) {
                    self.chains.insert(prev.id, bone_id);
                }

                if let Some(next) = self.next_for_chain(bone_id) {
                    self.chains.insert(bone_id, next.id);
                }
            }
            _ => {}
        }
    }

    fn rechain_maybe(&mut self, bone_id: u16) -> bool {
        // TODO: FEAT:FORKS
        // "Rechaining" protocol for operations that might cause chains to
        // change. Not very specific, but at least shouldn't disturb existing
        // chains
        if self.chain_eligible(bone_id) {
            if let Some(next) = self.next_for_chain(bone_id) {
                self.chains.insert(bone_id, next.id);

                return true;
            }
        }

        false
    }
}

impl From<&NmdFileData> for NmdAppEditorProjectState {
    fn from(data: &NmdFileData) -> Self {
        Self {
            chains: data
                .tree_with(|bone_data| bone_data.is_phys())
                .iter()
                .filter_map(|tree|
            {
                if let Some(child) = tree.children.first() {
                    // If both are physics...
                    if *tree.data() && *child.data() {
                        return Some((tree.id(), child.id()));
                    }
                }

                None
            }).collect(),
            map: data.bones.to_owned(),
            memory: Default::default(),
            selected_id: None,
            selection: None,
        }
    }
}

impl ChainSnippet {
    fn current(&self) -> &NmdFileBone {
        &self.1
    }

    fn current_mut(&mut self) -> &mut NmdFileBone {
        &mut self.1
    }

    fn next(&self) -> Option<&NmdFileBone> {
        self.2.as_ref()
    }

    fn prev(&self) -> Option<&NmdFileBone> {
        self.0.as_ref()
    }
}
