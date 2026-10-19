# Rust conventions

## Scope

These conventions apply to Rust source, tests, examples, build scripts, and
Cargo manifests.

Follow the repository’s existing Rust edition, minimum supported Rust version,
crate structure, async runtime, error strategy, and dependency choices. Do not
upgrade them unless the task requires it.

Repository-specific conventions take precedence over this document.

## Automated enforcement

Use rustfmt for formatting and Clippy for idiom and correctness. Repository
configuration is authoritative.

Use the project’s documented verification commands when available. Otherwise,
run:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Do not suppress lints broadly or weaken existing checks to make a change pass.
When a lint does not apply, use the narrowest practical allowance and explain
non-obvious exceptions.

Do not assume all Cargo features are compatible; follow the feature combinations
used by the repository and CI.

## Design

Use the type system to make invalid states difficult to represent:

- Prefer enums for modes and states.
- Prefer newtypes for validated or semantically distinct values.
- Prefer descriptive types over boolean mode arguments.
- Keep visibility as narrow as practical.

Express ownership according to the operation:

- Borrow values used temporarily.
- Take ownership when storing, consuming, or transferring them.
- Avoid cloning merely to bypass an ownership or lifetime problem.

Follow conventional Rust APIs:

- Use `From`, `TryFrom`, `AsRef`, and related traits when their semantics fit.
- Follow `as_`, `to_`, and `into_` conversion naming.
- Implement common traits when they accurately describe the type.
- Keep public APIs small and unsurprising.

Prefer safe Rust. When unsafe code is necessary, keep it narrow, encapsulate it
where possible, document public safety requirements, and add a nearby `SAFETY:`
explanation for each unsafe operation.

In async code, use runtime-compatible non-blocking APIs. Do not hold synchronous
lock guards across `.await`; isolate blocking I/O or substantial blocking work
using the runtime’s designated mechanism.

## Boundaries and errors

Use `Result` for recoverable failures. Reserve panics for bugs, violated
invariants, and genuinely unrecoverable states.

Do not use `unwrap` for failures caused by user input, configuration,
filesystems, networks, IPC, or other operational conditions. Use `expect` only
when failure would violate an invariant, and explain why success is expected.

Preserve useful error sources and add context when an error crosses a boundary.
Do not convert structured errors to strings prematurely or silently discard
failures.

Validate external data before treating it as a trusted domain type. Parsing or
deserialization does not necessarily establish semantic validity.

Do not expose secrets, credentials, private paths, or sensitive user data in
errors, panic messages, logs, or debug output.

## Testing

Add or update tests when behavior changes. Bug fixes should normally include a
regression test.

Use:

- unit tests near the module for focused internal behavior
- integration tests under `tests/` for public behavior across modules or crates
- documentation tests for public examples that should remain valid

Cover material success, failure, and boundary cases. Keep tests deterministic:
control time, randomness, filesystem state, and external dependencies where
practical.

Run the checks relevant to the affected crates, features, and targets before
reporting completion.

## References

- [Rust Style Guide](https://doc.rust-lang.org/style-guide/)
- [Clippy documentation](https://doc.rust-lang.org/clippy/)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/checklist.html)
- [Rust error handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Unsafe Rust reference](https://doc.rust-lang.org/reference/unsafe-keyword.html)
- [Rust test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html)
- [Async Rust: blocking and cancellation](https://rust-lang.github.io/async-book/part-guide/more-async-await.html)
