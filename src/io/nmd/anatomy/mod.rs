pub mod token;

mod bone_data;
mod bone_flag;
mod header_data;

/// Maximum `u32` in header, maybe `u64` in bones.
/// Minimum `u16` but unlikely.
/// Don't change lightly; there will be data chunks unaccounted for.
pub type NmdFileAddress = u32;

pub use {
    bone_data::NmdFileBone,
    bone_flag::NmdFileBoneFlag,
    bone_flag::NmdFileBoneFlagIterator,
    header_data::NmdFileHeader,
};
