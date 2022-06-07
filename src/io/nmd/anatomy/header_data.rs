use crate::io::nmd::anatomy::NmdFileAddress;
use crate::io::nmd::anatomy::NmdFileBone;
use serde::{Deserialize, Serialize};

/*
 * TODO:
 * ~ Write test for rel. type size check, e.g. expect:
 *      u16 < NmdFileAddress < u64
 *      etc.
 * (can put elsewhere like mod.rs)
 */

#[derive(Debug, Clone, Serialize, Deserialize)]
// Formal header properties only i.e. in-file
pub struct NmdFileHeader {
    pub bone_count: u16,
    pub blob_data_address: NmdFileAddress,
    pub bone_name_data_address: NmdFileAddress,
    pub bone_data_address: NmdFileAddress,
}

impl NmdFileHeader {
    pub const CHUNK_SIZE: u64 = 0x20;

    pub fn blob_data_length(&self) -> usize {
        self.blob_data_length_opt().unwrap()
    }

    fn blob_data_length_opt(&self) -> Option<usize> {
        (self.bone_name_data_address as usize)
            .checked_sub(self.blob_data_address as usize)
    }

    #[allow(unused)]
    pub fn bone_data_length(&self) -> usize {
        self.bone_data_length_opt().unwrap()
    }

    fn bone_data_length_opt(&self) -> Option<usize> {
        (self.bone_count as usize)
            .checked_mul(NmdFileBone::CHUNK_SIZE as usize)
    }

    // This is a useful check as it ensures no panic on address operations
    pub fn ordinal(&self) -> bool {
        self.blob_data_length_opt().is_some()
         && self.bone_data_length_opt().is_some()
         && self.physics_data_address_opt().is_some()
         && self.physics_data_length_opt().is_some()
    }

    pub fn physics_data_address(&self) -> NmdFileAddress {
        self.physics_data_address_opt().unwrap()
    }

    pub fn physics_data_address_opt(&self) -> Option<NmdFileAddress> {
        self.bone_data_address
            .checked_add(
                self.bone_data_length_opt()?
                    .try_into().ok()?)
    }

    pub fn physics_data_length(&self) -> usize {
        self.physics_data_length_opt().unwrap()
    }

    pub fn physics_data_length_opt(&self) -> Option<usize> {
        (self.blob_data_address as usize)
            .checked_sub(self.physics_data_address_opt()? as usize)
    }
}
