//! [`NameBase`], the player, event, site, and round names stored in a
//! `.sn4` file.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Write};

use crate::error::{Error, Result};

/// The first eight bytes of a `.sn4` file.
pub const NAMEBASE_MAGIC: &[u8; 8] = b"Scid.sn\0";
/// The number of kinds of name: [`NameType`] has one variant per kind.
pub const NUM_NAME_TYPES: usize = 4;
/// The longest name the name file stores, in bytes.
const MAX_NAME_LEN: usize = 255;

/// A kind of name in the name file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NameType {
    /// A player: the `White` and `Black` tags.
    Player = 0,
    /// An event: the `Event` tag.
    Event = 1,
    /// A site: the `Site` tag.
    Site = 2,
    /// A round: the `Round` tag.
    Round = 3,
}

impl NameType {
    /// Returns the most names of this kind a database can hold, which is set
    /// by the width of the ID field in an index entry.
    pub const fn max_id(self) -> u32 {
        match self {
            NameType::Player => 1_048_575,
            NameType::Event => 524_287,
            NameType::Site => 524_287,
            NameType::Round => 262_143,
        }
    }

    /// Returns the kind's number, from 0 to 3, as the name file orders them.
    pub fn as_usize(self) -> usize {
        self as usize
    }
}

/// Converts a kind's number, as [`NameType::as_usize`] returns it.
///
/// Fails with [`Error::InvalidNameType`] for a number above 3.
impl TryFrom<usize> for NameType {
    type Error = Error;

    fn try_from(value: usize) -> Result<Self> {
        match value {
            0 => Ok(NameType::Player),
            1 => Ok(NameType::Event),
            2 => Ok(NameType::Site),
            3 => Ok(NameType::Round),
            _ => Err(Error::InvalidNameType { value }),
        }
    }
}

/// Names the kind in lowercase, such as `player`.
impl fmt::Display for NameType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            NameType::Player => "player",
            NameType::Event => "event",
            NameType::Site => "site",
            NameType::Round => "round",
        })
    }
}

/// A name and how many games use it.
#[derive(Debug, Clone)]
pub struct NameEntry {
    /// The ID index entries use to refer to the name.
    pub id: u32,
    /// The name.
    pub name: String,
    /// The number of games that use the name.
    pub frequency: u32,
}

/// The names of a database, by kind, with the IDs index entries refer to
/// them by.
///
/// # Examples
///
/// ```
/// use scidtopgn::{NameBase, NameType};
///
/// let mut names = NameBase::new();
/// let id = names.add_name(NameType::Player, "Carlsen, Magnus")?;
/// assert_eq!(names.add_name(NameType::Player, "Carlsen, Magnus")?, id);
/// assert_eq!(names.get_name(NameType::Player, id), Some("Carlsen, Magnus"));
/// assert_eq!(names.find_name(NameType::Event, "Carlsen, Magnus"), None);
/// # Ok::<(), scidtopgn::Error>(())
/// ```
pub struct NameBase {
    trees: [BTreeMap<String, u32>; NUM_NAME_TYPES],
    by_id: [Vec<Option<NameEntry>>; NUM_NAME_TYPES],
    num_names: [u32; NUM_NAME_TYPES],
    max_frequency: [u32; NUM_NAME_TYPES],
    timestamp: u32,
}

impl Default for NameBase {
    fn default() -> Self {
        Self::new()
    }
}

