//! The `.sn4` name file, which stores the player, event, site, and round
//! names that index entries refer to by ID.

use std::fs::File;
use std::io::{BufReader, BufWriter, Write};
use std::path::Path;

use crate::error::{DatabaseFile, Result};
use crate::namebase::NameBase;

/// A `.sn4` name file: the [`NameBase`] held in memory and the path it is
/// written to.
pub struct NFile {
    namebase: NameBase,
    path: std::path::PathBuf,
}

impl NFile {
    /// Starts an empty name base for the database at `path`, given without an
    /// extension. Nothing is written until [`close`](Self::close).
    ///
    /// # Errors
    ///
    /// This doesn't fail at present; it returns a [`Result`] so that it can.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let sn4_path = path.with_extension("sn4");
        Ok(NFile {
            namebase: NameBase::new(),
            path: sn4_path,
        })
    }

    /// Reads the name file of the database at `path`, given without an
    /// extension.
    ///
    /// # Errors
    ///
    /// Returns [`Error::File`](crate::Error::File) for
    /// [`DatabaseFile::Name`] if the file can't be read or is damaged.
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

    /// Returns the names.
    pub fn namebase(&self) -> &NameBase {
        &self.namebase
    }

    /// Returns the names for changing.
    pub fn namebase_mut(&mut self) -> &mut NameBase {
        &mut self.namebase
    }

    /// Writes the names to the `.sn4` file, replacing it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if the file can't be written.
    pub fn close(&mut self) -> Result<()> {
        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);
        self.namebase.write_sn4(&mut writer)?;
        writer.flush()?;
        Ok(())
    }
}
