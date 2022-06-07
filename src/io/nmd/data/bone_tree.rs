use {
    crate::io::nmd::anatomy::NmdFileBone,
    std::{
        collections::{BTreeSet as OrderedSet, VecDeque, btree_set::IntoIter as OrderedIter},
        iter,
        ops::RangeInclusive,
    },
    serde::{
        ser,
        Deserialize,
        Serialize,
    },
};

/*
 * TODO:
 * ~ Add size field
 * ~ Indicate failure when out of IDs in `transform` functions
 *
 * NOTE:
 * ~ In hindsight `impl Default` on the associated data was better than this
 * mess
 */

#[derive(Default, Serialize, Deserialize)]
pub struct NmdFileBoneTreeRoot<A> {
    pub children: Vec<NmdFileBoneTree<A>>,
}

#[derive(Serialize, Deserialize)]
pub struct NmdFileBoneTree<A> {
    pub children: Vec<Self>,
    data: NmdFileBoneTreeData<A>,
}

#[derive(Serialize, Deserialize)]
pub struct NmdFileBoneTreeData<A> {
    associated_data: A,
    id: u16,
    parent_id: u16,
}

pub struct NmdFileBoneTreeIterator<'a, A> {
    stack: VecDeque<&'a NmdFileBoneTree<A>>,
}

struct NmdFileBoneTreeIdGenerator { 
    ids: RangeInclusive<u16>,
    used_ids: OrderedIter<u16>,
    used_id_echo: u16,
}

pub trait NmdFileBoneTreeNode {
    type Data;

    fn assimilate(&mut self, orphans: &mut VecDeque::<NmdFileBoneTree<Self::Data>>) {
        for i in 0..orphans.len() {
            let orphan = orphans.pop_front().unwrap();

            if let Some(unclaimed) = self.insert(orphan) {
                orphans.push_back(unclaimed);
            }
        }

        if orphans.len() > 0 {
            for child in self.children_mut() {
                child.assimilate(orphans);
            }
        }
    }

    fn at_path(&self, path: &VecDeque<u16>) -> Option<&NmdFileBoneTree<Self::Data>> {
        if let Some(first_id) = path.get(0) {
            let mut next = self.child(*first_id);

            for i in 1.. {
                if let Some(next_id) = path.get(i) {
                    next = next?.child(*next_id);
                } else {
                    return next;
                }
            }
        }

        None
    }

    fn at_path_mut(&mut self, path: &VecDeque<u16>) -> Option<&mut NmdFileBoneTree<Self::Data>> {
        if let Some(first_id) = path.get(0) {
            let mut next = self.child_mut(*first_id);

            for i in 1.. {
                if let Some(next_id) = path.get(i) {
                    next = next?.child_mut(*next_id);
                } else {
                    return next;
                }
            }
        }

        None
    }

    fn child(&self, id: u16) -> Option<&NmdFileBoneTree<Self::Data>> {
        self.children()
            .into_iter()
            .find(|child| child.id() == id)
    }

    fn child_index(&self, child_id: u16) -> Option<usize> {
        for (i, child) in self.children().iter().enumerate() {
            if child.id() == child_id {
                return Some(i);
            }
        }

        None
    }

    fn child_mut(&mut self, id: u16) -> Option<&mut NmdFileBoneTree<Self::Data>> {
        self.children_mut()
            .into_iter()
            .find(|child| child.id() == id)
    }

    fn children(&self) -> &Vec<NmdFileBoneTree<Self::Data>>;
    fn children_mut(&mut self) -> &mut Vec<NmdFileBoneTree<Self::Data>>;
    fn data_mut_opt(&mut self) -> Option<&mut Self::Data>;
    fn data_opt(&self) -> Option<&Self::Data>;

    fn find(&self, id: u16) -> Option<&NmdFileBoneTree<Self::Data>> {
        self.iter()
            .find(|descendent| descendent.id() == id)
    }

    fn find_mut(&mut self, id: u16) -> Option<&mut NmdFileBoneTree<Self::Data>> {
        if self.id() == id {
            self.mut_opt()
        } else {
            self.children_mut()
                .into_iter()
                .filter_map(|child| child.find_mut(id))
                .next()
        }
    }

    fn find_parent(&self, child_id: u16) -> Option<&dyn NmdFileBoneTreeNode<Data = Self::Data>>
        where Self: Sized,
    {
        if self.children()
            .iter()
            .any(|child| child.id() == child_id)
        {
            Some(self)
        } else {
            self.children()
                .into_iter()
                .filter_map(|child| child.find_parent(child_id))
                .next()
        }
    }

