---
default: major
---

# Read ratings from player names and rating tags, tidy player names, and export each rating with its kind

PGN import now reads player names and ratings as SCID does, as [docs/PGN_IMPORT.md](https://github.com/intermezzo-chess/scidtopgn/blob/main/docs/PGN_IMPORT.md) describes:

- `White` and `Black` tag values are tidied: spaces at the start and at the end are dropped, and each comma that something follows gets one space after it, so `Carlsen,Magnus` is stored as `Carlsen, Magnus`. Before, names were stored as written.
- A rating at the end of a player name, such as `Carlsen, Magnus (2850)`, is removed from the name and becomes the player's Elo rating.
- A rating tag of one of SCID's kinds, `WhiteElo`, `WhiteRating`, `WhiteRapid`, `WhiteICCF`, `WhiteUSCF`, `WhiteDWZ`, or `WhiteECF`, or the same for Black, sets the player's rating and its kind. The first one with a rating is used, and later ones are kept as ordinary tags. Before, only `WhiteElo` and `BlackElo` were read, and the last one was used.
- A rating is read as SCID reads it: `2100x` is 2100, and a value with no digits, such as `?`, is no rating. A negative value is no rating.
- A rating above 4,000 is stored as 4,000, with the new `PgnWarningKind::RatingTooLarge` warning. Before, it was stored as 4,000 without a warning.

The new `RatingType` gives a rating's kind, and the new `Game::white_rating_type` and `Game::black_rating_type` fields hold it. A database stores it in the index, and reading a game sets it. PGN export writes each rating with its kind, as SCID does, such as `[WhiteUSCF "2400"]`, so games from SCID databases whose ratings aren't Elo ratings export differently. Before, every rating was exported as `WhiteElo` or `BlackElo`. An ordinary `WhiteElo` or `BlackElo` tag is now exported unless it has the same name as the player's rating tag.

Code that builds a `Game` with a struct expression must set the two new fields, or use `..Game::new()`.
