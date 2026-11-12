//! Tests of behavior across modules that use items private to the crate,
//! such as the index, name, and game files, so they can't be integration
//! tests.

mod test_compact;
mod test_database;
mod test_errors;
mod test_game;
mod test_index;
mod test_interop;
mod test_move_encoding;
mod test_namebase;
mod test_names_not_utf8;
mod test_position;
mod test_search;
mod test_twins;
mod test_variations;