    fn find_parent_mut(&mut self, child_id: u16) -> Option<&mut dyn NmdFileBoneTreeNode<Data = Self::Data>>
        where Self: Sized,
    {
        if self.children()
            .iter()
            .any(|child| child.id() == child_id)
        {
            Some(self)
        } else {
            self.children_mut()
                .into_iter()
                .filter_map(|child| child.find_parent_mut(child_id))
                .next()
        }
    }

    fn for_path_mut<F: FnMut(&mut NmdFileBoneTree<Self::Data>)>(&mut self, path: &VecDeque<u16>, mut routine: F)
        where Self: Sized
    {
        for_path_mut_internal(self, path, 0, &mut routine);
    }

    fn give(&mut self, children: Vec<NmdFileBoneTree<Self::Data>>) {
        self.children_mut().extend(children);
    }

    fn give_child(&mut self, mut child: NmdFileBoneTree<Self::Data>) {
        child.set_parent_id(self.id());

        self.children_mut().push(child);
    }

    fn give_child_at_index(&mut self, mut child: NmdFileBoneTree<Self::Data>, mut index: usize) {
        child.set_parent_id(self.id());

        index = index.min(self.children().len());

        self.children_mut().insert(index, child);
    }

    fn id(&self) -> u16;
    fn iter(&self) -> NmdFileBoneTreeIterator<Self::Data>;

    fn insert(&mut self, descendent: NmdFileBoneTree<Self::Data>) -> Option<NmdFileBoneTree<Self::Data>> {
        match self.find_mut(descendent.parent_id()) {
            Some(parent) => {
                parent.children.push(descendent);
                None
            },
            None => Some(descendent),
        }
    }

    fn insert_at_path(&mut self, path: &VecDeque<u16>, descendent: NmdFileBoneTree<Self::Data>) -> Option<NmdFileBoneTree<Self::Data>> {
        if let Some(mut parent) = self.at_path_mut(path) {
            parent.give_child(descendent); 

            None
        } else {
            Some(descendent)
        }
    }

    fn mut_opt(&mut self) -> Option<&mut NmdFileBoneTree<Self::Data>>;
    fn opt(&self) -> Option<&NmdFileBoneTree<Self::Data>>;

    fn parent_id(&self) -> u16;

    fn path_to(&self, id: u16) -> (VecDeque<u16>, Option<&dyn NmdFileBoneTreeNode<Data = Self::Data>>)
        where Self: Sized,
    {
        let mut path = VecDeque::new();
        let target = path_to_internal(self, id, &mut path);

        (path, target)
    }

    fn path_to_except(&self, id: u16) -> (VecDeque<u16>, Option<&dyn NmdFileBoneTreeNode<Data = Self::Data>>)
        where Self: Sized,
    {
        let (mut path, target) = self.path_to(id);
        path.pop_back();

        (path, target)
    }

    fn path_to_mut(&mut self, id: u16) -> (VecDeque<u16>, Option<&mut dyn NmdFileBoneTreeNode<Data = Self::Data>>)
        where Self: Sized,
    {
        let mut path = VecDeque::new();
        let target = path_to_mut_internal(self, id, &mut path);

        (path, target)
    }

    fn path_to_mut_except(&mut self, id: u16) -> (VecDeque<u16>, Option<&mut dyn NmdFileBoneTreeNode<Data = Self::Data>>)
        where Self: Sized,
    {
        let (mut path, target) = self.path_to_mut(id);
        path.pop_back();

        (path, target)
    }

    fn path_to_parent_mut(&mut self, id: u16) -> (VecDeque<u16>, Option<&mut dyn NmdFileBoneTreeNode<Data = Self::Data>>)
        where Self: Sized,
    {
        let mut path = VecDeque::new();
        let target = path_to_parent_mut_internal(self, id, &mut path);

        (path, target)
    }

    fn set_id(&mut self, id: u16);

    fn set_parent(&mut self, id: u16, parent_id: u16) -> bool
        where Self: Sized
    {
        if let Some(parent) = self.find_parent_mut(id) {
            if let Some(mut child) = parent.take_child(id) {
                child.set_parent_id(parent_id);

                return self.insert(child).is_none();
            }
        }

        false
    }

    fn set_parent_id(&mut self, parent_id: u16);

    fn take(&mut self) -> Vec<NmdFileBoneTree<Self::Data>> {
        self.children_mut().drain(..).collect()
    }

    fn take_child(&mut self, child_id: u16) -> Option<NmdFileBoneTree<Self::Data>> {
        self.children()
            .iter()
            .enumerate()
            .filter_map(|(i, child)| (child.id() == child_id).then(|| i))
            .next()
            .and_then(|i| Some(self.children_mut().remove(i)))
    }
}

impl<A> NmdFileBoneTreeRoot<A> {
    fn new() -> Self {
        Self {
            children: vec!(),
        }
    }

