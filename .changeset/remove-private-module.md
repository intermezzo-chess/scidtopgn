---
default: major
---

# Remove the `scidtopgn::private` module

`skip_tags`, `is_move_byte`, and `is_special_byte` were public only so that tests could use them, and the library didn't call them. `Game::decode_tags` reads a record's tags, and `parse_move_byte` splits a move byte into its piece and value.
