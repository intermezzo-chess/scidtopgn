---
default: patch
---

# State what counts as a breaking change from 1.0 on

`docs/RELEASING.md` says what the semantic versioning promise covers from 1.0.0 on, including the minimum supported Rust version and the shakmaty dependency. The library documentation states that every public type is `Send` and `Sync`.
