//! Stored-line codes: the opening line an index entry records for its game,
//! which SCID vs. PC shows as the game list's opening moves and uses to skip
//! games in tree searches without decoding them.
//!
//! The table of lines is copied from `storedLineText` in SCID vs. PC 4.26's
//! `src/stored.cpp`, Copyright (c) 2001 Shane Hudson, distributed under the
//! GNU General Public License (see LICENSE).

use std::sync::LazyLock;

use shakmaty::san::SanPlus;
use shakmaty::{Chess, Color, Move, Position, Square};

use crate::game::{Game, Marker};
use crate::mov::{destination, GameMove};

/// The number of half-moves in the longest line.
const MAX_PLIES: usize = 21;

/// SCID's stored lines. A line's code is its position in the table, from 1.
const LINES: [&str; 254] = [
    "1.b3",
    "1.c4",
    "1.c4 c5",
    "1.c4 c5 2.Nf3",
    "1.c4 e5",
    "1.c4 e5 2.Nc3",
    "1.c4 e5 2.Nc3 Nf6",
    "1.c4 e6",
    "1.c4 e6 2.Nf3",
    "1.c4 g6",
    "1.c4 Nf6",
    "1.c4 Nf6 2.Nc3",
    "1.c4 Nf6 2.Nc3 e6",
    "1.c4 Nf6 2.Nc3 g6",
    "1.d4",
    "1.d4 d5",
    "1.d4 d5 2.c4",
    "1.d4 d5 2.c4 c6",
    "1.d4 d5 2.c4 c6 3.Nc3",
    "1.d4 d5 2.c4 c6 3.Nc3 Nf6",
    "1.d4 d5 2.c4 c6 3.Nc3 Nf6 4.Nf3",
    "1.d4 d5 2.c4 c6 3.Nf3",
    "1.d4 d5 2.c4 c6 3.Nf3 Nf6",
    "1.d4 d5 2.c4 c6 3.Nf3 Nf6 4.Nc3",
    "1.d4 d5 2.c4 c6 3.Nf3 Nf6 4.Nc3 e6",
    "1.d4 d5 2.c4 dxc4",
    "1.d4 d5 2.c4 dxc4 3.Nf3",
    "1.d4 d5 2.c4 dxc4 3.Nf3 Nf6",
    "1.d4 d5 2.c4 e6",
    "1.d4 d5 2.c4 e6 3.Nc3",
    "1.d4 d5 2.c4 e6 3.Nc3 c6",
    "1.d4 d5 2.c4 e6 3.Nc3 Nf6",
    "1.d4 d5 2.c4 e6 3.Nf3",
    "1.d4 d5 2.Nf3",
    "1.d4 d5 2.Nf3 Nf6",
    "1.d4 d5 2.Nf3 Nf6 3.c4",
    "1.d4 d5 2.Nf3 Nf6 3.c4 c6",
    "1.d4 d5 2.Nf3 Nf6 3.c4 e6",
    "1.d4 d6",
    "1.d4 d6 2.Nf3",
    "1.d4 e6",
    "1.d4 e6 2.c4",
    "1.d4 e6 2.c4 Nf6",
    "1.d4 f5",
    "1.d4 f5 2.g3 Nf6 3.Bg2",
    "1.d4 g6",
    "1.d4 g6 2.c4 Bg7",
    "1.d4 Nf6",
    "1.d4 Nf6 2.Bg5",
    "1.d4 Nf6 2.Bg5 Ne4",
    "1.d4 Nf6 2.c4",
    "1.d4 Nf6 2.c4 c5",
    "1.d4 Nf6 2.c4 c5 3.d5",
    "1.d4 Nf6 2.c4 c5 3.d5 b5",
    "1.d4 Nf6 2.c4 c5 3.d5 b5 4.cxb5 a6",
    "1.d4 Nf6 2.c4 e6 3.g3",
    "1.d4 Nf6 2.c4 e6 3.g3 d5",
    "1.d4 Nf6 2.c4 e6 3.Nc3",
    "1.d4 Nf6 2.c4 e6 3.Nc3 Bb4",
    "1.d4 Nf6 2.c4 e6 3.Nc3 Bb4 4.e3",
    "1.d4 Nf6 2.c4 e6 3.Nc3 Bb4 4.e3 O-O",
    "1.d4 Nf6 2.c4 e6 3.Nc3 Bb4 4.Qc2",
    "1.d4 Nf6 2.c4 e6 3.Nc3 Bb4 4.Qc2 O-O",
    "1.d4 Nf6 2.c4 e6 3.Nc3 Bb4 4.Qc2 O-O 5.a3 Bxc3+ 6.Qxc3",
    "1.d4 Nf6 2.c4 e6 3.Nc3 d5",
    "1.d4 Nf6 2.c4 e6 3.Nf3",
    "1.d4 Nf6 2.c4 e6 3.Nf3 b6",
    "1.d4 Nf6 2.c4 e6 3.Nf3 b6 4.a3",
    "1.d4 Nf6 2.c4 e6 3.Nf3 b6 4.g3",
    "1.d4 Nf6 2.c4 e6 3.Nf3 b6 4.g3 Ba6",
    "1.d4 Nf6 2.c4 e6 3.Nf3 Bb4+",
    "1.d4 Nf6 2.c4 e6 3.Nf3 d5",
    "1.d4 Nf6 2.c4 e6 3.Nf3 d5 4.Nc3",
    "1.d4 Nf6 2.c4 g6",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.Be2 O-O",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.Be2 O-O 6.Nf3",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.f3",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.f3 O-O",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.f3 O-O 6.Be3",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.Nf3 O-O",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.Nf3 O-O 6.Be2",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.Nf3 O-O 6.Be2 e5",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.Nf3 O-O 6.Be2 e5 7.O-O",
    "1.d4 Nf6 2.c4 g6 3.Nc3 Bg7 4.e4 d6 5.Nf3 O-O 6.Be2 e5 7.O-O Nc6 8.d5 Ne7",
    "1.d4 Nf6 2.c4 g6 3.Nc3 d5",
    "1.d4 Nf6 2.c4 g6 3.Nc3 d5 4.Nf3",
    "1.d4 Nf6 2.c4 g6 3.Nc3 d5 4.cxd5 Nxd5",
    "1.d4 Nf6 2.c4 g6 3.Nc3 d5 4.cxd5 Nxd5 5.e4 Nxc3 6.bxc3 Bg7",
    "1.d4 Nf6 2.Nf3",
    "1.d4 Nf6 2.Nf3 c5",
    "1.d4 Nf6 2.Nf3 d5",
    "1.d4 Nf6 2.Nf3 e6",
    "1.d4 Nf6 2.Nf3 e6 3.Bg5",
    "1.d4 Nf6 2.Nf3 e6 3.c4",
    "1.d4 Nf6 2.Nf3 g6",
    "1.d4 Nf6 2.Nf3 g6 3.Bg5",
    "1.d4 Nf6 2.Nf3 g6 3.c4",
    "1.d4 Nf6 2.Nf3 g6 3.c4 Bg7",
    "1.d4 Nf6 2.Nf3 g6 3.c4 Bg7 4.Nc3",
    "1.d4 Nf6 2.Nf3 g6 3.c4 Bg7 4.Nc3 O-O",
    "1.d4 Nf6 2.Nf3 g6 3.g3",
    "1.d4 Nf6 2.Nf3 g6 3.g3 Bg7 4.Bg2",
    "1.e4",
    "1.e4 c5",
    "1.e4 c5 2.c3",
    "1.e4 c5 2.c3 d5 3.exd5 Qxd5 4.d4",
    "1.e4 c5 2.c3 d5 3.exd5 Qxd5 4.d4 Nf6",
    "1.e4 c5 2.c3 Nf6 3.e5 Nd5",
    "1.e4 c5 2.c3 Nf6 3.e5 Nd5 4.d4 cxd4",
    "1.e4 c5 2.d4 cxd4",
    "1.e4 c5 2.Nc3",
    "1.e4 c5 2.Nc3 Nc6",
    "1.e4 c5 2.Nc3 Nc6 3.g3",
    "1.e4 c5 2.Nc3 Nc6 3.g3 g6",
    "1.e4 c5 2.Nc3 Nc6 3.g3 g6 4.Bg2 Bg7",
    "1.e4 c5 2.Nc3 Nc6 3.g3 g6 4.Bg2 Bg7 5.d3",
    "1.e4 c5 2.Nf3",
    "1.e4 c5 2.Nf3 d6",
    "1.e4 c5 2.Nf3 d6 3.Bb5+",
    "1.e4 c5 2.Nf3 d6 3.d4",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 a6",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 a6 6.Bc4",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 a6 6.Be2",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 a6 6.Be3",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 a6 6.Bg5",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 a6 6.Bg5 e6",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 g6",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 g6 6.Be3 Bg7 7.f3",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 g6 6.Be3 Bg7 7.f3 O-O",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 Nc6",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 Nc6 6.Bg5",
    "1.e4 c5 2.Nf3 d6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 Nc6 6.Bg5 e6 7.Qd2",
    "1.e4 c5 2.Nf3 e6",
    "1.e4 c5 2.Nf3 e6 3.d3",
    "1.e4 c5 2.Nf3 e6 3.d4 cxd4 4.Nxd4",
    "1.e4 c5 2.Nf3 e6 3.d4 cxd4 4.Nxd4 a6",
    "1.e4 c5 2.Nf3 e6 3.d4 cxd4 4.Nxd4 a6 5.Bd3",
    "1.e4 c5 2.Nf3 e6 3.d4 cxd4 4.Nxd4 Nc6",
    "1.e4 c5 2.Nf3 e6 3.d4 cxd4 4.Nxd4 Nc6 5.Nc3",
    "1.e4 c5 2.Nf3 e6 3.d4 cxd4 4.Nxd4 Nc6 5.Nc3 Qc7",
    "1.e4 c5 2.Nf3 e6 3.d4 cxd4 4.Nxd4 Nf6",
    "1.e4 c5 2.Nf3 e6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3",
    "1.e4 c5 2.Nf3 e6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 d6",
    "1.e4 c5 2.Nf3 Nc6",
    "1.e4 c5 2.Nf3 Nc6 3.Bb5",
    "1.e4 c5 2.Nf3 Nc6 3.Bb5 g6",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 e5",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 g6",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 d6",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 d6 6.Bg5",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 e5",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 e5 6.Ndb5 d6",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 e5 6.Ndb5 d6 7.Bg5 a6",
    "1.e4 c5 2.Nf3 Nc6 3.d4 cxd4 4.Nxd4 Nf6 5.Nc3 e5 6.Ndb5 d6 7.Bg5 a6 8.Na3 b5",
    "1.e4 c6",
    "1.e4 c6 2.d4 d5",
    "1.e4 c6 2.d4 d5 3.e5",
    "1.e4 c6 2.d4 d5 3.e5 Bf5",
    "1.e4 c6 2.d4 d5 3.exd5 cxd5",
    "1.e4 c6 2.d4 d5 3.exd5 cxd5 4.c4 Nf6 5.Nc3",
    "1.e4 c6 2.d4 d5 3.Nc3",
    "1.e4 c6 2.d4 d5 3.Nc3 dxe4 4.Nxe4",
    "1.e4 c6 2.d4 d5 3.Nd2 dxe4 4.Nxe4",
    "1.e4 d5 2.exd5 Nf6",
    "1.e4 d5 2.exd5 Qxd5",
    "1.e4 d5 2.exd5 Qxd5 3.Nc3",
    "1.e4 d5 2.exd5 Qxd5 3.Nc3 Qa5",
    "1.e4 d6",
    "1.e4 d6 2.d4",
    "1.e4 d6 2.d4 Nf6",
    "1.e4 d6 2.d4 Nf6 3.Nc3",
    "1.e4 d6 2.d4 Nf6 3.Nc3 g6",
    "1.e4 d6 2.d4 Nf6 3.Nc3 g6 4.f4 Bg7 5.Nf3",
    "1.e4 d6 2.d4 Nf6 3.Nc3 g6 4.Nf3 Bg7",
    "1.e4 e5",
    "1.e4 e5 2.f4",
    "1.e4 e5 2.Nc3",
    "1.e4 e5 2.Nf3",
    "1.e4 e5 2.Nf3 Nc6",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6 5.O-O",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6 5.O-O b5 6.Bb3",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6 5.O-O Be7",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6 5.O-O Be7 6.Re1 b5 7.Bb3 d6",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6 5.O-O Be7 6.Re1 b5 7.Bb3 d6 8.c3 O-O",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6 5.O-O Be7 6.Re1 b5 7.Bb3 d6 8.c3 O-O 9.h3",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6 5.O-O Be7 6.Re1 b5 7.Bb3 d6 8.c3 O-O 9.h3 Na5 10.Bc2 c5 11.d4",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 a6 4.Ba4 Nf6 5.O-O Be7 6.Re1 b5 7.Bb3 O-O",
    "1.e4 e5 2.Nf3 Nc6 3.Bb5 Nf6",
    "1.e4 e5 2.Nf3 Nc6 3.Bc4",
    "1.e4 e5 2.Nf3 Nc6 3.Bc4 Nf6",
    "1.e4 e5 2.Nf3 Nc6 3.d4 exd4",
    "1.e4 e5 2.Nf3 Nc6 3.d4 exd4 4.Nxd4",
    "1.e4 e5 2.Nf3 Nc6 3.Nc3",
    "1.e4 e5 2.Nf3 Nc6 3.Nc3 Nf6",
    "1.e4 e5 2.Nf3 Nf6",
    "1.e4 e5 2.Nf3 Nf6 3.Nxe5 d6",
    "1.e4 e6",
    "1.e4 e6 2.d3",
    "1.e4 e6 2.d4",
    "1.e4 e6 2.d4 d5",
    "1.e4 e6 2.d4 d5 3.e5 c5",
    "1.e4 e6 2.d4 d5 3.e5 c5 4.c3",
    "1.e4 e6 2.d4 d5 3.Nc3",
    "1.e4 e6 2.d4 d5 3.Nc3 Bb4",
    "1.e4 e6 2.d4 d5 3.Nc3 Bb4 4.e5",
    "1.e4 e6 2.d4 d5 3.Nc3 Bb4 4.e5 c5",
    "1.e4 e6 2.d4 d5 3.Nc3 Bb4 4.e5 c5 5.a3 Bxc3+ 6.bxc3",
    "1.e4 e6 2.d4 d5 3.Nc3 Nf6",
    "1.e4 e6 2.d4 d5 3.Nc3 Nf6 4.Bg5",
    "1.e4 e6 2.d4 d5 3.Nd2",
    "1.e4 e6 2.d4 d5 3.Nd2 c5",
    "1.e4 e6 2.d4 d5 3.Nd2 Nf6",
    "1.e4 e6 2.d4 d5 3.Nd2 Nf6 4.e5 Nfd7",
    "1.e4 e6 2.d4 d5 3.Nd2 Nf6 4.e5 Nfd7 5.Bd3 c5 6.c3 Nc6 7.Ne2",
    "1.e4 g6",
    "1.e4 g6 2.d4",
    "1.e4 g6 2.d4 Bg7",
    "1.e4 g6 2.d4 Bg7 3.Nc3",
    "1.e4 g6 2.d4 Bg7 3.Nc3 d6",
    "1.e4 Nf6",
    "1.e4 Nf6 2.e5 Nd5",
    "1.e4 Nf6 2.e5 Nd5 3.d4 d6",
    "1.e4 Nf6 2.e5 Nd5 3.d4 d6 4.Nf3",
    "1.f4",
    "1.g3",
    "1.Nf3",
    "1.Nf3 c5",
    "1.Nf3 c5 2.c4",
    "1.Nf3 d5",
    "1.Nf3 d5 2.c4",
    "1.Nf3 d5 2.d4",
    "1.Nf3 d5 2.g3",
    "1.Nf3 g6",
    "1.Nf3 Nf6",
    "1.Nf3 Nf6 2.c4",
    "1.Nf3 Nf6 2.c4 c5",
    "1.Nf3 Nf6 2.c4 e6",
    "1.Nf3 Nf6 2.c4 g6",
    "1.Nf3 Nf6 2.c4 g6 3.Nc3",
    "1.Nf3 Nf6 2.g3",
    "1.Nf3 Nf6 2.g3 g6",
];

