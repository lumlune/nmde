use {
    super::{
        InputField,
    },
    std::sync::{Arc, Mutex, MutexGuard},
    eframe::egui::*,
    eframe::egui::util::id_type_map::IdTypeMap,
};

#[derive(Clone, Eq, PartialEq)]
struct Editing(bool);
#[derive(Clone, Eq, PartialEq)]
struct Invalidate(bool);

#[derive(Clone)]
pub struct InputFieldMemoryData {
    pub display_text: String,
    pub original_display_text: String,
    pub value_text: String,
    pub original_value_text: String,
    owner_memory_id: Id,
    pub(in super) reverted: bool,
}

impl InputFieldMemoryData {
    pub fn new<'a, I>(owner: &mut impl InputFieldMemory<'a, I>) -> Self
        where I: Clone + PartialEq + ToString,
    {
        let value = owner.get_value();
        let value_original = owner
            .default()
            .unwrap_or(value.to_owned());

        Self {
            value_text: value.to_string(),
            original_value_text: value_original.to_string(),
            display_text: owner.display(value),
            original_display_text: owner.display(value_original),
            owner_memory_id: owner.memory_id(),
            reverted: false,
        }
    }

    pub fn deviates(&self) -> bool {
        self.value_text != "" && self.value_text != self.original_value_text
    }

    // TODO: Deprecate probably
    pub fn invalidate(&self, ui: &mut Ui) {
        ui.memory().data.insert_temp(self.owner_memory_id, Invalidate(true));
    }

    pub fn reverted(&self) -> bool {
        self.reverted
    }

    pub fn update<'a, I>(&mut self, owner: &mut impl InputFieldMemory<'a, I>)
        where I: Clone + PartialEq + ToString,
    {
        let value = owner.get_value();

        self.value_text = value.to_string();
        self.display_text = owner.display(value);
    }
}

pub trait InputFieldMemory<'a, I>: InputField<'a, Input = I> + Sized
    where I: Clone + PartialEq + ToString,
{
    fn commit(&mut self, memory: &mut MutexGuard<InputFieldMemoryData>) {
        let prev_value = self.get_value();

        if memory.value_text != "" {
            self.set_value(&memory.value_text);
            // Few possible cases:
            //
            // 1) Value text could not be parsed, or
            // 2) Value text was parsed but same as previous, or
            // 3) Value text was parsed but value did not change due to e.g.
            //    floating point limitations
            //
            // #2 is not distinguished here but doesn't matter for current
            // usage; false positive not harmful (for now)
            memory.reverted = prev_value == self.get_value();
        } else {
            self.set_value(&memory.original_value_text);
            memory.reverted = true;
        }

        memory.update(self);
    }

    fn default(&self) -> Option<Self::Input> {
        None
    }

    fn memory(&mut self, ui: &mut Ui) -> Arc<Mutex<InputFieldMemoryData>> {
        let memory_map = &mut ui.memory().data;

        if memory_map.get_temp(self.memory_id()) == Some(Invalidate(true)) {
            memory_map.remove::<Arc<Mutex<InputFieldMemoryData>>>(self.memory_id());
            memory_map.remove::<Invalidate>(self.memory_id());
            // Keep `Editing`
        }

        if let Some(memory) = memory_map.get_temp(self.memory_id()) {
            memory
        } else {
            let memory = Arc::new(Mutex::new(InputFieldMemoryData::new(self)));

            memory_map.insert_temp(self.memory_id(), memory.clone());
            memory
        }
    }

    /// An ID which may or may not be the same as `Self::widget_id`; a distinct
    /// memory ID is useful if multiple widgets need to share the same memory.
    fn memory_id(&self) -> Id {
        self.widget_id()
    }

    fn set_editing(&self, ui: &mut Ui, editing: bool) {
        if editing {
            ui.memory().data.insert_temp(self.widget_id(), Editing(true));
        } else {
            ui.memory().data.remove::<Editing>(self.widget_id());
        }
    }

    fn was_editing(&self, ui: &Ui) -> bool {
        ui.memory().data.get_temp(self.widget_id()) == Some(Editing(true))
    }

    fn widget_id(&self) -> Id;
}

pub fn remove(ctx: &Context, memory_id: Id) {
    let memory_map = &mut ctx.memory().data;

    memory_map.remove::<Arc<Mutex<InputFieldMemoryData>>>(memory_id);
    memory_map.remove::<Invalidate>(memory_id);
}

