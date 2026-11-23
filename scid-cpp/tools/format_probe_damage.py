#!/usr/bin/env python3
"""Writes damaged and unusual copies of tests/data/one, one per case of
docs/SCID_FORMAT_AUDIT.md, into a directory.

    python3 -I scid-cpp/tools/format_probe_damage.py tests/data <directory>
"""
import sys

source, out = sys.argv[1:]
FILES = {e: open(f"{source}/one.{e}", "rb").read() for e in ("si4", "sn4", "sg4")}
ENTRY = 182


def make(name, **files):
    for extension, data in {**FILES, **files}.items():
        with open(f"{out}/{name}.{extension}", "wb") as f:
            f.write(data)


def patch(offset, data):
    return FILES["si4"][:offset] + data + FILES["si4"][offset + len(data):]


def record(name, data):
    make(name, si4=patch(ENTRY + 4, len(data).to_bytes(2, "big")), sg4=data)


def names(players, most=b"\0\0\1", width=1):
    data = b"Scid.sn\0" + bytes(4) + len(players).to_bytes(3, "big") + b"\0\0\1" * 3
    data += most + b"\0\0\1" * 3
    for i, n in enumerate(players):
        data += i.to_bytes(2, "big") + (1).to_bytes(width, "big") + bytes([len(n)])
        data += (b"\0" if i else b"") + n
    for n in (b"E", b"S", b"1"):
        data += b"\0\0\1" + bytes([len(n)]) + n
    return data


# The index file.
for version in (300, 399, 401):
    make(f"v{version}", si4=patch(8, version.to_bytes(2, "big")))
make("trailing", si4=FILES["si4"] + bytes(3))
make("elo4095", si4=patch(ENTRY + 29, (4095).to_bytes(2, "big")))
make("badname", si4=patch(ENTRY + 10, (9).to_bytes(2, "big")))

# The name file.
make("order", sn4=names([b"Ab", b"Aa"]))
make("duplicate", sn4=names([b"Aa", b"Aa"]))
make("firstbyte", sn4=names([b"B", b"A"]))
make("signed", sn4=names([b"A\xc3\xa9", b"Ab"]))
make("unsigned", sn4=names([b"Ab", b"A\xc3\xa9"]))
make("maxfreq", sn4=names([b"Aa", b"Ab"], (256).to_bytes(3, "big"), 2))

# Where records are.
start = 131072 - 10
make("cross", si4=patch(ENTRY, start.to_bytes(4, "big")), sg4=bytes(start) + FILES["sg4"])
make("truncated", sg4=FILES["sg4"][:20])
record("zero", b"")

# Records. The tags end and the flags are 0, then 1. e4.
E4 = b"\0\0\xcf"
date = (2021 << 9 | 3 << 5 | 7).to_bytes(3, "big")
record("compactdate", b"\xff" + date + E4 + b"\x0f")
record("spare", b"\xfb\1x" + E4 + b"\x0f")
record("flags8", b"\0\xf8\xcf\x0f")
record("e4", E4 + b"\x0f")
record("d4", b"\0\0\xbf\x0f")
record("knight15", b"\0\0\x6f\x0f")
record("queen", b"\0\0\x43\x10\x0f")
record("illegal", b"\0\0\x18\x0f")
record("nag0", E4 + b"\x0b\0\x0f")
record("nag9", E4 + b"".join(bytes([11, n]) for n in range(1, 10)) + b"\x0f")
record("twocomments", E4 + b"\x0c\x0c\x0fa\0")
record("commentafter", E4 + b"\x0d\xbf\x0e\x0c\x0fa\0")
record("nocomment", E4 + b"\x0c\x0f")
record("varfirst", b"\0\0\x0d\xcf\x0e\x0f")
record("emptyvar", E4 + b"\x0d\x0e\x0f")
record("endvar", E4 + b"\x0e")
record("noend", E4)
record("after", E4 + b"\x0fzz")
