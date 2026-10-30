//! [`NameBase`], the player, event, site, and round names stored in a
//! `.sn4` file.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::io::{Read, Write};

use crate::error::{Error, Result};
use crate::pgn::windows_1252_char;

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

/// The new ID of each old name ID, or `None` for a name that was dropped,
/// indexed by kind and then by old ID.
pub(crate) type NewIds = [Vec<Option<u32>>; NUM_NAME_TYPES];

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
#[derive(Clone)]
pub struct NameBase {
    trees: [BTreeMap<String, u32>; NUM_NAME_TYPES],
    by_id: [Vec<Option<NameEntry>>; NUM_NAME_TYPES],
    num_names: [u32; NUM_NAME_TYPES],
    max_frequency: [u32; NUM_NAME_TYPES],
    timestamp: u32,
    /// The names read from bytes that weren't valid UTF-8, by kind.
    converted: Vec<(NameType, String)>,
    /// The bytes the name file stores for each name read from other bytes
    /// than its own, by kind and ID.
    stored: [HashMap<u32, Vec<u8>>; NUM_NAME_TYPES],
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
            converted: Vec::new(),
            stored: Default::default(),
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
        self.clear_converted();
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

    /// Returns the kind and text of each name that
    /// [`read_sn4`](Self::read_sn4) read from bytes that weren't valid UTF-8,
    /// in the order the file stores them. A [`Database`](crate::Database)
    /// empties the list once it has written the names as UTF-8.
    ///
    /// Older versions of SCID store the bytes of a Latin-1 or Windows-1252
    /// PGN file unchanged, so their name files can hold such names.
    pub fn converted_names(&self) -> &[(NameType, String)] {
        &self.converted
    }

    /// Empties the list of [`converted_names`](Self::converted_names), and
    /// forgets the bytes names were read from, once the names have been
    /// written as UTF-8.
    pub(crate) fn clear_converted(&mut self) {
        self.converted.clear();
        self.stored.iter_mut().for_each(HashMap::clear);
    }

