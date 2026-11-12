---
default: major
---

# Let option, game, and error types gain fields without a breaking change

`HeaderSearch`, `HeaderFlags`, `TwinOptions`, `DeleteTwin`, `Game`, `MoveNode`, and `PgnWarning` are `#[non_exhaustive]`, and so is each variant of `Error`, `CorruptGameKind`, and `InvalidMoveByte` that has fields. Start from `Default::default()` and set fields, instead of writing a struct expression, and add `..` to patterns that name a variant's fields, such as `Error::GameNotFound { number, .. }`.
