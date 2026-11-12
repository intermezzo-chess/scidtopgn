---
default: major
---

# Keep the library's public API to databases, games, and PGN

The modules are private, and every public item is at the crate root. `scidtopgn::index::MAX_ELO` is now `scidtopgn::MAX_ELO`, and `scidtopgn::search::DEFAULT_MAX_LENGTH` is `scidtopgn::DEFAULT_MAX_LENGTH`.

The types and functions that read and write SCID's files are no longer public: `ByteBuffer`, `GFile`, `FileMode`, `NFile`, `NameBase`, `NameEntry`, `Index`, `IndexHeader`, `ScidPosition`, `PieceList`, `encode_move`, `decode_move`, `make_move_byte`, `parse_move_byte`, `eco_to_string`, and `eco_from_string`, and the constants that describe the file formats, such as `ENCODE_*`, `IDX_FLAG_*`, `IDX_MASK_*`, `INDEX_*`, `GF_BLOCKSIZE`, `NAMEBASE_MAGIC`, and `GAME_DECODE_*`. `MAX_NAGS` is private too: `MoveNode::nags` states that a move keeps at most 7 NAGs.

- `Database::index`, `nfile`, and `gfile` are removed. The new `Database::converted_names` lists the names that weren't UTF-8, and `Database::name_count` counts the names of a kind.
- `IndexEntry` is read-only, and keeps the getters that need no knowledge of SCID's packing: the date, its parts, the result, the ratings, the ECO code as text, the half-move count, the start, promotion, and delete flags, and the variation, comment, and NAG counts. `get_date` returns a `Date`, replacing `get_date_obj`.
- `Game::decode`, `decode_start`, `decode_tags`, `initial_position`, `load_standard_tags`, and `add_variation`, and the `current_move` field, are private.
- `GameResult::to_byte` and `from_byte`, `RatingType::to_byte` and `from_byte`, `Date::from_raw`, `raw`, and `month_day`, `Eco::code`, and `NameType::as_usize` are private.
- `date_valid_string`, `date_make`, `date_get_year`, `date_get_month`, `date_get_day`, `date_decode_to_string`, `date_encode_from_string`, `Date::add_months`, and `GameResult::from_char` and `to_char` are removed or private. The library doesn't use them; `Date` reads, writes, and compares dates.
- `Error::BufferFull` and `Error::NoCurrentMove` are removed. Only the private `ByteBuffer` and `Game::add_variation` returned them.
