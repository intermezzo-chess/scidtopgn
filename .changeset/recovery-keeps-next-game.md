---
default: patch
---

# Keep the next game after a failed header

A line starting with `[` that isn't a tag no longer takes the next game with it: the failed header ends at that line, and the first well-formed tag after it starts a new game. A broken tag still ends its header with the failed game — its remaining tag lines are skipped. A game without a result now also ends at a tag line indented with whitespace other than spaces and tabs, as text before a game already does, instead of failing at the `[` and skipping to the end of the input.
