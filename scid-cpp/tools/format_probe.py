#!/usr/bin/env python3
"""Prints everything in a SCID database, reading the files only as
docs/SCID_FORMAT.md and the documents it links to describe them.

The output is the same as that of format_probe.cpp, which uses SCID's own
code. docs/SCID_FORMAT_AUDIT.md says how to run both. It checks every bound
the documents state, and that each entry's derived fields fit its game, and
exits with 1 at the first one that doesn't hold.

    python3 -I scid-cpp/tools/format_probe.py <database>
"""
import sys


class Corrupt(Exception):
    pass


class Reader:
    def __init__(self, data):
        self.data, self.pos = data, 0

    def take(self, n):
        if n < 0 or self.pos + n > len(self.data):
            raise Corrupt(f"needs {n} bytes at {self.pos} of {len(self.data)}")
        self.pos += n
        return self.data[self.pos - n : self.pos]

    def uint(self, n):
        return int.from_bytes(self.take(n), "big")

    def zstring(self):
        end = self.data.find(b"\0", self.pos)
        if end < 0:
            raise Corrupt(f"no NUL after {self.pos}")
        return self.take(end - self.pos + 1)[:-1]


def h(b):
    return b.hex() or "-"


def cstr(b):
    return b.split(b"\0")[0]


def date(v):
    return f"{v >> 9}.{v >> 5 & 15}.{v & 31}"


COUNTS = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 15, 20, 30, 40, 50]


def read_index(data):
    r = Reader(data)
    if r.take(8) != b"Scid.si\0":
        raise Corrupt("index magic")
    version, kind, games, autoload = r.uint(2), r.uint(4), r.uint(3), r.uint(3)
    if version != 400:
        raise Corrupt("index version")
    print(f"index version={version} type={kind} games={games} autoload={autoload} "
          f"description={h(cstr(r.take(108)))}")
    for i in range(6):
        print(f"custom {i + 1} {h(cstr(r.take(9)))}")
    entries = []
    for _ in range(games):
        e = Reader(r.take(47))
        offset, low, high, flags = e.uint(4), e.uint(2), e.uint(1), e.uint(2)
        players, white, black = e.uint(1), e.uint(2), e.uint(2)
        others, event, site, round_ = e.uint(1), e.uint(2), e.uint(2), e.uint(2)
        counts, eco, dates = e.uint(2), e.uint(2), e.uint(4)
        welo, belo, sig, moves = e.uint(2), e.uint(2), e.uint(4), e.uint(1)
        pawns = bytearray(e.take(9))
        moves |= pawns[0] >> 6 << 8
        pawns[0] &= 63
        length = low | (high >> 7) << 16
        names = [white | (players >> 4) << 16, black | (players & 15) << 16,
                 event | (others >> 5) << 16, site | (others >> 2 & 7) << 16,
                 round_ | (others & 3) << 16]
        day, coded = dates & 0xFFFFF, dates >> 20
        edate = 0
        if coded >> 9:
            edate = ((day >> 9) + (coded >> 9) - 4) << 9 | (coded & 511)
        c = [COUNTS[counts >> s & 15] for s in (0, 4, 8)]
        text = (f"offset={offset} length={length} flags={flags:04x} custom={high & 0x3F:02x} "
                f"names={','.join(map(str, names))} result={counts >> 12} "
                f"counts={c[0]},{c[1]},{c[2]} eco={eco} date={date(day)} event={date(edate)} "
                f"elo={min(welo & 4095, 4000)}/{welo >> 12},{min(belo & 4095, 4000)}/{belo >> 12} "
                f"material={sig & 0xFFFFFF:06x} line={sig >> 24} moves={moves} pawns={h(pawns)}")
        fields = (flags & 7, c, sig & 0xFFFFFF, moves, h(pawns))
        entries.append((offset, length, names, text, fields))
    if r.pos != len(data):
        raise Corrupt("bytes after the last entry")
    return entries


def read_names(data):
    r = Reader(data)
    if r.take(8) != b"Scid.sn\0":
        raise Corrupt("name magic")
    time = r.uint(4)
    counts = [r.uint(3) for _ in range(4)]
    most = [r.uint(3) for _ in range(4)]
    print(f"names time={time} " + " ".join(f"{c}:{m}" for c, m in zip(counts, most)))
    for kind in range(4):
        previous, seen = b"", set()
        for i in range(counts[kind]):
            ident = r.uint(3 if counts[kind] >= 65536 else 2)
            width = 3 if most[kind] >= 65536 else 2 if most[kind] >= 256 else 1
            frequency, length = r.uint(width), r.uint(1)
            prefix = r.uint(1) if i else 0
            if prefix > length or prefix > len(previous):
                raise Corrupt("prefix length")
            if ident >= counts[kind] or ident in seen:
                raise Corrupt("name ID")
            seen.add(ident)
            name = previous[:prefix] + r.take(length - prefix)
            if i and not order(previous, name):
                raise Corrupt("name order")
            previous = name
            print(f"name {kind} id={ident} frequency={frequency} {h(name)}")
    if r.pos != len(data):
        raise Corrupt("bytes after the last name")
    return counts


