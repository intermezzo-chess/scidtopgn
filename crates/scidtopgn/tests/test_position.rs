use scidtopgn::{piece_make, Board, Color, Piece, PieceList, Square};

#[test]
fn test_piece_list_new() {
    let pl = PieceList::new();
    assert_eq!(pl.count[0], 0);
    assert_eq!(pl.count[1], 0);
    for i in 0..16 {
        assert_eq!(pl.list[0][i], Square::Null);
        assert_eq!(pl.list[1][i], Square::Null);
    }
    for i in 0..64 {
        assert_eq!(pl.list_pos[i], 255);
    }
}

#[test]
fn test_piece_list_add_single_piece() {
    let mut pl = PieceList::new();

    pl.add_piece(Square::E1, Piece::King, Color::White);

    assert_eq!(pl.count[0], 1);
    assert_eq!(pl.list[0][0], Square::E1);
    assert_eq!(pl.list_pos[Square::E1.to_index()], 0);
    assert_eq!(pl.get_king_square(Color::White), Square::E1);
}

#[test]
fn test_piece_list_king_always_zero() {
    let mut pl = PieceList::new();

    pl.add_piece(Square::A1, Piece::Rook, Color::White);
    assert_eq!(pl.list[0][0], Square::A1);
    assert_eq!(pl.count[0], 1);

    pl.add_piece(Square::B1, Piece::Knight, Color::White);
    assert_eq!(pl.list[0][0], Square::A1);
    assert_eq!(pl.list[0][1], Square::B1);
    assert_eq!(pl.count[0], 2);

    pl.add_piece(Square::E1, Piece::King, Color::White);
    assert_eq!(pl.list[0][0], Square::E1);
    assert_eq!(pl.list_pos[Square::E1.to_index()], 0);
    assert_eq!(pl.list[0][2], Square::A1);
    assert_eq!(pl.list_pos[Square::A1.to_index()], 2);
    assert_eq!(pl.list[0][1], Square::B1);
    assert_eq!(pl.list_pos[Square::B1.to_index()], 1);
    assert_eq!(pl.count[0], 3);
}

#[test]
fn test_piece_list_remove_swap() {
    let mut pl = PieceList::new();

    pl.add_piece(Square::A1, Piece::Rook, Color::White);
    pl.add_piece(Square::B1, Piece::Knight, Color::White);
    pl.add_piece(Square::C1, Piece::Bishop, Color::White);

    assert_eq!(pl.count[0], 3);
    assert_eq!(pl.list_pos[Square::B1.to_index()], 1);

    let captured_num = pl.remove_piece(Square::B1, Color::White);

    assert_eq!(captured_num, 1);
    assert_eq!(pl.count[0], 2);
    assert_eq!(pl.list[0][1], Square::C1);
    assert_eq!(pl.list_pos[Square::C1.to_index()], 1);
}

#[test]
fn test_std_start_piece_numbers() {
    let board = Board::std_start();

    assert_eq!(board.piece_list.get_piece_num(Square::E1), 0);
    assert_eq!(board.piece_list.get_piece_num(Square::A1), 1);
    assert_eq!(board.piece_list.get_piece_num(Square::B1), 2);
    assert_eq!(board.piece_list.get_piece_num(Square::C1), 3);
    assert_eq!(board.piece_list.get_piece_num(Square::D1), 4);
    assert_eq!(board.piece_list.get_piece_num(Square::F1), 5);
    assert_eq!(board.piece_list.get_piece_num(Square::G1), 6);
    assert_eq!(board.piece_list.get_piece_num(Square::H1), 7);

    assert_eq!(board.piece_list.get_piece_num(Square::A2), 8);
    assert_eq!(board.piece_list.get_piece_num(Square::B2), 9);
    assert_eq!(board.piece_list.get_piece_num(Square::C2), 10);
    assert_eq!(board.piece_list.get_piece_num(Square::D2), 11);
    assert_eq!(board.piece_list.get_piece_num(Square::E2), 12);
    assert_eq!(board.piece_list.get_piece_num(Square::F2), 13);
    assert_eq!(board.piece_list.get_piece_num(Square::G2), 14);
    assert_eq!(board.piece_list.get_piece_num(Square::H2), 15);

    assert_eq!(board.piece_list.get_piece_num(Square::E8), 0);
    assert_eq!(board.piece_list.get_piece_num(Square::A8), 1);
    assert_eq!(board.piece_list.get_piece_num(Square::H7), 15);
}

