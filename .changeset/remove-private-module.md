---
default: major
---

# Remove the `scidtopgn::private` module

`skip_tags`, `is_move_byte`, and `is_special_byte` were public only so that tests could use them, and the library didn't call them.
