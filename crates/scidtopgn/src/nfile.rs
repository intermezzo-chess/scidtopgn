use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use crate::error::{DatabaseFile, Result};
use crate::namebase::NameBase;

pub struct NFile {
    namebase: NameBase,
    path: std::path::PathBuf,
}

impl NFile {
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let sn4_path = path.with_extension("sn4");
        Ok(NFile {
            namebase: NameBase::new(),
            path: sn4_path,
        })
    }

    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let sn4_path = path.with_extension("sn4");
        let read = || -> Result<NameBase> {
            let mut reader = BufReader::new(File::open(&sn4_path)?);
            let mut namebase = NameBase::new();
            namebase.read_sn4(&mut reader)?;
            Ok(namebase)
        };
        let namebase = read().map_err(|e| e.in_file(DatabaseFile::Name))?;

        Ok(NFile {
            namebase,
            path: sn4_path,
        })
    }

    pub fn namebase(&self) -> &NameBase {
        &self.namebase
    }

    pub fn namebase_mut(&mut self) -> &mut NameBase {
        &mut self.namebase
    }

    pub fn close(&mut self) -> Result<()> {
        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);
        self.namebase.write_sn4(&mut writer)?;
        writer.flush()?;
        Ok(())
    }
}
