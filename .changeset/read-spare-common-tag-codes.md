---
default: patch
---

# Read games whose tags include a spare common-tag code

Reading the tags of a game record with a common-tag code from 251 to 254 misread the tags after it, so `Database::get_game` failed or misread the game. SCID never writes these spare codes, but, like SCID, reading a game now skips the value that follows one. The value is dropped, because SCID has no tag name for it.
