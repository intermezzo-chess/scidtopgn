---
default: minor
---

# Search games by their header, as SCID does, and export the matches

`Database::search_header` returns the games that match a `HeaderSearch`, counting from 0, as SCID vs. PC's header search (`sc_search header`) selects them. It reads only the index and the names, never a game's record. `HeaderSearch` has SCID's criteria: White, Black, event, site, and round names, matched as text without case or spaces, or as a pattern in double quotes; a range of dates, results, White's and Black's ratings, and their difference, where an unrated player counts with their highest rating in the database; a range of ECO codes whose end includes its extensions, and whether games without one match; the number of half-moves; the side to move at the end; game numbers counting from 1, or from the end if negative; ignoring colors; and, in `HeaderFlags`, whether each of the 25 index flags must be set, as a `FlagMatch`. Its defaults are those of SCID's search window, which matches every game of at most 999 half-moves. The new `Eco` type parses and formats ECO codes such as `B20` and `B20a1`, and `IndexEntry::get_custom_flag` reads custom flags 1 to 6.

`scidtopgn export`, with or without the subcommand, exports only the games that match the new options `--white`, `--black`, `--event`, `--site`, `--round`, `--date`, `--result`, `--white-elo`, `--black-elo`, `--elo-diff`, `--eco`, `--eco-missing`, `--length`, `--to-move`, `--ignore-colors`, and `--flag`. Ranges are written `FROM..TO`, with either end optional. Combined with `--games`, a game must match both, and deleted games are still skipped. With search options, `--count` prints the number of games that match, including deleted ones, as SCID counts them.
