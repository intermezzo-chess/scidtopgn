---
default: minor
---

# Find twin games and mark them deleted, as SCID does

`Database::delete_twins` finds twin games, such as the same game imported twice, and marks the copies SCID vs. PC's Delete Twin Games command (`sc_base duplicates`) would delete as logically deleted. It returns their numbers. `compact_games` then removes them.

`TwinOptions` selects what twins must have in common: the same players, colors, event, site, round, year, month, day, result, basic ECO code, and main-line moves up to 120 half-moves, with SCID's home pawn check first. It also selects whether to skip games shorter than 10 half-moves, which twin to delete (`DeleteTwin::Shorter`, `Older`, or `Newer`), whether games with comments or variations are kept, and whether ratings are copied to the kept game. Its defaults are those of SCID's Delete Twin Games window, and its documentation lists where the command's defaults differ. As in SCID, a game of at most 2 half-moves is the twin of a full game with the same year, site, and round, and pairs are compared in SCID's order, so the same games are deleted. Unlike the window, `delete_twins` doesn't undelete every game first.

Only delete flags change, and ratings if they are copied. If a game's record can't be read, nothing changes and the error is returned. SCID's results for games without an ECO code are undefined when ECO codes must match; here such games match.
