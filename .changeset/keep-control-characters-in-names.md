---
default: patch
---

# Preserve stored control characters in names, as SCID does

Reading a name file no longer removes control characters or trims names, so a name in a SCID database keeps its characters the first time scidtopgn writes the database, and names that differ only by control characters stay separate, as in SCID. Importing PGN and exporting PGN keep the control characters a name has after its spaces are trimmed: `M\x01nster Open` imports, survives an unrelated write, and exports as `[Event "M\x01nster Open"]`. The UTF-8 conversion of names that aren't UTF-8, and the cut of a converted name to 255 bytes, are unchanged.
