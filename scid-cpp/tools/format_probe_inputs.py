#!/usr/bin/env python3
"""Writes the PGN files that docs/SCID_FORMAT_AUDIT.md's probes import with
pgnscid, into a directory. The same files are written each time.

    python3 -I scid-cpp/tools/format_probe_inputs.py <directory>
"""
import os
import random
import sys

out = sys.argv[1]


def write(name, games):
    with open(os.path.join(out, name + ".pgn"), "w", encoding="utf-8") as f:
        for tags, moves in games:
            given = dict(tags)
            base = {"Event": "E", "Site": "S", "Date": "2020.06.15", "Round": "1",
                    "White": "A", "Black": "B", "Result": "*"}
            for key, value in {**base, **given}.items():
                f.write(f'[{key} "{value}"]\n')
            f.write(f"\n{moves} *\n\n")


# Name counts and frequencies. Each has N games, and so N names of the kinds
# given below, and frequency N for the names every game has.
for n in (255, 256, 65535, 65536):
    write(f"names{n}", [({"Event": f"Event {i:05}"}, "1. e4") for i in range(n)])
# Every kind of name has IDs above 65,535.
write("names66000", [({"Event": f"E{i}", "Site": f"S{i}", "Round": f"R{i}", "White": f"W{i}",
                       "Black": f"B{i}"}, "1. d4") for i in range(66000)])
# Names: empty, 255 bytes, prefixes that end inside a character.
long = "x" * 254
write("namecases", [({"White": w, "Black": b, "Event": e}, "1. e4") for w, b, e in [
    ("Aé", "Aê", ""), (long + "y", long + "z", "e" * 255), ("É", "Z", "Zed"),
    ("Bl", "Blü", "é"), ("Bla", "Blz", "z"), ("a\x01b", "ab", "a\tb")]])

# Dates: unknown parts, limits, and event dates around three years away.
dates = [("????.??.??", "????.??.??"), ("2020.??.??", "2020.??.??"), ("2020.06.??", "2021.02.??"),
         ("0001.01.01", "0001.01.01"), ("2047.12.31", "2047.12.31"), ("2048.01.01", "2047.01.01"),
         ("2020.13.01", "2020.12.32"), ("2020.12.32", ""), ("0002.01.01", "0001.01.01"),
         ("0003.01.01", "0001.01.01"), ("2022.12.19", "2022.08.10")]
dates += [("2020.06.15", f"{y}.01.02") for y in range(2015, 2026)]
write("dates", [({"Date": d, **({"EventDate": e} if e else {})}, "1. e4") for d, e in dates])

# Ratings and their types, ECO codes, and results.
kinds = ["Elo", "Rating", "Rapid", "ICCF", "USCF", "DWZ", "ECF"]
ratings = [({"White" + k: 1000 + i, "Black" + k: 4000 - i}, "1. e4") for i, k in enumerate(kinds)]
ratings += [({"WhiteElo": r, "BlackElo": 0}, "1. e4") for r in (1, 3999, 4000, 4001, 4095, 4096)]
ecos = ["A00", "A00a", "A00a1", "A00z4", "A01", "B12", "B12c", "C42b3", "E99", "E99z4"]
ratings += [({"ECO": e}, "1. e4") for e in ecos]
write("ratings", ratings)

# Counts of variations, comments, and NAGs, across each coded value.
shuffle = ["Nf3", "Nf6", "Ng1", "Ng8"]


def counted(n):
    text = []
    for i in range(max(n, 1)):
        text.append(shuffle[i % 4] + (f" $1 {{c{i}}} ({'h3' if i % 2 == 0 else 'h6'})" if i < n else ""))
    return " ".join(text)


write("counts", [({"Round": n}, counted(n)) for n in range(56)])
# Half-move counts around 8 and 10 bits.
write("moves", [({"Round": n}, " ".join(shuffle[i % 4] for i in range(n)))
                for n in (0, 1, 255, 256, 1023, 1024, 1025, 2049)])

# Comments and NAGs before, between, and after moves and variations.
write("comments", [({}, m) for m in [
    "{start} 1. e4 {a} $1 $2 (1. d4 {b} (1. c4 {c} 1... c5 $3) 1... d5) ({before} 1. Nf3 {d}) 1... e5",
    "1. e4 $1 $2 $3 $4 $5 $6 $7 $8 $9", "1. e4 (1. d4) {after}", "1. e4 {} 1... e5", "{only}", "",
    "1. e4 -- 2. d4 (2. -- e5)"]])
