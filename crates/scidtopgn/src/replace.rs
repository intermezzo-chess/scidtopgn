//! [`NewFiles`], which replaces some of a database's files with new ones.

use std::fs::{self, File};
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::error::Result;

/// New files for the database at a path, written in a temporary directory
/// beside it until [`replace`](Self::replace) puts them in place of the
/// database's files. Dropping it removes the directory and the files in it.
pub(crate) struct NewFiles {
    /// The database's path, without an extension.
    path: PathBuf,
    dir: TempDir,
    /// The extensions of the files created, in the order they replace the
    /// database's files.
    extensions: Vec<&'static str>,
}

impl NewFiles {
    /// Makes a temporary directory for new files beside the database at
    /// `path`, given without an extension, so that they can be renamed onto
    /// its files. The directory's name ends with random characters, so it
    /// can't collide with another.
    pub(crate) fn new(path: &Path) -> Result<Self> {
        let parent = match path.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => parent,
            _ => Path::new("."),
        };
        let dir = tempfile::Builder::new()
            .prefix(".scidtopgn-compact-")
            .tempdir_in(parent)?;
        Ok(Self {
            path: path.to_path_buf(),
            dir,
            extensions: Vec::new(),
        })
    }

    /// Creates the new file that replaces the database's file with extension
    /// `ext`, open for reading and writing.
    pub(crate) fn create(&mut self, ext: &'static str) -> Result<File> {
        let file = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(self.new_path(ext))?;
        self.extensions.push(ext);
        Ok(file)
    }

    fn new_path(&self, ext: &str) -> PathBuf {
        self.dir.path().join("new").with_extension(ext)
    }

    fn old_path(&self, ext: &str) -> PathBuf {
        self.dir.path().join("old").with_extension(ext)
    }

    /// Gives the new files the database files' permissions and puts them in
    /// their place, then removes the database's old files.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`](crate::Error::Io) if a file can't be renamed or
    /// its permissions set. The database's files are then put back. If one
    /// can't be, the temporary directory is kept, holding the files that
    /// weren't put back.
    pub(crate) fn replace(self) -> Result<()> {
        for ext in &self.extensions {
            let permissions = fs::metadata(self.path.with_extension(ext))?.permissions();
            fs::set_permissions(self.new_path(ext), permissions)?;
        }
        self.swap()
    }

    /// Moves the database's files into the temporary directory, then moves
    /// the new files into their place. If a file can't be moved, it moves the
    /// database's files back.
    fn swap(self) -> Result<()> {
        let mut moved = 0;
        let mut result = Ok(());
        for ext in &self.extensions {
            result = fs::rename(self.path.with_extension(ext), self.old_path(ext));
            if result.is_err() {
                break;
            }
            moved += 1;
        }
        if result.is_ok() {
            for ext in &self.extensions {
                result = fs::rename(self.new_path(ext), self.path.with_extension(ext));
                if result.is_err() {
                    break;
                }
            }
        }
        let Err(err) = result else {
            return Ok(());
        };

        let restored = self.extensions[..moved]
            .iter()
            .all(|ext| fs::rename(self.old_path(ext), self.path.with_extension(ext)).is_ok());
        if !restored {
            // The directory now holds a database file, which must be kept.
            let _ = self.dir.keep();
        }
        Err(err.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;
    use std::io::Write;

    /// Writes a database file at `path` and a new file for each of
    /// `extensions`, with contents that name the file.
    fn new_files(path: &Path, extensions: &[&'static str]) -> NewFiles {
        let mut new = NewFiles::new(path).unwrap();
        for ext in extensions {
            fs::write(path.with_extension(ext), format!("old {ext}")).unwrap();
            let mut file = new.create(ext).unwrap();
            file.write_all(format!("new {ext}").as_bytes()).unwrap();
        }
        new
    }

    fn contents(path: &Path, extensions: &[&str]) -> Vec<String> {
        extensions
            .iter()
            .map(|ext| fs::read_to_string(path.with_extension(ext)).unwrap())
            .collect()
    }

    fn entries(dir: &Path) -> Vec<OsString> {
        let mut names: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn test_replace_puts_the_new_files_in_place() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("db");
        new_files(&path, &["si4", "sg4"]).replace().unwrap();

        assert_eq!(contents(&path, &["si4", "sg4"]), ["new si4", "new sg4"]);
        assert_eq!(entries(temp.path()), ["db.sg4", "db.si4"]);
    }

    #[test]
    fn test_a_failed_rename_puts_the_files_back() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("db");
        let new = new_files(&path, &["si4", "sg4"]);
        // The new index moves into place, then the new game file can't.
        fs::remove_file(new.new_path("sg4")).unwrap();

        assert!(new.swap().is_err());
        assert_eq!(contents(&path, &["si4", "sg4"]), ["old si4", "old sg4"]);
        assert_eq!(entries(temp.path()), ["db.sg4", "db.si4"]);
    }

    #[test]
    fn test_a_missing_database_file_changes_nothing() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("db");
        let new = new_files(&path, &["si4", "sg4"]);
        fs::remove_file(path.with_extension("sg4")).unwrap();

        assert!(new.replace().is_err());
        assert_eq!(contents(&path, &["si4"]), ["old si4"]);
        assert_eq!(entries(temp.path()), ["db.si4"]);
    }

    #[cfg(unix)]
    #[test]
    fn test_the_new_files_keep_the_database_files_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let temp = TempDir::new().unwrap();
        let path = temp.path().join("db");
        let new = new_files(&path, &["si4"]);
        let si4 = path.with_extension("si4");
        fs::set_permissions(&si4, fs::Permissions::from_mode(0o640)).unwrap();

        new.replace().unwrap();
        let mode = fs::metadata(&si4).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o640);
    }

    #[test]
    fn test_directories_for_one_database_dont_collide() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("db");
        let first = NewFiles::new(&path).unwrap();
        let second = NewFiles::new(&path).unwrap();

        assert_ne!(first.dir.path(), second.dir.path());
        let name = first.dir.path().file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with(".scidtopgn-compact-"), "{name}");
    }

    #[test]
    fn test_a_path_without_a_directory_uses_the_current_one() {
        let new = NewFiles::new(Path::new("db")).unwrap();
        let current = std::env::current_dir().unwrap();
        assert_eq!(new.dir.path().parent(), Some(current.as_path()));
    }
}
