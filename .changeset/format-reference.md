---
default: patch
---

# Correct and split the SCID database format reference

`docs/SCID_FORMAT.md`, with a document for each of the `.si4`, `.sn4`, and `.sg4` files, replaces `SCID_DATABASE_FORMAT.md`. Each rule links to the source of SCID vs. PC 4.26 and says how it was checked. `docs/SCID_FORMAT_AUDIT.md` lists the errors that were corrected, among them the width of name IDs, the layout of name records, and the codes for annotation counts.

`docs/COMPATIBILITY.md` was audited too. It now lists losses and differences it left out, such as those of comments that contain `}`, dates with unknown parts, and tags after a game's 40th, and corrects others.
