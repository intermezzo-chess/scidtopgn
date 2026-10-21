use scidtopgn::shakmaty::fen::Fen;
use scidtopgn::shakmaty::uci::UciMove;
use scidtopgn::shakmaty::{CastlingMode, Chess, Color, Position, Square};
use scidtopgn::{GameMove, PieceList, ScidPosition};

fn position(fen: &str) -> ScidPosition {
    let chess: Chess = fen
        .parse::<Fen>()
        .unwrap()
        .into_position(CastlingMode::Standard)
        .unwrap();
    ScidPosition::from_setup_position(chess).unwrap()
}

/// Plays moves given in UCI and returns their SAN.
fn play(pos: &mut ScidPosition, moves: &[&str]) -> Vec<String> {
    moves
        .iter()
        .map(|uci| {
            let m = uci
                .parse::<UciMove>()
                .unwrap()
                .to_move(pos.chess())
                .unwrap();
            pos.play(GameMove::Move(m)).unwrap().to_string()
        })
        .collect()
}

fn san(fen: &str, uci: &str) -> String {
    play(&mut position(fen), &[uci]).remove(0)
}

#[test]
fn test_std_start_piece_numbers() {
    let pos = ScidPosition::standard();
    let pieces = pos.pieces();

    let expected = [
        (Square::E1, 0),
        (Square::A1, 1),
        (Square::B1, 2),
        (Square::C1, 3),
        (Square::D1, 4),
        (Square::F1, 5),
        (Square::G1, 6),
        (Square::H1, 7),
        (Square::A2, 8),
        (Square::B2, 9),
        (Square::C2, 10),
        (Square::D2, 11),
        (Square::E2, 12),
        (Square::F2, 13),
        (Square::G2, 14),
        (Square::H2, 15),
    ];
    for (square, number) in expected {
        assert_eq!(pieces.number(square), Some(number), "{square}");
        assert_eq!(
            pieces.square(Color::White, number),
            Some(square),
            "{number}"
        );
        let black = square.flip_vertical();
        assert_eq!(pieces.number(black), Some(number), "{black}");
    }
    assert_eq!(pieces.count(Color::White), 16);
    assert_eq!(pieces.count(Color::Black), 16);
    assert_eq!(pieces, &PieceList::standard());
}

#[test]
fn test_capture_renumbers_last_piece() {
    let mut pos = ScidPosition::standard();
    play(&mut pos, &["e2e4", "d7d5", "e4d5"]);

    let pieces = pos.pieces();
    assert_eq!(pieces.count(Color::Black), 15);
    assert_eq!(pieces.number(Square::H7), Some(11));
    assert_eq!(pieces.square(Color::Black, 11), Some(Square::H7));
    assert_eq!(pieces.square(Color::Black, 15), None);
    assert_eq!(pieces.number(Square::D5), Some(12));
    assert_eq!(pieces.number(Square::E2), None);
}

#[test]
fn test_sequential_captures() {
    let mut pos = ScidPosition::standard();
    // White's e-pawn takes d5, then h7's new number moves to g7.
    play(
        &mut pos,
        &["e2e4", "d7d5", "e4d5", "g8f6", "d5d6", "h7h6", "d6c7"],
    );
    let pieces = pos.pieces();
    assert_eq!(pieces.count(Color::Black), 14);
    // c7 had 10. The last piece, g7 (14), takes it.
    assert_eq!(pieces.number(Square::G7), Some(10));
    assert_eq!(pieces.number(Square::H6), Some(11));
}

