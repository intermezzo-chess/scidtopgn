---
default: major
---

# Require Rust 1.97 or later

Building `scidtopgn` or `scidtopgn-cli`, including with `cargo install --locked`, now requires Rust 1.97 or later instead of 1.70. The library's new chess rules dependency, shakmaty 0.30.2, requires it.
