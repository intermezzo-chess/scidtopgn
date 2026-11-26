---
default: patch
---

# Warn about a `}` or `]` before the first move

A stray `}` or `]` before the first move now gives the same `skipped unknown text` warning as one after a move, once per character, and the game is imported as before. Other characters skipped before the first move, such as `!` or `.`, stay silent: they are annotation symbols or move-number punctuation, not stray closers.
