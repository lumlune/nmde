use {
    crate::io::nmd::{
        anatomy::{
            token::{
                NmdFileToken,
                NmdFileTokenValue,
            },
            NmdFileAddress,
            NmdFileBone,
            NmdFileBoneFlag,
            NmdFileHeader,
        },
        data::ByteVec,
    },
    std::{
        collections::BTreeMap,
        cmp,
        io::{
            Cursor,
            Error,
            ErrorKind,
            Read,
            Result,
            Seek,
            SeekFrom,
        },
        mem,
        path::PathBuf,
        iter::from_fn as iter_fn,
        fs,
    },
};

/*
 * TODO:
 * ~ Handle duplicate IDs & orphans (TODO FEAT:LOST_DATA)
 *
 *      Example:
 *
 *          hair_r006_f_H_Hair.nmd
 *
 *          (Duplicate)
 *          KOSHI                           ID: 0x0C    PARENT_ID: 0xFFFF
 *          ` Shares an ID with bone "shoulder_r__shit"
 *
 *          (Orphan)
 *          KATA_L                          ID: 0x09    PARENT_ID: 0x06
 *          ` Has a parent ID not corresponding to any bone
 *
 *          (Duplicate/Orphan)
 *          KATA_RT_L__prot_x0__offset      ID: 0x09    PARENT_ID: 0x07
 *
 *          (Orphan)
 *          shoulder_l_shit                 ID: 0x0A    PARENT_ID: 0x09
 *          ` Orphan with ambiguous parent (which would also be an orphan)
 *
 *      (Intuitively, KATA_L should be under SAKOTSU_L (ID: 0x08), and the next
 *      two bones should follow - same pattern as SAKOTSU_R.)
 */

macro_rules! read {
    ($reader:expr, $type:ty) => {
        $reader.read::<$type, { mem::size_of::<$type>() }>()
    };
}

macro_rules! read_at {
    ($reader:expr, $type:ty, $token:path) => {
        {
            $reader.seek_token($token)?;

            read!($reader, $type)
        }
    };
}

macro_rules! read_rewind {
    ($reader:ident, $type:ty, $token:path) => {
        {
            let rewind_address = $reader.cursor.stream_position()?;
            let read_result = read_at!($reader, $type, $token);

            $reader.seek(rewind_address)?;

            read_result
        }
    };
}

macro_rules! skip_then {
    ($reader:ident, $type:ty, $then_expr:expr) => {
        {
            $reader.seek_rel(mem::size_of::<$type>() as i64)?;
            $then_expr
        }
    };
}

type ByteCursor = Cursor<Vec<u8>>;

#[derive(Debug)]
pub struct NmdFileReader {
    cached_header: Option<NmdFileHeader>,
    cursor: ByteCursor,
}

impl NmdFileReader {
    fn anchor_cursor<F: FnOnce(&mut ByteCursor) -> T, T>(cursor: &mut ByteCursor, callback: F) -> Result<T> {
        let resume_address = cursor.stream_position()?;
        let value = callback(cursor);

        cursor.seek(SeekFrom::Start(resume_address))?;

        Ok(value)
    }

    fn blob_metadata(&mut self) -> (u64, usize) {
        (self.header().blob_data_address as u64, self.header().blob_data_length())
    }

    fn bone_metadata(&mut self) -> (u64, usize) {
        (NmdFileHeader::CHUNK_SIZE, self.header().bone_count as usize)
    }

    pub fn header(&self) -> &NmdFileHeader {
        self.cached_header.as_ref().unwrap()
    }