#[test]
fn test_fen_numbering() {
    let pos = position("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    let pieces = pos.pieces();
    assert_eq!(pieces.number(Square::E1), Some(0));
    assert_eq!(pieces.number(Square::A1), Some(1));
    assert_eq!(pieces.number(Square::H1), Some(2));
    assert_eq!(pieces.number(Square::E8), Some(0));
    assert_eq!(pieces.number(Square::A8), Some(1));
    assert_eq!(pieces.number(Square::H8), Some(2));
}

#[test]
fn test_promotion_keeps_number() {
    let mut pos = position("1n2k3/P7/8/8/8/8/8/4K3 w - - 0 1");
    let number = pos.pieces().number(Square::A7);
    assert_eq!(play(&mut pos, &["a7b8n"]), ["axb8=N"]);
    assert_eq!(pos.pieces().number(Square::B8), number);
    assert_eq!(pos.pieces().count(Color::Black), 1);
}

#[test]
fn test_en_passant_removes_captured_pawn() {
    let mut pos = position("4k3/3p4/8/4P3/8/8/8/4K3 b - - 0 1");
    assert_eq!(play(&mut pos, &["d7d5", "e5d6"]), ["d5", "exd6"]);
    assert_eq!(pos.pieces().count(Color::Black), 1);
    assert_eq!(pos.pieces().number(Square::D5), None);
    assert_eq!(pos.pieces().number(Square::D6), Some(1));
}

#[test]
fn test_castling_numbers() {
    let mut pos = position("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");
    assert_eq!(play(&mut pos, &["e1g1", "e8c8"]), ["O-O", "O-O-O"]);
    let pieces = pos.pieces();
    assert_eq!(pieces.number(Square::G1), Some(0));
    assert_eq!(pieces.number(Square::F1), Some(2));
    assert_eq!(pieces.number(Square::C8), Some(0));
    assert_eq!(pieces.number(Square::D8), Some(1));
}

#[test]
fn test_disambiguation_by_file() {
    // Game 2 of tests/data/five, move 15 (#25).
    let fen = "rq3rk1/4bppp/3pbn2/npp1p1B1/3PP3/2P2N2/1PB2PPP/RN1QR1K1 w - - 0 15";
    assert_eq!(san(fen, "b1d2"), "Nbd2");
}

#[test]
fn test_disambiguation_by_rank() {
    assert_eq!(san("4k3/8/8/R7/8/8/8/R3K3 w - - 0 1", "a1a3"), "R1a3");
}

#[test]
fn test_disambiguation_by_file_and_rank() {
    assert_eq!(san("8/2k5/8/8/4Q2Q/8/8/K6Q w - - 0 1", "h4e1"), "Qh4e1");
    // Four knights after promotions.
    assert_eq!(san("4k3/8/8/8/8/1N3N2/8/1N2KN2 w - - 0 1", "b1d2"), "Nb1d2");
}

#[test]
fn test_disambiguation_of_capture() {
    // Game 4 of tests/data/five, move 22 (#25).
    let fen = "5rk1/3nqrb1/1p2p1np/p1ppN3/P2Pb3/2PN2B1/1P1QB1PP/4RRK1 b - - 1 22";
    assert_eq!(san(fen, "d7e5"), "Ndxe5");
}

#[test]
fn test_pinned_piece_does_not_disambiguate() {
    // The knight on f3 is pinned by the bishop on e4.
    assert_eq!(san("4k3/8/8/8/4b3/5N2/8/1N5K w - - 0 1", "b1d2"), "Nd2");
}

#[test]
fn test_check_that_can_be_blocked_is_not_mate() {
    // The king can't move, but the bishop can block on f8.
    assert_eq!(san("7k/6pp/8/8/8/b7/8/4R1K1 w - - 0 1", "e1e8"), "Re8+");
    // The king can't move, but the rook can take the checker.
    assert_eq!(san("r6k/6pp/8/8/8/8/8/4R1K1 w - - 0 1", "e1e8"), "Re8+");
    assert_eq!(san("7k/6pp/8/8/8/8/8/4R1K1 w - - 0 1", "e1e8"), "Re8#");
}

#[test]
fn test_stalemate() {
    let mut pos = position("7k/8/8/6Q1/8/8/8/K7 w - - 0 1");
    assert_eq!(play(&mut pos, &["g5g6"]), ["Qg6"]);
    assert!(pos.chess().is_stalemate());
}

#[test]
fn test_null_move() {
    let mut pos = ScidPosition::standard();
    play(&mut pos, &["e2e4"]);
    assert_eq!(pos.play(GameMove::Null).unwrap().to_string(), "--");
    assert_eq!(pos.chess().turn(), Color::White);
    assert_eq!(play(&mut pos, &["d2d4"]), ["d4"]);
}

#[test]
fn test_illegal_moves_are_rejected() {
    let mut pos = ScidPosition::standard();
    let m = scidtopgn::shakmaty::Move::Normal {
        role: scidtopgn::shakmaty::Role::Knight,
        from: Square::B1,
        capture: None,
        to: Square::D2,
        promotion: None,
    };
    assert!(matches!(
        pos.play(GameMove::Move(m)),
        Err(scidtopgn::Error::InvalidMove { .. })
    ));
    assert_eq!(pos.pieces(), &PieceList::standard());
}
