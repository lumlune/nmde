use {
    crate::io::nmd::anatomy::{
        NmdFileAddress,
        NmdFileBoneFlag,
    },
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NmdFileBone {
    pub collision_data: [u8; 16],
    pub translation_x: f32,
    pub translation_y: f32,
    pub translation_z: f32,
    pub unknown_data_a: [u8; 4],
    pub rotation_x: f32,
    pub rotation_y: f32,
    pub rotation_z: f32,
    pub unknown_data_b: [u8; 4],
    pub name: String,
    pub unknown_data_c: [u8; 4],
    pub physics_data_address: NmdFileAddress,
    pub unknown_data_d: [u8; 4],
    pub translation_x_next: f32,
    pub gravity_x: i16,
    pub gravity_y: i16,
    pub physics_constraint_x_max: i8,
    pub physics_constraint_x_min: i8,
    pub physics_constraint_y_max: i8,
    pub physics_constraint_y_min: i8,
    pub unknown_data_e: [u8; 19],
    pub flag: NmdFileBoneFlag,
    pub parent_id: u16,
    pub id: u16,
    pub unknown_data_f: [u8; 12],
}

impl NmdFileBone {
    pub const ASCII_BYTE_SHIFT: u8 = 0x40;
    pub const CHUNK_SIZE: u64 = 0x70;
    pub const ROOT_BONE_ID: u16 = 0xFFFF;

    pub fn is_phys(&self) -> bool {
        self.flag.is_phys()
    }
}

