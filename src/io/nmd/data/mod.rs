mod bone_tree;
mod file_data;

pub(in crate) type ByteArr<'a> = &'a [u8];
pub(in crate) type ByteVec = Vec<u8>;

pub mod tree {
    pub use {
        super::bone_tree::{
            NmdFileBoneTree,
            NmdFileBoneTreeIterator,
            NmdFileBoneTreeNode,
            NmdFileBoneTreeRoot,
        }
    };
}

pub use {
    file_data::NmdFileData,
};