/// Each line's moves as SCID compares them: their from and to squares.
static LINE_SQUARES: LazyLock<Vec<Vec<(Square, Square)>>> =
    LazyLock::new(|| LINES.iter().map(|line| line_squares(line)).collect());

/// The from and to squares of `line`'s moves, played from the standard
/// start.
fn line_squares(line: &str) -> Vec<(Square, Square)> {
    let mut chess = Chess::default();
    line.split_whitespace()
        .map(|token| {
            let san = token.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
            let m = SanPlus::from_ascii(san.as_bytes())
                .ok()
                .and_then(|san| san.san.to_move(&chess).ok())
                .unwrap_or_else(|| panic!("stored line {line:?} has an illegal move {token:?}"));
            let squares = squares(m, chess.turn()).expect("a stored line has no drops");
            chess.play_unchecked(m);
            squares
        })
        .collect()
}

/// The from and to squares SCID records for `m`, played by `color`: the
/// king's for castling.
fn squares(m: Move, color: Color) -> Option<(Square, Square)> {
    Some((m.from()?, destination(m, color)))
}

/// Returns `game`'s stored-line code, as SCID's `Game::Encode` sets it: the
/// code of the longest line whose every move matches the main line's, in
/// order, by from and to squares. Of lines equally long, the first wins. The
/// code is 0 if no line matches, or if the game starts from a FEN.
pub(crate) fn code(game: &Game) -> u8 {
    if game.start_position.is_some() {
        return 0;
    }
    let main_line = main_line_squares(game);
    let mut code = 0;
    let mut longest = 0;
    for (line_code, line) in (1..=u8::MAX).zip(LINE_SQUARES.iter()) {
        if line.len() > longest && main_line.starts_with(line) {
            longest = line.len();
            code = line_code;
        }
    }
    code
}