def order(a, b):
    """Whether name a comes before name b."""
    if a[:1] != b[:1]:
        return a[:1] < b[:1]
    signed = lambda s: [c - 256 if c > 127 else c for c in s + b"\0"]
    return signed(a) < signed(b)


KING = [0, -9, -8, -7, -1, 1, 7, 8, 9, -2, 2]
KNIGHT = [None, -17, -15, -10, -6, 6, 10, 15, 17]
PAWN = [7, 8, 9] * 5 + [16]
PROMOTION = [0] * 3 + [2] * 3 + [3] * 3 + [4] * 3 + [5] * 3 + [0]


class Position:
    def __init__(self, fen=None):
        self.board, self.lists, self.turn = {}, ([], []), 0
        if fen is None:
            for color, back, front in ((0, 0, 8), (1, 56, 48)):
                for f in (4, 0, 1, 2, 3, 5, 6, 7):
                    self.add(color, "RNBQKBNR"[f], back + f)
                for f in range(8):
                    self.add(color, "P", front + f)
            return
        fields = fen.decode().split()
        for row, text in enumerate(fields[0].split("/")):
            f = 0
            for c in text:
                if c.isdigit():
                    f += int(c)
                    continue
                color = int(c.islower())
                self.add(color, c.upper(), (7 - row) * 8 + f)
                squares = self.lists[color]
                if c.upper() == "K":
                    squares[0], squares[-1] = squares[-1], squares[0]
                f += 1
        self.turn = "wb".index(fields[1])

    def add(self, color, piece, square):
        self.board[square] = (color, piece)
        self.lists[color].append(square)

    def copy(self):
        p = Position.__new__(Position)
        p.board, p.lists, p.turn = dict(self.board), tuple(map(list, self.lists)), self.turn
        return p

    def decode(self, r, byte):
        number, value = byte >> 4, byte & 15
        if number >= len(self.lists[self.turn]):
            raise Corrupt("piece number")
        source = self.lists[self.turn][number]
        piece, file, promotion = self.board[source][1], source & 7, 0
        along = source & 7 | (value - 8) * 8 if value >= 8 else source & 56 | value
        if piece == "K":
            if value > 10:
                raise Corrupt("king value")
            target = source + KING[value]
        elif piece == "N":
            if not 1 <= value <= 8:
                raise Corrupt("knight value")
            target = source + KNIGHT[value]
        elif piece == "R" or piece == "Q" and value != file:
            target = along
        elif piece == "Q":
            second = r.uint(1)
            if not 64 <= second <= 127:
                raise Corrupt("queen byte")
            target = second - 64
        elif piece == "B":
            target = source + ((value & 7) - file) * (-7 if value >= 8 else 9)
        else:
            target = source + PAWN[value] * (-1 if self.turn else 1)
            promotion = PROMOTION[value]
        if not 0 <= target <= 63:
            raise Corrupt("target square")
        return source, target, promotion

    def play(self, source, target, promotion):
        us, them = self.turn, 1 - self.turn
        self.turn = them
        if source == target:
            return
        piece = self.board[source][1]
        taken = target
        if piece == "P" and (source ^ target) & 7 and target not in self.board:
            taken = target - 8 if us == 0 else target + 8
        if taken in self.board:
            squares = self.lists[them]
            number = squares.index(taken)
            squares[number] = squares[-1]
            squares.pop()
            del self.board[taken]
        self.move(us, source, target)
        if promotion:
            self.board[target] = (us, " KQRBNP"[promotion])
        if piece == "K" and abs(source - target) == 2:
            rook, to = (target + 1, target - 1) if target > source else (target - 2, target + 1)
            if self.board.get(rook) != (us, "R"):
                raise Corrupt("castling without a rook")
            self.move(us, rook, to)

    def move(self, color, source, target):
        self.lists[color][self.lists[color].index(source)] = target
        self.board[target] = self.board.pop(source)