    fn iter_bones<'s>(&'s mut self) -> Result<impl IntoIterator<Item = NmdFileBone> + 's> {
        let (mut address, mut count) = self.bone_metadata();

        iter_fn(|| {
            if count > 0 {
                match self.read_bone_at(address) {
                    Ok(bone_data) => {
                        address += NmdFileBone::CHUNK_SIZE;
                        count -= 1;

                        Some(Ok(bone_data))
                    }
                    Err(error) => Some(Err(error))
                }
            } else {
                None
            }
        }).collect::<Result<Vec<_>>>()
    }

    fn physics_metadata(&self) -> (u64, usize) {
        (self.header().physics_data_address() as u64, self.header().physics_data_length())
    }

    fn read<T: Sized, const T_SIZE: usize>(&mut self) -> Result<T> {
        unsafe {
            let mut bytes = [0u8; T_SIZE];

            self.cursor.read_exact(&mut bytes)?;

            Ok(mem::transmute_copy(&bytes))
        }
    }

    pub fn read_blob_bytes(&mut self) -> Result<ByteVec> {
        let (address, length) = self.blob_metadata();

        self.read_bytes(address, length)
    }

    fn read_bytes(&mut self, address: u64, count: usize) -> Result<ByteVec> {
        let mut bytes = Vec::<u8>::new();

        self.seek(address)?;
        self.cursor.by_ref().take(count as u64).read_to_end(&mut bytes)?;

        Ok(bytes)
    }

    pub fn read_bones(&mut self) -> Result<BTreeMap<u16, NmdFileBone>> {
        let mut bone_map = BTreeMap::<u16, NmdFileBone>::new();

        for bone_data in self.iter_bones()? {
            // TODO FEAT:LOST_DATA
            bone_map.insert(bone_data.id, bone_data);
        }

        Ok(bone_map)
    }

    fn read_bone_at(&mut self, address: u64) -> Result<NmdFileBone> {
        self.seek(address)?;

        Ok(self.read_bone_at_cursor()?)
    }

    fn read_bone_at_cursor(&mut self) -> Result<NmdFileBone> {
        use NmdFileToken::*;

        let name_address = read_rewind!(self, NmdFileAddress, BoneNameAddress)?;

        Ok(NmdFileBone {
            collision_data: read!(self, [u8; 16])?,
            translation_x: read!(self, f32)?,
            translation_y: read!(self, f32)?,
            translation_z: read!(self, f32)?,
            unknown_data_a: read!(self, [u8; 4])?,
            rotation_x: read!(self, f32)?,
            rotation_y: read!(self, f32)?,
            rotation_z: read!(self, f32)?,
            unknown_data_b: read!(self, [u8; 4])?,
            name: self.read_bone_name(name_address as u64)?,
            unknown_data_c: skip_then!(self, NmdFileAddress, read!(self, [u8; 4])?),
            physics_data_address: read!(self, NmdFileAddress)?,
            unknown_data_d: read!(self, [u8; 4])?,
            translation_x_next: read!(self, f32)?,
            gravity_x: read!(self, i16)?,
            gravity_y: read!(self, i16)?,
            physics_constraint_x_max: read!(self, i8)?,
            physics_constraint_x_min: read!(self, i8)?,
            physics_constraint_y_max: read!(self, i8)?,
            physics_constraint_y_min: read!(self, i8)?,
            unknown_data_e: read!(self, [u8; 19])?,
            flag: NmdFileBoneFlag::from(read!(self, u8)?),
            parent_id: read!(self, u16)?,
            id: read!(self, u16)?,
            unknown_data_f: read!(self, [u8; 12])?,
        })
    }

    fn read_bone_name(&mut self, name_address: u64) -> Result<String> {
        let mut name = String::from("");

        Self::anchor_cursor(&mut self.cursor, |cursor| {
            cursor.seek(SeekFrom::Start(name_address))?;

            for byte in cursor.bytes() {
                let character = Self::to_ascii(byte?);

                if character != '\0' {
                    name.push(character);
                } else {
                    break;
                }
            }

            Ok::<_, Error>(())
        })??;

        Ok(name)
    }

    // Use `header()` instead of this method for outside calls
    fn read_header(&mut self) -> Result<NmdFileHeader> {
        use NmdFileToken::*;

        Ok(NmdFileHeader {
            bone_count: read_at!(self, u16, HeaderBoneCount)?,
            blob_data_address: read_at!(self, NmdFileAddress, HeaderBlobDataAddress)?,
            bone_name_data_address: read_at!(self, NmdFileAddress, HeaderBoneNameDataAddress)?,
            bone_data_address: read_at!(self, NmdFileAddress, HeaderBoneDataAddress)?,
        })
    }

    pub fn read_header_bytes(&mut self) -> Result<ByteVec> {
        self.read_bytes(0, NmdFileHeader::CHUNK_SIZE as usize)
    }

    pub fn read_physics_bytes(&mut self) -> Result<ByteVec> {
        let (address, length) = self.physics_metadata();

        self.read_bytes(address, length)
    }

    pub fn seek(&mut self, address: u64) -> Result<u64> {
        self.cursor.seek(SeekFrom::Start(address))
    }

    pub fn seek_rel(&mut self, offset: i64) -> Result<u64> {
        self.cursor.seek(SeekFrom::Current(offset))
    }

    pub fn seek_token(&mut self, token: NmdFileToken) -> Result<u64> {
        let NmdFileTokenValue {
            is_relative,
            offset,
            ..
        } = token.value();

        self.cursor.seek(match is_relative {
            true  => SeekFrom::Current(offset as i64),
            false => SeekFrom::Start(offset as u64),
        })
    }

    fn to_ascii(byte: u8) -> char {
        char::from(cmp::max(NmdFileBone::ASCII_BYTE_SHIFT, byte) - NmdFileBone::ASCII_BYTE_SHIFT)
    }
}

impl TryFrom<&PathBuf> for NmdFileReader {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self> {
        let mut reader = Self {
            cached_header: None,
            cursor: Cursor::new(fs::read(path)?),
        };

        let header = reader.read_header()?;

        // `Ok` only if computed sizes will not panic
        if header.ordinal() {
            reader.cached_header = Some(header);
            Ok(reader)
        } else {
            Err(Error::new(ErrorKind::Other, "Non-ordinal header"))
        }
    }
}
