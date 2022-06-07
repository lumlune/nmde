use {
    crate::io::nmd::{
        anatomy::{
            token::*,
            NmdFileAddress,
            NmdFileBone,
            NmdFileBoneFlag,
            NmdFileHeader,
        },
        data::NmdFileData,
    },
    std::{
        collections::BTreeMap,
        io::{
            self,
            Error,
            ErrorKind,
            Read,
            Result,
            Seek,
            SeekFrom,
            Write,
        },
        fs::File,
        path::PathBuf,
        mem,
    },
};

/*
 * TODO:
 * ~ Test writing an outrageous number of bones. Error handling not robust
 */

macro_rules! delta {
    ($value:expr, $type:ty, $delta:expr) => {
        (($value as isize) + $delta) as $type
    }
}

macro_rules! write {
    ($writer:ident, $data:expr, $type:ty) => {
        $writer.write::<$type, { mem::size_of::<$type>() }>(&$data)
    };

    ($writer:ident, $data:expr, $type:ty, 'little_endian) => {
        $writer.write::<$type, { mem::size_of::<$type>() }>(&$data.to_le())
    };

    ($writer:ident, $data:expr, $type:ty, 'little_endian_f32) => {
        // Could also use `to_le_bytes()` but have to modify caller
        $writer.write::<$type, { mem::size_of::<$type>() }>(&$data.to_bits().to_le())
    };
}

macro_rules! write_at {
    ($writer:ident, $data:expr, $type:ty, $token:path $(, $endian_switch:tt)?) => {
        {
            $writer.seek_token($token)?;

            write!($writer, $data, $type $(, $endian_switch)?)
        }
    }
}

type BoneIterable<'a> = Vec<&'a NmdFileBone>;

pub struct NmdFileWriter {
    file: File,
}