impl NameBase {
    /// Makes an empty name base.
    pub fn new() -> Self {
        NameBase {
            trees: [
                BTreeMap::new(),
                BTreeMap::new(),
                BTreeMap::new(),
                BTreeMap::new(),
            ],
            by_id: [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            num_names: [0; NUM_NAME_TYPES],
            max_frequency: [0; NUM_NAME_TYPES],
            timestamp: 0,
        }
    }

    /// Removes every name and resets the timestamp.
    pub fn clear(&mut self) {
        for i in 0..NUM_NAME_TYPES {
            self.trees[i].clear();
            self.by_id[i].clear();
            self.num_names[i] = 0;
            self.max_frequency[i] = 0;
        }
        self.timestamp = 0;
    }

    /// Sets the timestamp written to the file's header.
    pub fn set_timestamp(&mut self, ts: u32) {
        self.timestamp = ts;
    }

    /// Returns the timestamp from the file's header.
    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    /// Returns the number of names of kind `ntype`.
    pub fn count(&self, ntype: NameType) -> u32 {
        self.num_names[ntype.as_usize()]
    }

    /// Returns the highest frequency of any name of kind `ntype`.
    pub fn max_frequency(&self, ntype: NameType) -> u32 {
        self.max_frequency[ntype.as_usize()]
    }

    /// Returns the ID of `name`, adding it with a frequency of 0 if it isn't
    /// already present. IDs count from 0 in the order names are added.
    ///
    /// As in SCID, a name longer than 255 bytes is cut to 255 before it is
    /// looked up, so names that start with the same 255 bytes share an ID.
    /// Unlike SCID, a cut that would split a character ends before it.
    ///
    /// # Errors
    ///
    /// Returns [`Error::NameBaseFull`] if there are already
    /// [`ntype.max_id()`](NameType::max_id) names of kind `ntype`.
    pub fn add_name(&mut self, ntype: NameType, name: &str) -> Result<u32> {
        let idx = ntype.as_usize();
        let name = stored_name(name);

        if self.num_names[idx] >= ntype.max_id() {
            return Err(Error::NameBaseFull { name_type: ntype });
        }

        if let Some(&id) = self.trees[idx].get(name) {
            return Ok(id);
        }

        let id = self.num_names[idx];
        let entry = NameEntry {
            id,
            name: name.to_string(),
            frequency: 0,
        };

        self.trees[idx].insert(name.to_string(), id);

        while self.by_id[idx].len() <= id as usize {
            self.by_id[idx].push(None);
        }
        self.by_id[idx][id as usize] = Some(entry);

        self.num_names[idx] += 1;
        Ok(id)
    }

    /// Returns the ID of `name`, or `None` if it isn't present. A name
    /// longer than 255 bytes is cut as [`add_name`](Self::add_name) cuts it.
    pub fn find_name(&self, ntype: NameType, name: &str) -> Option<u32> {
        self.trees[ntype.as_usize()].get(stored_name(name)).copied()
    }

    /// Returns the name with ID `id`, or `None` if there is none.
    pub fn get_name(&self, ntype: NameType, id: u32) -> Option<&str> {
        self.by_id[ntype.as_usize()]
            .get(id as usize)?
            .as_ref()
            .map(|e| e.name.as_str())
    }

    /// Returns the frequency of the name with ID `id`, or 0 if there is none.
    pub fn get_frequency(&self, ntype: NameType, id: u32) -> u32 {
        self.by_id[ntype.as_usize()]
            .get(id as usize)
            .and_then(|e| e.as_ref().map(|e| e.frequency))
            .unwrap_or(0)
    }

    /// Adds 1 to the frequency of the name with ID `id`, if there is one.
    pub fn increment_frequency(&mut self, ntype: NameType, id: u32) {
        let idx = ntype.as_usize();
        if let Some(Some(entry)) = self.by_id[idx].get_mut(id as usize) {
            entry.frequency += 1;
            if entry.frequency > self.max_frequency[idx] {
                self.max_frequency[idx] = entry.frequency;
            }
        }
    }

    /// Subtracts 1 from the frequency of the name with ID `id`, if there is
    /// one and its frequency isn't 0. The highest frequency stays as it was,
    /// which only makes the name file store frequencies wider than needed.
    pub(crate) fn decrement_frequency(&mut self, ntype: NameType, id: u32) {
        if let Some(Some(entry)) = self.by_id[ntype.as_usize()].get_mut(id as usize) {
            entry.frequency = entry.frequency.saturating_sub(1);
        }
    }

    /// Returns the ID, name, and frequency of every name of kind `ntype`, in
    /// ID order.
    pub fn get_all_names(&self, ntype: NameType) -> Vec<(u32, String, u32)> {
        self.by_id[ntype.as_usize()]
            .iter()
            .filter_map(|opt| opt.as_ref().map(|e| (e.id, e.name.clone(), e.frequency)))
            .collect()
    }

    fn sorted_names(&self, ntype: NameType) -> Vec<&NameEntry> {
        let idx = ntype.as_usize();
        let mut entries: Vec<&NameEntry> = self.by_id[idx]
            .iter()
            .filter_map(|opt| opt.as_ref())
            .collect();
        entries.sort_by(|a, b| scid_order(a.name.as_bytes(), b.name.as_bytes()));
        entries
    }
}

/// Returns the part of `name` that the name file stores: at most
/// [`MAX_NAME_LEN`] bytes, ending on a character boundary.
fn stored_name(name: &str) -> &str {
    &name[..name.floor_char_boundary(MAX_NAME_LEN)]
}

/// Compares two names in the order SCID requires in a name file.
///
/// SCID keeps the names of each kind in one tree per first byte, taken in
/// unsigned order. Within a tree it compares names with `strCompare`, as
/// signed `char`s followed by a 0 byte, so a byte of 0x80 or above sorts
/// before every ASCII byte and before the end of a shorter name. SCID
/// rejects a file that breaks the order within a tree, and can't look
/// names up in one that breaks the order of first bytes.
fn scid_order(a: &[u8], b: &[u8]) -> Ordering {
    fn signed(s: &[u8]) -> impl Iterator<Item = i8> + '_ {
        s.iter().map(|&byte| byte as i8).chain(std::iter::once(0))
    }
    a.first()
        .cmp(&b.first())
        .then_with(|| signed(a).cmp(signed(b)))
}

