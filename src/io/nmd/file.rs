use {
    crate::io::nmd::{
        data::NmdFileData,
        NmdFileReader,
        NmdFileWriter,
    },
    std::{
        io::Error,
        io::Result,
        path::PathBuf,
    },
};

/*
 * TODO
 * ~ Rethink with Rc; persistent reader not needed (?), file as whole should be
 * immutable.  The byte vector can be a field under data.
 */

pub struct NmdFile {
    pub data: NmdFileData,
    path: PathBuf,
}

impl NmdFile {
    // pub fn save(&mut self) -> Result<()> {
    //     self.save_as(&self.path.to_owned())?;

    //     Ok(())
    // }

    // pub fn save_as(&mut self, path: &PathBuf) -> Result<()> {
    //     let writer = NmdFileWriter::try_new(&path, &mut self.reader)?;

    //     writer.write_all(self.data.bone_map.values())?;

    //     Ok(())
    // }
}

impl TryFrom<&PathBuf> for NmdFile {
    type Error = Error;

    fn try_from(path: &PathBuf) -> Result<Self> {
        let mut reader = NmdFileReader::try_from(path)?;

        Ok(Self {
            data: NmdFileData::try_from(&mut reader)?,
            path: path.to_owned(),
        })
    }
}