# Tags: each common code, others, and the longest.
common = ["WhiteCountry", "BlackCountry", "Annotator", "PlyCount", "Opening", "Variation", "Setup",
          "Source"]
write("tags", [({t: t.lower() for t in common}, "1. e4"), ({"Custom": "v", "Empty": ""}, "1. e4"),
               ({"T" * 240: "v" * 254}, "1. e4"), ({"T" * 241: "v"}, "1. e4"),
               ({"SetUp": "1", "FEN": "8/8/8/3k4/8/8/4P3/4K2R w K - 5 40"}, "1. O-O Ke4")])
# SCID holds 40 tags, and pgnscid crashes at the 41st.
for n in (40, 41):
    write(f"tags{n}", [({f"Extra{i:02}": "v" for i in range(n)}, "1. e4")])
# SCID writes a 255-byte tag value, but overruns its buffer when it reads one.
write("tag255", [({"Custom": "v" * 255}, "1. e4")])


# Records that fill and cross blocks. A record of `size` bytes has one move,
# comments of at most 15,000 bytes, and no tags: 5 bytes before the comments
# (end of tags, flags, move, comment marker, end) and a NUL after each.
def sized(size):
    moves, left = [], size - 3
    i = 0
    while left > 0:
        n = min(left - 3, 15000)
        if 0 < left - 3 - n < 4:
            n -= 10
        moves.append(shuffle[i % 4] + " {" + "x" * n + "}")
        left -= n + 3
        i += 1
    return " ".join(moves)


write("blocks", [({"Round": n}, sized(n)) for n in (60000, 60000, 11071, 2, 100, 131071, 100, 131070)])
for n in (131071, 131072, 131073):
    write(f"size{n}", [({"Round": n}, sized(n)), ({}, "1. e4")])


# Random legal games, from the standard position and from FENs.
def attacked(board, square, by):
    f, r = square % 8, square // 8

    def at(df, dr):
        x, y = f + df, r + dr
        return board.get(y * 8 + x) if 0 <= x < 8 and 0 <= y < 8 else None

    back = -1 if by == 0 else 1
    if any(at(d, back) == (by, "P") for d in (-1, 1)):
        return True
    if any(at(a, b) == (by, "N") for a, b in KNIGHT):
        return True
    if any(at(a, b) == (by, "K") for a, b in ALL):
        return True
    for a, b in ALL:
        for n in range(1, 8):
            x, y = f + a * n, r + b * n
            if not (0 <= x < 8 and 0 <= y < 8):
                break
            p = board.get(y * 8 + x)
            if p:
                if p[0] == by and p[1] in ("QR" if a == 0 or b == 0 else "QB"):
                    return True
                break
    return False


KNIGHT = [(1, 2), (2, 1), (-1, 2), (-2, 1), (1, -2), (2, -1), (-1, -2), (-2, -1)]
STRAIGHT, DIAGONAL = [(1, 0), (-1, 0), (0, 1), (0, -1)], [(1, 1), (1, -1), (-1, 1), (-1, -1)]
ALL = STRAIGHT + DIAGONAL
STEPS = {"N": KNIGHT, "K": ALL, "R": STRAIGHT, "B": DIAGONAL, "Q": ALL}


