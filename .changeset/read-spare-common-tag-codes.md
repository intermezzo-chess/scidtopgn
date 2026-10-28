---
default: patch
---

# Read games whose tags include a spare common-tag code

Reading the tags of a game record with a common-tag code from 251 to 254 misread the tags after it, so `Game::decode` with `GAME_DECODE_TAGS`, `Game::decode_start`, and `Database::get_game` failed or misread the game. SCID never writes these spare codes, but, like SCID, `Game::decode_tags` now reads past the value that follows one. The value is dropped, because SCID has no tag name for it.