impl NmdFileWriter {
    fn bones_sorted<'a>(new_bones: &'a BTreeMap<u16, NmdFileBone>) -> Vec<&'a NmdFileBone> {
        let mut bones: Vec<_> = new_bones.values().collect();

        bones.sort_by_key(|bone_data| bone_data.id);
        bones
    }

    fn error(error_message: &str) -> Result<()> {
        Err(Error::new(ErrorKind::Other, error_message))
    }

    fn file_delta(data: &NmdFileData, new_bones: &BTreeMap<u16, NmdFileBone>) -> (isize, isize) {
        let bone_delta = new_bones.len() as isize - data.header.bone_count as isize;

        (bone_delta, (NmdFileBone::CHUNK_SIZE as isize) * bone_delta)
    }

    fn nth_bone_address(n: usize) -> u64 {
        NmdFileHeader::CHUNK_SIZE + ((n as u64) * NmdFileBone::CHUNK_SIZE)
    }

    fn physics_data_address(bone_data: &NmdFileBone, byte_delta: isize) -> NmdFileAddress {
        if bone_data.is_phys() {
            delta!(bone_data.physics_data_address, NmdFileAddress, byte_delta)
        } else {
            // We don't offset, just preserve the original data - which might be
            // garbage, or might mean something else
            bone_data.physics_data_address
        }
    }

    fn seek(&mut self, address: u64) -> Result<u64> {
        self.file.seek(SeekFrom::Start(address))
    }

    fn seek_token(&mut self, token: NmdFileToken) -> Result<u64> {
        let NmdFileTokenValue {
            is_relative,
            offset,
            ..
        } = token.value();

        self.file.seek(match is_relative {
            true  => SeekFrom::Current(offset as i64),
            false => SeekFrom::Start(offset as u64),
        })
    }

    fn write<T, const T_SIZE: usize>(&mut self, data: &T) -> Result<()> {
        unsafe {
            let bytes: &mut [u8; T_SIZE] = mem::transmute_copy(&data);

            self.file.write(bytes)?;
        }

        Ok(())
    }

    pub fn write_new(mut self, data: &NmdFileData, new_bones: &BTreeMap<u16, NmdFileBone>) -> Result<()> {
        let bone_data_sorted = Self::bones_sorted(new_bones);
        let file_delta @ (_, byte_delta) = Self::file_delta(data, new_bones);

        self.file.write(data.raw_header())?;
        self.write_header_revisions(&data.header, file_delta)?;
        self.write_bone_data(&bone_data_sorted, byte_delta)?;
        self.file.write(data.raw_physics_data())?;
        self.file.write(data.raw_blob())?;
        self.write_bone_name_data(&bone_data_sorted)?;

        self.file.sync_all()?;

        Ok(())
    }

    fn write_bone_data(&mut self, bone_data: &Vec<&NmdFileBone>, byte_delta: isize) -> Result<()> {
        let stream_start_position = self.file.stream_position()?;
        let stream_final_position;

        let mut i = 0;

        // All numerical values are written as little endian, but `f32` values
        // are first cast to bits (`u32` value with same bytes).
        for bone_data in bone_data {
            write!(self, bone_data.collision_data, [u8; 16])?;
            write!(self, bone_data.translation_x, u32, 'little_endian_f32)?;
            write!(self, bone_data.translation_y, u32, 'little_endian_f32)?;
            write!(self, bone_data.translation_z, u32, 'little_endian_f32)?;
            write!(self, bone_data.unknown_data_a, [u8; 4])?;
            write!(self, bone_data.rotation_x, u32, 'little_endian_f32)?;
            write!(self, bone_data.rotation_y, u32, 'little_endian_f32)?;
            write!(self, bone_data.rotation_z, u32, 'little_endian_f32)?;
            write!(self, bone_data.unknown_data_b, [u8; 4])?;
            write!(self, 0x0, NmdFileAddress)?; // Later becomes name address
            write!(self, bone_data.unknown_data_c, [u8; 4])?;
            write!(self, Self::physics_data_address(bone_data, byte_delta), NmdFileAddress, 'little_endian);
            write!(self, bone_data.unknown_data_d, [u8; 4])?;
            write!(self, bone_data.translation_x_next, u32, 'little_endian_f32)?;
            write!(self, bone_data.gravity_x, i16, 'little_endian)?;
            write!(self, bone_data.gravity_y, i16, 'little_endian)?;
            write!(self, bone_data.physics_constraint_x_max, i8)?;
            write!(self, bone_data.physics_constraint_x_min, i8)?;
            write!(self, bone_data.physics_constraint_y_max, i8)?;
            write!(self, bone_data.physics_constraint_y_min, i8)?;
            write!(self, bone_data.unknown_data_e, [u8; 19])?;
            write!(self, u8::from(bone_data.flag), u8)?;
            write!(self, bone_data.parent_id, u16, 'little_endian)?;
            write!(self, bone_data.id, u16, 'little_endian)?;
            write!(self, bone_data.unknown_data_f, [u8; 12])?;

            i += 1;
        }

        stream_final_position = self.file.stream_position()?;

        if (stream_final_position - stream_start_position) == (i * NmdFileBone::CHUNK_SIZE) {
            Ok(()) 
        } else {
            Self::error("[NmdFileWriter::write_bone_data] Wrote an unexpected number of bytes")
        }
    }

    fn write_bone_name_data(&mut self, bone_data: &Vec<&NmdFileBone>) -> Result<()> {
        const BYTE_SHIFT: u8 = NmdFileBone::ASCII_BYTE_SHIFT;

        for (i, NmdFileBone { name, .. }) in bone_data.into_iter().enumerate() {
            let name_address = self.file.stream_position()?;

            self.write_nth_bone_name_address(i, name_address)?;

            self.seek(name_address)?;
            self.file.write(&name.as_bytes().iter().map(|b| b + BYTE_SHIFT).collect::<Vec<_>>()[..])?;
            self.file.write(&[BYTE_SHIFT])?;
        }

        Ok(())
    }

    fn write_header_revisions(&mut self, header: &NmdFileHeader, (bone_delta, byte_delta): (isize, isize)) -> Result<()>
    {
        use NmdFileToken::*;

        let NmdFileHeader {
            bone_count,
            bone_name_data_address,
            blob_data_address,
            ..
        } = header;

        let new_bone_count             = delta!(*bone_count,             u16,            bone_delta);
        let new_bone_name_data_address = delta!(*bone_name_data_address, NmdFileAddress, byte_delta);
        let new_blob_data_address      = delta!(*blob_data_address,      NmdFileAddress, byte_delta);

        write_at!(self, new_bone_count,             u16,            HeaderBoneCount,           'little_endian)?;
        write_at!(self, new_bone_name_data_address, NmdFileAddress, HeaderBoneNameDataAddress, 'little_endian)?;
        write_at!(self, new_blob_data_address,      NmdFileAddress, HeaderBlobDataAddress,     'little_endian)?;
        write_at!(self, new_bone_count,             u16,            HeaderBoneCountEcho,       'little_endian)?;

        self.seek(NmdFileHeader::CHUNK_SIZE)?;

        Ok(())
    }

    fn write_nth_bone_name_address(&mut self, n: usize, name_address: u64) -> Result<()> {
        let bone_address = Self::nth_bone_address(n);

        self.seek(bone_address);
        self.seek_token(NmdFileToken::BoneNameAddress);

        match NmdFileAddress::try_from(name_address) {
            Ok(name_address) => {
                write!(self, name_address, NmdFileAddress, 'little_endian)?;

                Ok(())
            }
            _ => Self::error("[NmdFileWriter::write_nth_bone_name_address] Name address out of bounds")
        }
    }
}

impl TryFrom<&PathBuf> for NmdFileWriter {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self> {
        Ok(Self {
            file: File::create(path)?,
        })
    }
}
