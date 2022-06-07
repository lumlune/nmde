use {
    crate::io::nmd::{
        anatomy::{
            NmdFileBone,
            NmdFileHeader,
        },
        data::tree::*,
        data::{ByteArr, ByteVec},
        NmdFileReader,
    },
    std::{
        collections::BTreeMap,
        io::Error,
        io::Result,
    },
    serde::{
        Deserialize,
        Serialize,
    },
};

#[derive(Serialize, Deserialize)]
pub struct NmdFileData {
    pub header: NmdFileHeader,
    pub bones: BTreeMap<u16, NmdFileBone>,
    bytes: NmdFileRawData,
}

#[derive(Serialize, Deserialize)]
struct NmdFileRawData {
    header: ByteVec,
    physics: ByteVec,
    blob: ByteVec,
}

impl NmdFileData {
    pub fn get(&self, bone_id: u16) -> Option<&NmdFileBone> {
        self.bones.get(&bone_id)
    }

    pub fn get_clone(&self, bone_id: u16) -> Option<NmdFileBone> {
        self.bones.get(&bone_id).map(|bone_data| bone_data.clone())
    }

    pub fn raw_blob(&self) -> &ByteVec {
        &self.bytes.blob
    }

    pub fn raw_header(&self) -> ByteArr<'_> {
        &self.bytes.header[..]
    }

    pub fn raw_physics_data(&self) -> &ByteVec {
        &self.bytes.physics
    }

    pub fn set(&mut self, bone_id: u16, bone_data: NmdFileBone) {
        self.bones.insert(bone_id, bone_data);
    }

    pub fn tree_with<A>(&self, assoc_fn: impl Fn(&NmdFileBone) -> A) -> NmdFileBoneTreeRoot<A> {
        NmdFileBoneTreeRoot::new_with(self.bones.values(), assoc_fn)
    }
}

impl TryFrom<&mut NmdFileReader> for NmdFileData {
    type Error = Error;

    fn try_from(reader: &mut NmdFileReader) -> Result<Self> {
        Ok(Self {
            bones: reader.read_bones()?,
            header: reader.header().to_owned(),
            bytes: NmdFileRawData::try_from(reader)?,
        })
    }
}

impl TryFrom<&mut NmdFileReader> for NmdFileRawData {
    type Error = Error;

    fn try_from(reader: &mut NmdFileReader) -> Result<Self> {
        Ok(Self {
            header: reader.read_header_bytes()?,
            physics: reader.read_physics_bytes()?,
            blob: reader.read_blob_bytes()?,
        })
    }
}