    /// Returns the bytes the name file stores for the name with ID `id`, or
    /// `None` if there is no such name. They differ from the name's UTF-8
    /// when it was read from bytes that weren't valid UTF-8, or had
    /// characters that reading it removed, until the names are written.
    pub(crate) fn stored_bytes(&self, ntype: NameType, id: u32) -> Option<&[u8]> {
        match self.stored[ntype.as_usize()].get(&id) {
            Some(bytes) => Some(bytes),
            None => self.get_name(ntype, id).map(str::as_bytes),
        }
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
        self.stored[idx].remove(&id);

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

    /// Sets the frequency of every name, and the highest frequency of each
    /// kind, to 0.
    pub(crate) fn clear_frequencies(&mut self) {
        for (names, max) in self.by_id.iter_mut().zip(&mut self.max_frequency) {
            names.iter_mut().flatten().for_each(|e| e.frequency = 0);
            *max = 0;
        }
    }

    /// Returns a name base with only the names whose frequency isn't 0, and
    /// the new ID of each old ID, by kind, or `None` if every name has a
    /// frequency, no two names of a kind are the same, and the IDs have no
    /// gaps. The names left are renumbered as [`merged`](Self::merged)
    /// renumbers them.
    ///
    /// A name file can skip IDs, but [`add_name`](Self::add_name) gives a new
    /// name the ID after the number of names, which may already be taken.
    pub(crate) fn without_unused(&self) -> Option<(NameBase, NewIds)> {
        let all_used = self
            .by_id
            .iter()
            .flatten()
            .all(|e| e.as_ref().is_some_and(|e| e.frequency > 0));
        if all_used && !self.has_duplicates() {
            return None;
        }
        Some(self.renumbered(|e| e.frequency > 0))
    }

    /// Returns a name base in which the names of a kind that are the same
    /// are one name, and the new ID of each old ID, by kind, or `None` if no
    /// two names of a kind are the same. Names that no game uses are kept.
    ///
    /// Names read from the name file can be the same when they were stored
    /// differently, such as one as Latin-1 and the other as UTF-8, and SCID
    /// can't open a name file that has both. The merged name's frequency is
    /// the sum of theirs. Within each kind, the new IDs count from 0, without
    /// gaps, in the order of the old ones, and a merged name takes the place
    /// of its lowest old ID.
    pub(crate) fn merged(&self) -> Option<(NameBase, NewIds)> {
        self.has_duplicates().then(|| self.renumbered(|_| true))
    }

    /// Returns whether two names of a kind are the same.
    fn has_duplicates(&self) -> bool {
        self.trees
            .iter()
            .zip(&self.by_id)
            .any(|(tree, names)| tree.len() < names.iter().flatten().count())
    }

    /// Returns a name base with the names for which `keep` is true, those
    /// that are the same made one, and the new ID of each old ID, by kind,
    /// as [`merged`](Self::merged) describes.
    fn renumbered(&self, keep: impl Fn(&NameEntry) -> bool) -> (NameBase, NewIds) {
        let mut kept = NameBase::new();
        kept.timestamp = self.timestamp;
        let new_ids = std::array::from_fn(|idx| {
            self.by_id[idx]
                .iter()
                .map(|entry| {
                    let entry = entry.as_ref().filter(|e| keep(e))?;
                    Some(kept.add_counted(idx, &entry.name, entry.frequency))
                })
                .collect()
        });
        (kept, new_ids)
    }

    /// Adds `frequency` to the frequency of `name`, of the kind at `idx`,
    /// adding the name if it isn't present, and returns its ID.
    fn add_counted(&mut self, idx: usize, name: &str, frequency: u32) -> u32 {
        let id = match self.trees[idx].get(name) {
            Some(&id) => id,
            None => {
                let id = self.num_names[idx];
                self.trees[idx].insert(name.to_string(), id);
                self.by_id[idx].push(Some(NameEntry {
                    id,
                    name: name.to_string(),
                    frequency: 0,
                }));
                self.num_names[idx] += 1;
                id
            }
        };
        let entry = self.by_id[idx][id as usize]
            .as_mut()
            .expect("the IDs of a name base without gaps index its names");
        entry.frequency += frequency;
        self.max_frequency[idx] = self.max_frequency[idx].max(entry.frequency);
        id
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

/// Reads a name's bytes as UTF-8, and any bytes that aren't valid UTF-8 as
/// Windows-1252, as [`PgnParser::from_bytes`](crate::PgnParser::from_bytes)
/// reads PGN. Returns the name and whether any byte wasn't valid UTF-8.
fn decode_name(bytes: &[u8]) -> (String, bool) {
    let mut name = String::with_capacity(bytes.len());
    let mut converted = false;
    for chunk in bytes.utf8_chunks() {
        name.push_str(chunk.valid());
        converted |= !chunk.invalid().is_empty();
        name.extend(chunk.invalid().iter().map(|&b| windows_1252_char(b)));
    }
    (name, converted)
}

/// Returns the number of leading bytes `s1` and `s2` share, at most 255.
fn compute_prefix(s1: &[u8], s2: &[u8]) -> u8 {
    let count = s1.iter().zip(s2).take_while(|(b1, b2)| b1 == b2).count();
    count.min(255) as u8
}

impl NameBase {
    /// Reads a `.sn4` file into this name base, which should be empty.
    ///
    /// A name that isn't valid UTF-8, as older versions of SCID store
    /// Latin-1 or Windows-1252 names, is converted: the bytes that aren't
    /// valid UTF-8 are read as Windows-1252, as
    /// [`PgnParser::from_bytes`](crate::PgnParser::from_bytes) reads them,
    /// and [`converted_names`](Self::converted_names) lists the name. Then
    /// control characters and surrounding spaces are removed from each name,
    /// and a name longer than 255 bytes is cut as
    /// [`add_name`](Self::add_name) cuts it.
    ///
    /// Names of a kind that read the same, such as a name stored as Latin-1
    /// and as UTF-8, keep their IDs, which [`get_name`](Self::get_name)
    /// gives the same name for, and [`find_name`](Self::find_name) and
    /// [`add_name`](Self::add_name) give the lowest of them. A
    /// [`Database`](crate::Database) merges them into one name when it next
    /// writes its name file.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidMagic`] if the data doesn't start with
    /// [`NAMEBASE_MAGIC`], [`Error::NamePrefixTooLong`] for a damaged name,
    /// and [`Error::Io`] if the data can't be read or ends early.
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

            // A name shares its prefix with the previous name as stored.
            let mut prev_bytes = Vec::new();

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
                name_bytes.extend_from_slice(&prev_bytes[..prefix.min(prev_bytes.len())]);
                name_bytes.extend_from_slice(&suffix);

                let (name, converted) = decode_name(&name_bytes);
                let name: String = name.chars().filter(|&c| c >= ' ').collect();
                // The cut can leave spaces at the end, which reading the
                // name again would remove.
                let clean_name = stored_name(name.trim()).trim_end().to_string();
                if converted {
                    self.converted.push((ntype, clean_name.clone()));
                }
                if name_bytes != clean_name.as_bytes() {
                    self.stored[ntype.as_usize()].insert(id, name_bytes.clone());
                }

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
                self.trees[idx]
                    .entry(clean_name)
                    .and_modify(|first| *first = (*first).min(id))
                    .or_insert(id);

                prev_bytes = name_bytes;
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

    /// A name base with the players `names`, each used as often as given.
    fn players(names: &[(&str, u32)]) -> NameBase {
        let mut nb = NameBase::new();
        nb.set_timestamp(7);
        for &(name, uses) in names {
            let id = nb.add_name(NameType::Player, name).unwrap();
            (0..uses).for_each(|_| nb.increment_frequency(NameType::Player, id));
        }
        nb
    }

    #[test]
    fn test_without_unused_renumbers_the_names_left() {
        let nb = players(&[("A", 2), ("B", 0), ("C", 1), ("D", 0)]);
        let (kept, new_ids) = nb.without_unused().unwrap();

        assert_eq!(new_ids[0], [Some(0), None, Some(1), None]);
        assert!(new_ids[1..].iter().all(Vec::is_empty));
        assert_eq!(
            kept.get_all_names(NameType::Player),
            [(0, "A".to_string(), 2), (1, "C".to_string(), 1)]
        );
        assert_eq!(kept.find_name(NameType::Player, "C"), Some(1));
        assert_eq!(kept.count(NameType::Player), 2);
        assert_eq!(kept.max_frequency(NameType::Player), 2);
        assert_eq!(kept.timestamp(), 7);
    }

    #[test]
    fn test_without_unused_is_none_when_every_name_is_used() {
        assert!(players(&[("A", 1), ("B", 3)]).without_unused().is_none());
        assert!(NameBase::new().without_unused().is_none());
    }

    #[test]
    fn test_without_unused_closes_gaps_in_the_ids() {
        // A name file can skip IDs: here B has ID 2, and no name has ID 1.
        let mut nb = players(&[("A", 1), ("B", 1)]);
        let b = nb.by_id[0].pop().unwrap().map(|e| NameEntry { id: 2, ..e });
        nb.by_id[0].extend([None, b]);
        nb.trees[0].insert("B".to_string(), 2);
        let (kept, new_ids) = nb.without_unused().unwrap();

        assert_eq!(new_ids[0], [Some(0), None, Some(1)]);
        assert_eq!(
            kept.get_all_names(NameType::Player),
            [(0, "A".to_string(), 1), (1, "B".to_string(), 1)]
        );
    }

    #[test]
    fn test_without_unused_merges_names_that_are_the_same() {
        // A damaged name file can have names that are the same once read.
        let mut nb = players(&[("A", 1), ("B", 0), ("C", 2)]);
        nb.by_id[0][2].as_mut().unwrap().name = "A".to_string();
        let (kept, new_ids) = nb.without_unused().unwrap();

        assert_eq!(new_ids[0], [Some(0), None, Some(0)]);
        assert_eq!(
            kept.get_all_names(NameType::Player),
            [(0, "A".to_string(), 3)]
        );
        assert_eq!(kept.max_frequency(NameType::Player), 3);
    }

    #[test]
    fn test_clear_frequencies() {
        let mut nb = players(&[("A", 2), ("B", 1)]);
        nb.clear_frequencies();
        assert_eq!(nb.get_frequency(NameType::Player, 0), 0);
        assert_eq!(nb.get_frequency(NameType::Player, 1), 0);
        assert_eq!(nb.max_frequency(NameType::Player), 0);
        assert_eq!(nb.count(NameType::Player), 2);
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

    /// A name file whose players are `players`: each one's ID, frequency,
    /// bytes shared with the previous name, and remaining bytes, in the
    /// order the file stores them. It has no names of the other kinds.
    fn sn4_with_players(players: &[(u16, u8, u8, &[u8])]) -> Vec<u8> {
        let mut file = NAMEBASE_MAGIC.to_vec();
        file.extend([0; 4]);
        let max_frequency = players.iter().map(|p| p.1).max().unwrap_or(0);
        for counts in [players.len() as u8, max_frequency] {
            file.extend([0, 0, counts]);
            file.extend([0; 9]);
        }
        for (i, &(id, frequency, prefix, suffix)) in players.iter().enumerate() {
            file.extend(id.to_be_bytes());
            file.push(frequency);
            file.push(prefix + suffix.len() as u8);
            if i > 0 {
                file.push(prefix);
            }
            file.extend(suffix);
        }
        file
    }

    fn read(file: &[u8]) -> NameBase {
        let mut nb = NameBase::new();
        nb.read_sn4(&mut &file[..]).unwrap();
        nb
    }

    #[test]
    fn test_read_converts_names_that_arent_utf8() {
        let nb = read(&sn4_with_players(&[
            (0, 1, 0, b"M\xfcller, Hans"),
            (1, 1, 0, b"Anand"),
            // The five bytes Windows-1252 leaves undefined, and the euro.
            (2, 1, 0, b"\x81\x8d\x8f\x90\x9d \x80"),
            // A Latin-1 "\xc3\xa9" is valid UTF-8, so only the first byte
            // is converted.
            (3, 1, 0, b"\xe9\xc3\xa9"),
            (4, 1, 0, "Müller, Hans".as_bytes()),
        ]));

        let names: Vec<_> = nb
            .get_all_names(NameType::Player)
            .into_iter()
            .map(|(_, name, _)| name)
            .collect();
        assert_eq!(
            names,
            [
                "Müller, Hans",
                "Anand",
                "\u{81}\u{8d}\u{8f}\u{90}\u{9d} €",
                "éé",
                "Müller, Hans"
            ]
        );
        assert_eq!(
            nb.converted_names(),
            [
                (NameType::Player, "Müller, Hans".to_string()),
                (
                    NameType::Player,
                    "\u{81}\u{8d}\u{8f}\u{90}\u{9d} €".to_string()
                ),
                (NameType::Player, "éé".to_string()),
            ]
        );
        assert!(players(&[("A", 1)]).converted_names().is_empty());
    }

    #[test]
    fn test_read_cuts_a_converted_name_to_255_bytes() {
        // 200 Latin-1 bytes are 400 bytes of UTF-8, cut before the 128th é.
        let nb = read(&sn4_with_players(&[(0, 1, 0, &[0xe9; 200])]));
        let name = "é".repeat(127);
        assert_eq!(nb.get_name(NameType::Player, 0), Some(name.as_str()));
        assert_eq!(nb.converted_names(), [(NameType::Player, name.clone())]);

        let mut file = Vec::new();
        nb.write_sn4(&mut file).unwrap();
        assert_eq!(
            read(&file).get_name(NameType::Player, 0),
            Some(name.as_str())
        );
    }

    #[test]
    fn test_read_removes_spaces_the_cut_leaves_at_the_end() {
        // 127 é and a space are 255 bytes of UTF-8, and the é after them is
        // cut, which would leave the space at the end.
        let mut stored = vec![0xe9; 127];
        stored.extend([b' ', 0xe9]);
        let nb = read(&sn4_with_players(&[(0, 1, 0, &stored)]));
        let name = "é".repeat(127);
        assert_eq!(nb.get_name(NameType::Player, 0), Some(name.as_str()));

        let mut file = Vec::new();
        nb.write_sn4(&mut file).unwrap();
        assert_eq!(
            read(&file).get_name(NameType::Player, 0),
            Some(name.as_str())
        );
    }

    #[test]
    fn test_read_takes_the_shared_prefix_from_the_stored_name() {
        // The first name is stored with a control character, which isn't
        // part of the name read, but is part of the prefix the next shares.
        let nb = read(&sn4_with_players(&[(0, 1, 0, b"\x01Ab"), (1, 1, 2, b"Zz")]));
        assert_eq!(nb.get_name(NameType::Player, 0), Some("Ab"));
        assert_eq!(nb.get_name(NameType::Player, 1), Some("AZz"));
    }

    /// Three players that read as "Müller, Hans", stored as UTF-8, as
    /// Latin-1, and as Latin-1 with a space before it, with IDs 2, 0, and 3,
    /// and an unused player with ID 1.
    fn three_millers() -> NameBase {
        read(&sn4_with_players(&[
            (1, 0, 0, b"Anand"),
            (2, 3, 0, "Müller, Hans".as_bytes()),
            (0, 1, 1, b"\xfcller, Hans"),
            (3, 2, 0, b" M\xfcller, Hans"),
        ]))
    }

    #[test]
    fn test_names_that_read_the_same_share_the_lowest_id() {
        let nb = three_millers();
        for id in [0, 2, 3] {
            assert_eq!(nb.get_name(NameType::Player, id), Some("Müller, Hans"));
        }
        assert_eq!(nb.find_name(NameType::Player, "Müller, Hans"), Some(0));
        let mut nb = nb;
        assert_eq!(nb.add_name(NameType::Player, "Müller, Hans").unwrap(), 0);
    }

    #[test]
    fn test_merged_makes_names_that_are_the_same_one() {
        let (merged, new_ids) = three_millers().merged().unwrap();

        assert_eq!(new_ids[0], [Some(0), Some(1), Some(0), Some(0)]);
        assert!(new_ids[1..].iter().all(Vec::is_empty));
        // The unused name is kept.
        assert_eq!(
            merged.get_all_names(NameType::Player),
            [
                (0, "Müller, Hans".to_string(), 6),
                (1, "Anand".to_string(), 0)
            ]
        );
        assert_eq!(merged.count(NameType::Player), 2);
        assert_eq!(merged.max_frequency(NameType::Player), 6);
        assert!(merged.converted_names().is_empty());
        assert!(merged.merged().is_none());
    }

    #[test]
    fn test_merged_is_none_without_names_that_are_the_same() {
        assert!(players(&[("A", 1), ("B", 0)]).merged().is_none());
        assert!(NameBase::new().merged().is_none());
    }

    #[test]
    fn test_without_unused_merges_names_read_the_same_when_every_name_is_used() {
        let (kept, new_ids) = read(&sn4_with_players(&[
            (0, 1, 0, b"M\xfcller"),
            (1, 1, 0, "Müller".as_bytes()),
        ]))
        .without_unused()
        .unwrap();
        assert_eq!(new_ids[0], [Some(0), Some(0)]);
        assert_eq!(
            kept.get_all_names(NameType::Player),
            [(0, "Müller".to_string(), 2)]
        );
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
