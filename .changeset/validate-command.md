---
default: minor
---

# Report every damaged game in a SCID database

`scidtopgn validate mydb` reads every stored game, including logically deleted games, and reports each one it can't read as a warning, such as `SCID game 4 (deleted): Game data at byte 710, 166 bytes long, is past the end of the game file`. A final line gives the number of games checked and the number damaged. It writes the report to stderr and to the log given with `--log`, and doesn't change the database. It exits with 0 if no game is damaged, 1 if any is, and 2 if the check couldn't finish, for example because the database couldn't be opened.
