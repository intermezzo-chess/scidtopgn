---
default: minor
---

# Read and write databases whose names aren't UTF-8

Older versions of SCID, and the C++ reference converter, store the names of a Latin-1 or Windows-1252 PGN file as they are. Export, append, and `info` failed for these databases, because their name files hold names that aren't valid UTF-8.

- `NameBase::read_sn4` reads the bytes of such a name that aren't valid UTF-8 as Windows-1252, as `PgnParser::from_bytes` reads PGN, and the new `NameBase::converted_names` lists the names it converted. The CLI warns about each one.
- **Writing a database stores the converted names as UTF-8**, as SCID vs. PC stores the names of PGN it imports. If two names of a kind read the same, such as a name stored once as Latin-1 and once as UTF-8, `Database::flush`, `close`, `compact_games`, and `compact_names` merge them into one name, whose frequency is the sum of theirs, and give the names new IDs, because SCID can't open a name file that holds a name twice. `flush` then writes the name and index files in a temporary directory and replaces both once they are complete, and returns `Error::Game` with `Error::NameNotFound` for a game that refers to a name the name file lacks.
- A name longer than 255 bytes once converted is cut to 255, as `add_name` cuts it.
- A front-coded name now shares its prefix with the previous name as stored, rather than as read with control characters and surrounding spaces removed.
- `Error::NameNotUtf8` is removed, since no name fails to read for that reason.