    pub fn new_with<'a, I>(iterable: I, assoc_fn: impl Fn(&NmdFileBone) -> A) -> Self
        where I: IntoIterator<Item = &'a NmdFileBone>
    {
        let mut root = Self::new();
        let mut orphans = VecDeque::new();

        for bone_data in iterable {
            let orphan = NmdFileBoneTree::new(&bone_data, assoc_fn(&bone_data));

            if bone_data.parent_id == NmdFileBone::ROOT_BONE_ID {
                root.children.push(orphan);
            } else {
                orphans.push_back(orphan);
            }
        }

        root.assimilate(&mut orphans);
        root
    }
}

impl<A> NmdFileBoneTreeNode for NmdFileBoneTreeRoot<A> {
    type Data = A;

    fn children(&self) -> &Vec<NmdFileBoneTree<A>> {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<NmdFileBoneTree<A>> {
        &mut self.children
    }

    fn data_mut_opt(&mut self) -> Option<&mut A> {
        None
    }

    fn data_opt(&self) -> Option<&A> {
        None
    }

    fn id(&self) -> u16 {
        NmdFileBone::ROOT_BONE_ID
    }

    fn iter(&self) -> NmdFileBoneTreeIterator<A> {
        self.into_iter()
    }

    fn mut_opt(&mut self) -> Option<&mut NmdFileBoneTree<A>> {
        None
    }

    fn opt(&self) -> Option<&NmdFileBoneTree<A>> {
        None
    }

    fn parent_id(&self) -> u16 {
        NmdFileBone::ROOT_BONE_ID
    }

    fn set_id(&mut self, _: u16) {}
    fn set_parent_id(&mut self, _: u16) {}
}

impl<'a, A> IntoIterator for &'a NmdFileBoneTreeRoot<A> {
    type Item = &'a NmdFileBoneTree<A>;
    type IntoIter = NmdFileBoneTreeIterator<'a, A>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            stack: self.children.iter().collect(),
        }
    }
}

impl<A> NmdFileBoneTree<A> {
    fn new(bone_data: &NmdFileBone, associated_data: A) -> Self {
        Self {
            data: NmdFileBoneTreeData {
                associated_data: associated_data,
                id: bone_data.id,
                parent_id: bone_data.parent_id,
            },
            children: vec!(),
        }
    }

    pub fn transformed(&self, recursive: bool, used_id_set: &OrderedSet<u16>, mut clone_fn: impl FnMut((u16, u16), &A) -> Option<A>) -> Option<Self> {
        let mut id_generator = NmdFileBoneTreeIdGenerator::new(used_id_set);

        if recursive {
            self.transformed_recursive(&mut id_generator, &mut clone_fn)
        } else {
            self.transformed_shallow(&mut id_generator, &mut clone_fn)
        }
    }

    fn transformed_recursive(&self, id_generator: &mut NmdFileBoneTreeIdGenerator, clone_fn: &mut impl FnMut((u16, u16), &A) -> Option<A>) -> Option<Self> {
        let mut clone = self.transformed_shallow(id_generator, clone_fn)?;

        for child in &self.children {
            if let Some(child_clone) = child.transformed_recursive(id_generator, clone_fn) {
                clone.children.push(child_clone);
            }
        }

        Some(clone)
    }

    fn transformed_shallow(&self, id_generator: &mut NmdFileBoneTreeIdGenerator, clone_fn: &mut impl FnMut((u16, u16), &A) -> Option<A>) -> Option<Self> {
        let new_id = id_generator.next()?;

        if let Some(associated_data) = clone_fn((self.id(), new_id), self.data()) {
            Some(Self {
                data: NmdFileBoneTreeData {
                    associated_data: associated_data,
                    id: new_id,
                    parent_id: self.data.parent_id,
                },
                children: vec!(),
            })
        } else {
            id_generator.back_one();

            None
        }
    }

    #[inline(always)]
    pub fn data(&self) -> &A {
        &self.data.associated_data
    }

    #[inline(always)]
    pub fn data_mut(&mut self) -> &mut A {
        &mut self.data.associated_data
    }
}

impl<A> NmdFileBoneTreeNode for NmdFileBoneTree<A> {
    type Data = A;

    fn children(&self) -> &Vec<Self> {
        &self.children
    }

    fn children_mut(&mut self) -> &mut Vec<Self> {
        &mut self.children
    }

    #[inline(always)]
    fn data_mut_opt(&mut self) -> Option<&mut A> {
        Some(self.data_mut())
    }

    #[inline(always)]
    fn data_opt(&self) -> Option<&A> {
        Some(self.data())
    }

    fn id(&self) -> u16 {
        self.data.id
    }

    fn iter(&self) -> NmdFileBoneTreeIterator<A> {
        self.into_iter()
    }

    fn mut_opt(&mut self) -> Option<&mut Self> {
        Some(self)
    }