def name(s):
    return "abcdefgh"[s % 8] + str(s // 8 + 1)


def play(board, move, us):
    b = dict(board)
    s, t, promo, taken = move
    b.pop(taken, None)
    b[t] = b.pop(s)
    if promo:
        b[t] = (us, promo)
    if b[t][1] == "K" and abs(s - t) == 2:
        rook, to = (t + 1, t - 1) if t > s else (t - 2, t + 1)
        b[to] = b.pop(rook)
    return b


def legal(board, us, rights, ep):
    moves = []
    for s, (c, p) in list(board.items()):
        if c != us:
            continue
        f, r = s % 8, s // 8
        if p == "P":
            up, start, last = (1, 1, 7) if us == 0 else (-1, 6, 0)
            targets = []
            if s + 8 * up not in board:
                targets.append((s + 8 * up, s + 8 * up))
                if r == start and s + 16 * up not in board:
                    targets.append((s + 16 * up, s + 16 * up))
            for d in (-1, 1):
                if 0 <= f + d < 8:
                    t = s + 8 * up + d
                    if t in board and board[t][0] != us:
                        targets.append((t, t))
                    elif t == ep:
                        targets.append((t, t - 8 * up))
            for t, taken in targets:
                for promo in "QRBN" if t // 8 == last else [""]:
                    moves.append((s, t, promo, taken))
            continue
        for a, b in STEPS[p]:
            for n in range(1, 2 if p in "NK" else 8):
                x, y = f + a * n, r + b * n
                if not (0 <= x < 8 and 0 <= y < 8):
                    break
                t = y * 8 + x
                if t in board and board[t][0] == us:
                    break
                moves.append((s, t, "", t))
                if t in board:
                    break
        if p == "K" and not attacked(board, s, 1 - us):
            for side, d, empty in (("K", 1, [1, 2]), ("Q", -1, [1, 2, 3])):
                if (us, side) in rights and all(s + d * e not in board for e in empty) \
                        and not any(attacked(board, s + d * e, 1 - us) for e in (1, 2)):
                    moves.append((s, s + 2 * d, "", s + 2 * d))
    result = []
    for m in moves:
        b = play(board, m, us)
        king = next(s for s, p in b.items() if p == (us, "K"))
        if not attacked(b, king, 1 - us):
            result.append(m)
    return result


def san(board, m):
    s, t, promo, taken = m
    p = board[s][1]
    if p == "K" and abs(s - t) == 2:
        return "O-O" if t > s else "O-O-O"
    x = "x" if taken in board else ""
    return (p if p != "P" else "") + name(s) + x + name(t) + ("=" + promo if promo else "")


def state(fen):
    board = {}
    fields = fen.split()
    for row, text in enumerate(fields[0].split("/")):
        f = 0
        for ch in text:
            if ch.isdigit():
                f += int(ch)
            else:
                board[(7 - row) * 8 + f] = (int(ch.islower()), ch.upper())
                f += 1
    rights = {(int(c.islower()), c.upper()) for c in fields[2] if c != "-"}
    return board, "wb".index(fields[1]), rights, None


def line(rng, position, length, depth):
    board, us, rights, ep = position
    text = []
    for _ in range(length):
        moves = legal(board, us, rights, ep)
        if not moves:
            break
        special = [m for m in moves if m[2] or m[3] != m[1] or m[3] in board
                   or board[m[0]][1] == "K" and abs(m[0] - m[1]) == 2]
        m = rng.choice(special if special and rng.random() < 0.3 else moves)
        text.append(san(board, m))
        if rng.random() < 0.03:
            text.append(f"${rng.randrange(1, 256)}")
        if rng.random() < 0.03:
            text.append("{" + rng.choice(["a", "é", "comment " * 3]) + "}")
        if depth < 3 and rng.random() < 0.04:
            inner = line(rng, (board, us, rights, ep), rng.randrange(1, 12), depth + 1)
            text.append(f"({inner})")
        p = board[m[0]][1]
        ep = (m[0] + m[1]) // 2 if p == "P" and abs(m[0] - m[1]) == 16 else None
        board = play(board, m, us)
        lost = {0: (0, "Q"), 7: (0, "K"), 56: (1, "Q"), 63: (1, "K")}
        rights = {r for r in rights if not (p == "K" and r[0] == us)}
        rights -= {lost[q] for q in (m[0], m[1]) if q in lost}
        us = 1 - us
    return " ".join(text)


rng = random.Random(145)
START = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
FENS = ["r3k2r/pppq1ppp/2n5/8/8/2N5/PPPQ1PPP/R3K2R w KQkq - 0 1",
        "8/PPPk4/8/8/8/8/4Kppp/8 b - - 0 1", "4k3/8/8/pPpPpPpP/PpPpPpPp/8/8/4K3 w - - 0 1",
        "QQQQ4/6k1/8/8/8/8/8/K1qqqq2 w - - 0 30", "1n2k3/8/r2q3b/8/8/B3Q2R/8/2N1K3 b - - 12 34"]
write("random", [({"Round": i}, line(rng, state(START), rng.randrange(10, 300), 0))
                 for i in range(400)]
      + [({"Round": i, "SetUp": "1", "FEN": FENS[i % len(FENS)]},
          line(rng, state(FENS[i % len(FENS)]), rng.randrange(1, 120), 0)) for i in range(200)])