fn read_one_byte<R: Read>(reader: &mut R) -> Result<u8> {
    let mut buf = [0u8; 1];
    reader.read_exact(&mut buf)?;
    Ok(buf[0])
}

fn write_one_byte<W: Write>(writer: &mut W, value: u8) -> Result<()> {
    writer.write_all(&[value])?;
    Ok(())
}

fn read_two_bytes<R: Read>(reader: &mut R) -> Result<u16> {
    let mut buf = [0u8; 2];
    reader.read_exact(&mut buf)?;
    Ok(u16::from_be_bytes(buf))
}

fn write_two_bytes<W: Write>(writer: &mut W, value: u16) -> Result<()> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

fn read_three_bytes<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 3];
    reader.read_exact(&mut buf)?;
    Ok((buf[0] as u32) << 16 | (buf[1] as u32) << 8 | (buf[2] as u32))
}

fn write_three_bytes<W: Write>(writer: &mut W, value: u32) -> Result<()> {
    let b0 = ((value >> 16) & 0xFF) as u8;
    let b1 = ((value >> 8) & 0xFF) as u8;
    let b2 = (value & 0xFF) as u8;
    writer.write_all(&[b0, b1, b2])?;
    Ok(())
}

fn read_four_bytes<R: Read>(reader: &mut R) -> Result<u32> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_be_bytes(buf))
}

fn write_four_bytes<W: Write>(writer: &mut W, value: u32) -> Result<()> {
    writer.write_all(&value.to_be_bytes())?;
    Ok(())
}

/// Returns the number of leading bytes `s1` and `s2` share, at most 255.
fn compute_prefix(s1: &[u8], s2: &[u8]) -> u8 {
    let count = s1.iter().zip(s2).take_while(|(b1, b2)| b1 == b2).count();
    count.min(255) as u8
}