#[test]
fn test_capture_renumbers_last_piece() {
    let mut board = Board::std_start();

    assert_eq!(board.piece_list.get_piece_num(Square::D7), 11);
    assert_eq!(board.piece_list.get_piece_num(Square::H7), 15);
    assert_eq!(board.get_piece_count(Color::Black), 16);

    let captured_num = board.piece_list.remove_piece(Square::D7, Color::Black);
    board.remove_from_board(Square::D7);
    board.material[piece_make(Color::Black, Piece::Pawn) as usize] -= 1;

    assert_eq!(captured_num, 11);
    assert_eq!(board.get_piece_count(Color::Black), 15);

    assert_eq!(board.piece_list.get_piece_num(Square::H7), 11);
    assert_eq!(board.piece_list.get_square(Color::Black, 11), Square::H7);
}

#[test]
fn test_from_fen_std_position() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let board = Board::from_fen(fen).unwrap();

    assert_eq!(board.to_move, Color::White);
    assert_eq!(board.get_piece_count(Color::White), 16);
    assert_eq!(board.get_piece_count(Color::Black), 16);
    assert_eq!(board.get_king_square(Color::White), Square::E1);
    assert_eq!(board.get_king_square(Color::Black), Square::E8);
    assert_eq!(board.castling, 0xF);
}

#[test]
fn test_from_fen_custom_position() {
    let fen = "r3k2r/pppppppp/8/8/8/8/PPPPPPPP/R3K2R w KQkq - 0 1";
    let board = Board::from_fen(fen).unwrap();

    assert!(board.get_piece(Square::B1).is_none());
    assert!(board.get_piece(Square::C1).is_none());
    assert!(board.get_piece(Square::F1).is_none());
    assert!(board.get_piece(Square::G1).is_none());

    let rook = board.get_piece(Square::A1);
    assert!(rook.is_some());
    assert_eq!(rook.unwrap().0, Piece::Rook);
    assert_eq!(rook.unwrap().1, Color::White);

    assert_eq!(board.get_piece_count(Color::White), 11);
}

#[test]
fn test_fen_roundtrip() {
    let board = Board::std_start();
    let fen = board.to_fen();
    let board2 = Board::from_fen(&fen).unwrap();
    let fen2 = board2.to_fen();

    assert_eq!(fen, fen2);
    assert_eq!(
        fen,
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
    );
}

#[test]
fn test_king_swap_in_fen_parsing() {
    let fen = "8/8/8/8/8/8/8/R3K2R w KQ - 0 1";
    let board = Board::from_fen(fen).unwrap();

    assert_eq!(board.get_king_square(Color::White), Square::E1);
    assert_eq!(board.piece_list.get_piece_num(Square::E1), 0);

    assert_eq!(board.piece_list.get_piece_num(Square::A1), 1);
    assert_eq!(board.piece_list.get_piece_num(Square::H1), 2);
}

#[test]
fn test_en_passant_target() {
    let fen = "rnbqkbnr/pppp1ppp/8/4pP2/8/8/PPPPP1PP/RNBQKBNR w KQkq e6 0 3";
    let board = Board::from_fen(fen).unwrap();

    assert_eq!(board.ep_target, Some(Square::E6));
}

#[test]
fn test_halfmove_and_fullmove() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 5 10";
    let board = Board::from_fen(fen).unwrap();

    assert_eq!(board.halfmove_clock, 5);
    assert_eq!(board.fullmove_number, 10);
}

#[test]
fn test_castling_flags() {
    let mut board = Board::new();

    board.set_castling(Color::White, true, true);
    assert!(board.get_castling(Color::White, true));
    assert!(!board.get_castling(Color::White, false));

    board.set_castling(Color::Black, false, true);
    assert!(board.get_castling(Color::Black, false));
    assert!(!board.get_castling(Color::Black, true));
}

#[test]
fn test_sequential_captures() {
    let mut board = Board::std_start();

    board.piece_list.remove_piece(Square::D7, Color::Black);
    board.material[piece_make(Color::Black, Piece::Pawn) as usize] -= 1;
    board.get_piece_count(Color::Black);

    assert_eq!(board.piece_list.get_piece_num(Square::H7), 11);

    board.piece_list.remove_piece(Square::H7, Color::Black);
    board.material[piece_make(Color::Black, Piece::Pawn) as usize] -= 1;

    assert_eq!(board.piece_list.get_piece_num(Square::G7), 11);
}

#[test]
fn test_piece_list_move_piece() {
    let mut pl = PieceList::new();

    pl.add_piece(Square::E1, Piece::King, Color::White);
    pl.add_piece(Square::D1, Piece::Queen, Color::White);

    pl.move_piece(Square::D1, Square::D4, Color::White);

    assert_eq!(pl.get_square(Color::White, 1), Square::D4);
    assert_eq!(pl.get_piece_num(Square::D4), 1);
    assert_eq!(pl.get_piece_num(Square::D1), 255);
}
