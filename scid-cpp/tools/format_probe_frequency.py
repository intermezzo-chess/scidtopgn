#!/usr/bin/env python3
"""Writes a database whose next game takes a player's frequency to 2^24, from
one that holds a game of a player against themselves. docs/SCID_FORMAT_AUDIT.md
says how to run it. The index file is 394 MB.

    python3 -I scid-cpp/tools/format_probe_frequency.py <database> <new database>
"""
import sys

source, out = sys.argv[1:]
HEADER, ENTRY = 182, 47
GAMES = (1 << 23) - 1

index = open(f"{source}.si4", "rb").read()
header = index[:14] + GAMES.to_bytes(3, "big") + index[17:HEADER]
entry = index[HEADER:HEADER + ENTRY]
with open(f"{out}.si4", "wb") as f:
    f.write(header)
    for _ in range(GAMES // 65536):
        f.write(entry * 65536)
    f.write(entry * (GAMES % 65536))
with open(f"{out}.sg4", "wb") as f:
    f.write(open(f"{source}.sg4", "rb").read())

uses = [2 * GAMES, GAMES, GAMES, GAMES]
data = b"Scid.sn\0" + bytes(4) + b"\0\0\1" * 4
data += b"".join(n.to_bytes(3, "big") for n in uses)
for name, n in zip((b"A", b"E", b"S", b"1"), uses):
    data += bytes(2) + n.to_bytes(3, "big") + bytes([len(name)]) + name
with open(f"{out}.sn4", "wb") as f:
    f.write(data)