impl NameBase {
    /// Reads a `.sn4` file into this name base, which should be empty.
    /// Control characters and surrounding spaces are removed from each name.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMagic`] if the data doesn't start with
    /// [`NAMEBASE_MAGIC`], [`Error::NamePrefixTooLong`] or
    /// [`Error::NameNotUtf8`] for a damaged name, and [`Error::Io`] if the
    /// data can't be read or ends early.
    pub fn read_sn4<R: Read>(&mut self, reader: &mut R) -> Result<()> {
        let mut magic = [0u8; 8];
        reader.read_exact(&mut magic)?;
        if &magic != NAMEBASE_MAGIC {
            return Err(Error::InvalidMagic);
        }

        self.timestamp = read_four_bytes(reader)?;

        let mut num_names = [0u32; NUM_NAME_TYPES];
        for n in num_names.iter_mut() {
            *n = read_three_bytes(reader)?;
        }

        let mut max_freq = [0u32; NUM_NAME_TYPES];
        for freq in max_freq.iter_mut() {
            *freq = read_three_bytes(reader)?;
        }

        self.max_frequency = max_freq;

        for nt_idx in 0..NUM_NAME_TYPES {
            let count = num_names[nt_idx];
            let max_f = max_freq[nt_idx];
            let ntype = NameType::try_from(nt_idx)?;

            let mut prev_name = String::new();

            for i in 0..count {
                let id = if count >= 65536 {
                    read_three_bytes(reader)?
                } else {
                    read_two_bytes(reader)? as u32
                };

                let frequency = if max_f >= 65536 {
                    read_three_bytes(reader)?
                } else if max_f >= 256 {
                    read_two_bytes(reader)? as u32
                } else {
                    read_one_byte(reader)? as u32
                };

                let length = read_one_byte(reader)?;
                let prefix = if i > 0 { read_one_byte(reader)? } else { 0 };
                if prefix > length {
                    return Err(Error::NamePrefixTooLong {
                        name_type: ntype,
                        prefix,
                        length,
                    });
                }
                let (length, prefix) = (usize::from(length), usize::from(prefix));

                let mut suffix = vec![0u8; length - prefix];
                reader.read_exact(&mut suffix)?;

                let mut name_bytes = Vec::with_capacity(length);
                if prefix > 0 {
                    let prev_bytes = prev_name.as_bytes();
                    name_bytes.extend_from_slice(&prev_bytes[..prefix.min(prev_bytes.len())]);
                }
                name_bytes.extend_from_slice(&suffix);

                let name = String::from_utf8(name_bytes)
                    .map_err(|_| Error::NameNotUtf8 { name_type: ntype })?;

                let clean_name: String = name
                    .chars()
                    .filter(|&c| c >= ' ')
                    .collect::<String>()
                    .trim()
                    .to_string();

                let entry = NameEntry {
                    id,
                    name: clean_name.clone(),
                    frequency,
                };

                let idx = ntype.as_usize();
                while self.by_id[idx].len() <= id as usize {
                    self.by_id[idx].push(None);
                }
                self.by_id[idx][id as usize] = Some(entry);
                self.trees[idx].insert(clean_name.clone(), id);

                prev_name = clean_name;
            }

            self.num_names[nt_idx] = count;
        }

        Ok(())
    }