    fn opt(&self) -> Option<&Self> {
        Some(self)
    }

    fn parent_id(&self) -> u16 {
        self.data.parent_id
    }

    fn set_id(&mut self, id: u16) {
        self.data.id = id;
    }

    fn set_parent_id(&mut self, parent_id: u16) {
        self.data.parent_id = parent_id;
    }
}

impl<'a, A> IntoIterator for &'a NmdFileBoneTree<A> {
    type Item = &'a NmdFileBoneTree<A>;
    type IntoIter = NmdFileBoneTreeIterator<'a, A>;

    fn into_iter(self) -> Self::IntoIter {
        Self::IntoIter {
            stack: VecDeque::from([self]),
        }
    }
}

impl<'a, A> Iterator for NmdFileBoneTreeIterator<'a, A> {
    type Item = &'a NmdFileBoneTree<A>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stack.pop_front() {
            Some(tree) => {
                self.stack.extend(&tree.children);

                Some(tree)
            },
            None => None
        }
    }
}

impl NmdFileBoneTreeIdGenerator {
    fn new(used_id_set: &OrderedSet<u16>) -> Self {
        let mut used_ids = used_id_set.clone().into_iter();
        let (ids, used_id_echo) = match used_ids.next() {
            Some(0) => {
                let mut ids = 0..=0;
                
                ids.next();

                (ids, 0)
            }
            Some(used_id) => {
                (0..=used_id - 1, used_id)
            }
            None => {
                (0..=u16::MAX - 1, u16::MAX)
            }
        };

        Self {
            ids: ids,
            used_ids: used_ids,
            used_id_echo: used_id_echo,
        }
    }

    fn back_one(&mut self) {
        self.ids = (match self.ids.next() {
            Some(id) => id.max(1) - 1,
            None => *self.ids.end(),
        }..=*self.ids.end())
    }
}

impl Iterator for NmdFileBoneTreeIdGenerator {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        match self.ids.next() {
            None => loop {
                match self.used_ids.next() {
                    Some(used_id) => {
                        if used_id - self.used_id_echo > 1 {
                            self.ids = self.used_id_echo + 1..=used_id - 1;
                            self.used_id_echo = used_id;

                            break self.next();
                        } else {
                            self.used_id_echo = used_id;
                        }
                    }
                    None => {
                        self.ids = self.used_id_echo.checked_add(1)?..=u16::MAX - 1;
                        self.used_id_echo = u16::MAX;

                        break self.next();
                    }
                }
            }
            some_id => some_id
        }
    }
}

fn for_path_mut_internal<T, F: FnMut(&mut NmdFileBoneTree<T::Data>)>
(
    subtree: &mut T,
    path: &VecDeque<u16>,
    index: usize,
    routine: &mut F
)
    where T: NmdFileBoneTreeNode,
{
    if let Some(child_id) = path.get(index) {
        for child in subtree.children_mut() {
            if child.id() == *child_id {
                routine(child);

                for_path_mut_internal(child, path, index + 1, routine);

                break;
            }
        }
    }
}

fn path_to_internal<'a, T>
(
    subtree: &'a T,
    id: u16,
    path: &mut VecDeque<u16>
) -> Option<&'a dyn NmdFileBoneTreeNode<Data = T::Data>>
    where T: NmdFileBoneTreeNode,
{
    if subtree.id() == id {
        Some(subtree)
    } else {
        for child in subtree.children() {
            let child_id = child.id();

            if let target @ Some(_) = path_to_internal(child, id, path) {
                path.push_front(child_id);

                return target;
            }
        }

        None
    }
}

fn path_to_mut_internal<'a, T>
(
    subtree: &'a mut T,
    id: u16,
    path: &mut VecDeque<u16>
) -> Option<&'a mut dyn NmdFileBoneTreeNode<Data = T::Data>>
    where T: NmdFileBoneTreeNode,
{
    if subtree.id() == id {
        Some(subtree)
    } else {
        for child in subtree.children_mut() {
            let child_id = child.id();

            if let target @ Some(_) = path_to_mut_internal(child, id, path) {
                path.push_front(child_id);

                return target;
            }
        }

        None
    }
}

fn path_to_parent_mut_internal<'a, T>
(
    subtree: &'a mut T,
    id: u16,
    path: &mut VecDeque<u16>
) -> Option<&'a mut dyn NmdFileBoneTreeNode<Data = T::Data>>
    where T: NmdFileBoneTreeNode,
{
    if subtree.children().iter().any(|child| child.id() == id) {
        Some(subtree)
    } else {
        for child in subtree.children_mut() {
            let child_id = child.id();

            if let target @ Some(_) = path_to_parent_mut_internal(child, id, path) {
                path.push_front(child_id);

                return target;
            }
        }

        None
    }
}

