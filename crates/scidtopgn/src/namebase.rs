use std::collections::BTreeMap;
use std::fmt;
use std::io::{Read, Write};

use crate::error::{Error, Result};

pub const NAMEBASE_MAGIC: &[u8; 8] = b"Scid.sn\0";
pub const NUM_NAME_TYPES: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NameType {
    Player = 0,
    Event = 1,
    Site = 2,
    Round = 3,
}

impl NameType {
    pub const fn max_id(self) -> u32 {
        match self {
            NameType::Player => 1_048_575,
            NameType::Event => 524_287,
            NameType::Site => 524_287,
            NameType::Round => 262_143,
        }
    }

    pub fn as_usize(self) -> usize {
        self as usize
    }
}

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

#[derive(Debug, Clone)]
pub struct NameEntry {
    pub id: u32,
    pub name: String,
    pub frequency: u32,
}

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

    pub fn clear(&mut self) {
        for i in 0..NUM_NAME_TYPES {
            self.trees[i].clear();
            self.by_id[i].clear();
            self.num_names[i] = 0;
            self.max_frequency[i] = 0;
        }
        self.timestamp = 0;
    }

    pub fn set_timestamp(&mut self, ts: u32) {
        self.timestamp = ts;
    }

    pub fn timestamp(&self) -> u32 {
        self.timestamp
    }

    pub fn count(&self, ntype: NameType) -> u32 {
        self.num_names[ntype.as_usize()]
    }

    pub fn max_frequency(&self, ntype: NameType) -> u32 {
        self.max_frequency[ntype.as_usize()]
    }

    pub fn add_name(&mut self, ntype: NameType, name: &str) -> Result<u32> {
        let idx = ntype.as_usize();

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

    pub fn find_name(&self, ntype: NameType, name: &str) -> Option<u32> {
        self.trees[ntype.as_usize()].get(name).copied()
    }

    pub fn get_name(&self, ntype: NameType, id: u32) -> Option<&str> {
        self.by_id[ntype.as_usize()]
            .get(id as usize)?
            .as_ref()
            .map(|e| e.name.as_str())
    }

    pub fn get_frequency(&self, ntype: NameType, id: u32) -> u32 {
        self.by_id[ntype.as_usize()]
            .get(id as usize)
            .and_then(|e| e.as_ref().map(|e| e.frequency))
            .unwrap_or(0)
    }

    pub fn increment_frequency(&mut self, ntype: NameType, id: u32) {
        let idx = ntype.as_usize();
        if let Some(Some(entry)) = self.by_id[idx].get_mut(id as usize) {
            entry.frequency += 1;
            if entry.frequency > self.max_frequency[idx] {
                self.max_frequency[idx] = entry.frequency;
            }
        }
    }

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
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        entries
    }
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

fn compute_prefix(s1: &str, s2: &str) -> u8 {
    let mut count = 0usize;
    for (c1, c2) in s1.chars().zip(s2.chars()) {
        if c1 == c2 {
            count += 1;
        } else {
            break;
        }
    }
    count as u8
}

impl NameBase {
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

            let mut prev_name: Option<&str> = None;

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

                let length = entry.name.len().min(255) as u8;
                write_one_byte(writer, length)?;

                let prefix = if let Some(prev) = prev_name {
                    let p = compute_prefix(prev, &entry.name);
                    write_one_byte(writer, p)?;
                    p
                } else {
                    0
                };

                let name_bytes = entry.name.as_bytes();
                writer.write_all(&name_bytes[prefix as usize..length as usize])?;

                prev_name = Some(&entry.name);
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
    }

    #[test]
    fn test_prefix() {
        assert_eq!(compute_prefix("Carlsen, Magnus", "Caruana, Fabiano"), 3);
        assert_eq!(compute_prefix("Carlsen, Magnus", "Carlsen, Henrik"), 9);
        assert_eq!(compute_prefix("abc", "def"), 0);
        assert_eq!(compute_prefix("", "anything"), 0);
    }
}