/// The from and to squares of the first moves of `game`'s main line, up to
/// the length of the longest line, or up to its first null move, which no
/// line's move matches.
fn main_line_squares(game: &Game) -> Vec<(Square, Square)> {
    let mut main_line = Vec::with_capacity(MAX_PLIES);
    let mut color = Color::White;
    let mut idx = game.first_move;
    while let Some(node) = idx.and_then(|idx| game.moves.get(idx)) {
        if main_line.len() == MAX_PLIES
            || matches!(node.marker, Marker::EndMarker | Marker::EndGame)
        {
            break;
        }
        match node.move_data {
            Some(GameMove::Move(m)) => {
                let Some(squares) = squares(m, color) else {
                    break;
                };
                main_line.push(squares);
                color = !color;
            }
            Some(GameMove::Null) => break,
            None => {}
        }
        idx = node.next;
    }
    main_line
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_every_line_parses() {
        assert_eq!(LINE_SQUARES.len(), 254);
        assert!(LINE_SQUARES.iter().all(|line| !line.is_empty()));
        assert_eq!(LINE_SQUARES.iter().map(Vec::len).max(), Some(MAX_PLIES));
    }

    #[test]
    fn test_castling_is_the_kings_move() {
        // Line 61 ends with Black's O-O.
        assert_eq!(LINES[60], "1.d4 Nf6 2.c4 e6 3.Nc3 Bb4 4.e3 O-O");
        assert_eq!(LINE_SQUARES[60].last(), Some(&(Square::E8, Square::G8)));
    }
}