TAGS = ["WhiteCountry", "BlackCountry", "Annotator", "PlyCount", "EventDate", "Opening",
        "Variation", "Setup", "Source", "SetUp"]


def code(count):
    """The code an entry stores for a count."""
    if count <= 12:
        return min(count, 10)
    return next(c for c, most in ((11, 17), (12, 24), (13, 34), (14, 44), (15, count)) if count <= most)


class MainLine:
    """Works out the entry's fields that SCID derives from the main line."""

    def __init__(self, set_up):
        self.set_up, self.half_moves, self.flags, self.material = set_up, 0, int(set_up), 0
        self.home = [8 + f for f in range(8)] + [48 + f for f in range(8)]
        self.left = []

    def add(self, position, promotion):
        self.half_moves += 1
        if promotion:
            self.flags |= 2 if promotion == 2 else 6
        for i, square in enumerate(self.home):
            if 15 - i not in self.left and position.board.get(square) != (i // 8, "P"):
                self.left.append(15 - i)

    def end(self, position):
        for color, shift in ((0, 12), (1, 0)):
            count = lambda p: sum(x == (color, p) for x in position.board.values())
            self.material |= count("P") << shift
            for i, p in enumerate("NBRQ"):
                self.material |= min(count(p), 3) << shift + 4 + 2 * i

    def fields(self, totals):
        pawns = [] if self.set_up else self.left
        pairs = (pawns + [0] * 16)[:16]
        data = bytes([len(pawns)] + [a << 4 | b for a, b in zip(pairs[::2], pairs[1::2])])
        return (self.flags, [COUNTS[code(t)] for t in totals], self.material,
                self.half_moves % 1024, h(data))


def read_game(record):
    r = Reader(record)
    out, comments, totals = [], [], [0, 0, 0]
    while code := r.uint(1):
        if code == 255:
            out.append(f"eventdate={date(r.uint(3))}")
            continue
        if code > 250:
            raise Corrupt("spare tag code")
        tag = TAGS[code - 241].encode() if code > 240 else r.take(code)
        out.append(f"[{h(tag)} {h(r.take(r.uint(1)))}]")
    flags = r.uint(1)
    out.append(f"flags={flags}")
    position = Position()
    if flags & 1:
        fen = r.zstring()
        out.append(f"fen={h(fen)}")
        position = Position(fen)

    def line(position, depth):
        before = None
        while (b := r.uint(1)) not in (14, 15):
            if b == 11:
                out.append(f"${r.uint(1)}")
                totals[2] += 1
            elif b == 12:
                comments.append(len(out))
                out.append(None)
            elif b == 13:
                if before is None:
                    raise Corrupt("variation before a move")
                out.append("(")
                totals[0] += 1
                line(before.copy(), depth + 1)
                out.append(")")
            else:
                move = position.decode(r, b)
                before = position.copy()
                position.play(*move)
                if depth == 0:
                    main.add(position, move[2])
                text = "null" if move[0] == move[1] else f"{move[0]}-{move[1]}"
                out.append(text + (f"={move[2]}" if move[2] else ""))
        if b != (14 if depth else 15):
            raise Corrupt("end marker")

    main = MainLine(flags & 1)
    line(position, 0)
    main.end(position)
    totals[1] = len(comments)
    for i in comments:
        out[i] = "{" + h(r.zstring()) + "}"
    if r.pos != len(record):
        raise Corrupt("bytes after the last comment")
    return f"end={r.pos} " + " ".join(out), main.fields(totals)


def main(path):
    with open(path + ".si4", "rb") as f:
        index = f.read()
    with open(path + ".sn4", "rb") as f:
        names = f.read()
    with open(path + ".sg4", "rb") as f:
        games = f.read()
    entries = read_index(index)
    counts = read_names(names)
    for n, (offset, length, ids, text, fields) in enumerate(entries, 1):
        print(f"entry {n} {text}")
        for kind, ident in zip((0, 0, 1, 2, 3), ids):
            if ident >= counts[kind]:
                raise Corrupt(f"entry {n} name ID")
        if length == 0 or offset + length > len(games):
            raise Corrupt(f"entry {n} record")
        if offset // 131072 != (offset + length - 1) // 131072:
            raise Corrupt(f"entry {n} crosses a block")
        game, derived = read_game(games[offset:offset + length])
        print(f"game {n} {game}")
        if derived != fields:
            raise Corrupt(f"entry {n} has {fields}, and its game {derived}")


if __name__ == "__main__":
    sys.setrecursionlimit(10000)
    try:
        main(sys.argv[1])
    except Corrupt as e:
        print(f"corrupt: {e}")
        sys.exit(1)