    /// Writes the names as a `.sn4` file, sorted within each kind in the
    /// order SCID requires: by first byte, then by bytes compared as signed,
    /// as SCID's `strCompare` does on x86 and Apple arm64.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] if the data can't be written.
    pub fn write_sn4<W: Write>(&self, writer: &mut W) -> Result<()> {
        writer.write_all(NAMEBASE_MAGIC)?;
        write_four_bytes(writer, self.timestamp)?;

        for i in 0..NUM_NAME_TYPES {
            write_three_bytes(writer, self.num_names[i])?;
        }

        for i in 0..NUM_NAME_TYPES {
            write_three_bytes(writer, self.max_frequency[i])?;
        }

        for nt_idx in 0..NUM_NAME_TYPES {
            let ntype = NameType::try_from(nt_idx)?;
            let entries = self.sorted_names(ntype);
            let count = self.num_names[nt_idx];
            let max_f = self.max_frequency[nt_idx];

            let mut prev_name: Option<&[u8]> = None;

            for entry in &entries {
                if count >= 65536 {
                    write_three_bytes(writer, entry.id)?;
                } else {
                    write_two_bytes(writer, entry.id as u16)?;
                }

                if max_f >= 65536 {
                    write_three_bytes(writer, entry.frequency)?;
                } else if max_f >= 256 {
                    write_two_bytes(writer, entry.frequency as u16)?;
                } else {
                    write_one_byte(writer, entry.frequency as u8)?;
                }

                let name_bytes = entry.name.as_bytes();
                write_one_byte(writer, name_bytes.len() as u8)?;

                let prefix = if let Some(prev) = prev_name {
                    let p = compute_prefix(prev, name_bytes);
                    write_one_byte(writer, p)?;
                    p
                } else {
                    0
                };

                writer.write_all(&name_bytes[prefix as usize..])?;

                prev_name = Some(name_bytes);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_type() {
        assert_eq!(NameType::Player.max_id(), 1_048_575);
        assert_eq!(NameType::Event.max_id(), 524_287);
        assert_eq!(NameType::Site.max_id(), 524_287);
        assert_eq!(NameType::Round.max_id(), 262_143);
    }

    #[test]
    fn test_decrement_frequency() {
        let mut nb = NameBase::new();
        let id = nb.add_name(NameType::Event, "Open").unwrap();
        nb.increment_frequency(NameType::Event, id);
        nb.increment_frequency(NameType::Event, id);

        nb.decrement_frequency(NameType::Event, id);
        assert_eq!(nb.get_frequency(NameType::Event, id), 1);
        nb.decrement_frequency(NameType::Event, id);
        nb.decrement_frequency(NameType::Event, id);
        assert_eq!(nb.get_frequency(NameType::Event, id), 0);
        assert_eq!(nb.max_frequency(NameType::Event), 2);
        // A missing name is ignored.
        nb.decrement_frequency(NameType::Event, id + 1);
        nb.decrement_frequency(NameType::Player, id);
    }

    #[test]
    fn test_add_and_find() {
        let mut nb = NameBase::new();

        let id1 = nb.add_name(NameType::Player, "Carlsen, Magnus").unwrap();
        let id2 = nb.add_name(NameType::Player, "Caruana, Fabiano").unwrap();

        assert_eq!(id1, 0);
        assert_eq!(id2, 1);
        assert_eq!(nb.count(NameType::Player), 2);

        assert_eq!(nb.find_name(NameType::Player, "Carlsen, Magnus"), Some(0));
        assert_eq!(nb.find_name(NameType::Player, "Caruana, Fabiano"), Some(1));
        assert_eq!(nb.find_name(NameType::Player, "Nakamura, Hikaru"), None);

        assert_eq!(nb.get_name(NameType::Player, 0), Some("Carlsen, Magnus"));
        assert_eq!(nb.get_name(NameType::Player, 1), Some("Caruana, Fabiano"));
    }

    #[test]
    fn test_frequency() {
        let mut nb = NameBase::new();

        let id = nb.add_name(NameType::Player, "Carlsen, Magnus").unwrap();
        assert_eq!(nb.get_frequency(NameType::Player, id), 0);

        nb.increment_frequency(NameType::Player, id);
        assert_eq!(nb.get_frequency(NameType::Player, id), 1);

        nb.increment_frequency(NameType::Player, id);
        assert_eq!(nb.get_frequency(NameType::Player, id), 2);
        assert_eq!(nb.max_frequency(NameType::Player), 2);

        // A less frequent name, or an ID with no name, leaves the maximum.
        let other = nb.add_name(NameType::Player, "Caruana, Fabiano").unwrap();
        nb.increment_frequency(NameType::Player, other);
        nb.increment_frequency(NameType::Player, 99);
        assert_eq!(nb.get_frequency(NameType::Player, other), 1);
        assert_eq!(nb.get_frequency(NameType::Player, 99), 0);
        assert_eq!(nb.max_frequency(NameType::Player), 2);
    }

    #[test]
    fn test_prefix() {
        assert_eq!(compute_prefix(b"Carlsen, Magnus", b"Caruana, Fabiano"), 3);
        assert_eq!(compute_prefix(b"Carlsen, Magnus", b"Carlsen, Henrik"), 9);
        assert_eq!(compute_prefix(b"abc", b"def"), 0);
        assert_eq!(compute_prefix(b"", b"anything"), 0);
        // Ä and É share their first byte, 0xC3.
        assert_eq!(compute_prefix("Ärger".as_bytes(), "Émile".as_bytes()), 1);
    }

    /// Returns the names of kind `ntype` in the order `write_sn4` writes
    /// them, with each name's prefix byte. Expects fewer than 65,536 names
    /// and frequencies below 256.
    fn written_names(nb: &NameBase, ntype: NameType) -> Vec<(String, u8)> {
        let mut file = Vec::new();
        nb.write_sn4(&mut file).unwrap();
        let mut pos = 36;
        let mut names = Vec::new();
        for nt_idx in 0..NUM_NAME_TYPES {
            let mut prev: Vec<u8> = Vec::new();
            for i in 0..nb.num_names[nt_idx] {
                // Skip the 2-byte ID and the 1-byte frequency.
                pos += 3;
                let length = usize::from(file[pos]);
                pos += 1;
                let prefix = if i > 0 { file[pos] } else { 0 };
                if i > 0 {
                    pos += 1;
                }
                let suffix = length - usize::from(prefix);
                let mut name = prev[..usize::from(prefix)].to_vec();
                name.extend_from_slice(&file[pos..pos + suffix]);
                pos += suffix;
                if nt_idx == ntype.as_usize() {
                    names.push((String::from_utf8(name.clone()).unwrap(), prefix));
                }
                prev = name;
            }
        }
        assert_eq!(pos, file.len());
        names
    }

    #[test]
    fn test_write_in_scid_order() {
        let mut nb = NameBase::new();
        for name in [
            "Øst, Ole",
            "Bl",
            "Zed",
            "Blagojević, Dragisa",
            "",
            "Émile",
            "Blübaum, Matthias",
            "Ärger, Hans",
            "Abc",
        ] {
            nb.add_name(NameType::Player, name).unwrap();
        }
        nb.add_name(NameType::Site, "Zug").unwrap();
        nb.add_name(NameType::Site, "Zürich").unwrap();

        // Names group by first byte, in unsigned order. Within a group, a
        // byte of 0x80 or above sorts before ASCII and before the end of a
        // shorter name, and an ASCII prefix still sorts first.
        assert_eq!(
            written_names(&nb, NameType::Player),
            [
                ("", 0),
                ("Abc", 0),
                ("Blübaum, Matthias", 0),
                ("Bl", 2),
                ("Blagojević, Dragisa", 2),
                ("Zed", 0),
                ("Ärger, Hans", 0),
                ("Émile", 1),
                ("Øst, Ole", 1),
            ]
            .map(|(name, prefix)| (name.to_string(), prefix))
        );
        assert_eq!(
            written_names(&nb, NameType::Site),
            [("Zürich".to_string(), 0), ("Zug".to_string(), 1)]
        );

        // Lookups and IDs don't depend on the order.
        assert_eq!(nb.find_name(NameType::Player, "Øst, Ole"), Some(0));
        assert_eq!(nb.find_name(NameType::Player, "Abc"), Some(8));
        assert_eq!(nb.add_name(NameType::Player, "Bl").unwrap(), 1);
    }

    #[test]
    fn test_scid_order() {
        assert_eq!(scid_order(b"Bl", b"Bla"), Ordering::Less);
        assert_eq!(scid_order("Blü".as_bytes(), b"Bl"), Ordering::Less);
        assert_eq!(scid_order(b"Zed", "Ärger".as_bytes()), Ordering::Less);
        assert_eq!(scid_order(b"", b"A"), Ordering::Less);
        assert_eq!(
            scid_order("Émile".as_bytes(), "Émile".as_bytes()),
            Ordering::Equal
        );
    }
}
