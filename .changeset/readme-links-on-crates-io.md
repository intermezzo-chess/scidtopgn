---
default: patch
---

# Fix the README's links to the documentation on crates.io

The README linked to files in `docs/` with relative links, which crates.io resolves against each crate's own directory, so on both crates' pages the links to the SCID compatibility, PGN import, and benchmark documents and to the compatibility policy were broken. They now point to the files on GitHub.
