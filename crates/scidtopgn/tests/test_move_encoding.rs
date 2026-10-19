use scidtopgn::private::{is_move_byte, is_special_byte, skip_tags};
use scidtopgn::{
    decode_king, decode_knight, decode_pawn, decode_queen, decode_rook, encode_king, encode_knight,
    encode_pawn, encode_queen, encode_rook, parse_move_byte, Board, ByteBuffer, Color, Piece,
    SimpleMove, Square, ENCODE_COMMENT, ENCODE_END_GAME, ENCODE_END_MARKER, ENCODE_NAG,
    ENCODE_START_MARKER,
};
use std::fs;

#[allow(dead_code)]
fn load_sg4_game_data(filename: &str) -> Vec<u8> {
    fs::read(filename).expect("Failed to read sg4 file")
}

#[test]
fn test_encode_decode_roundtrip_king() {
    let moves = [
        (Square::E1, Square::E1, 0),
        (Square::E1, Square::D1, 4),
        (Square::E1, Square::F1, 5),
        (Square::E1, Square::D2, 6),
        (Square::E1, Square::E2, 7),
        (Square::E1, Square::F2, 8),
        (Square::E1, Square::G1, 10),
        (Square::E1, Square::C1, 9),
    ];

    for (from, to, expected_val) in moves {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 0;
        sm.from = from;
        sm.to = to;

        encode_king(&mut buf, &sm).unwrap();
        let byte = buf.as_slice()[0];
        let (_, val) = parse_move_byte(byte);
        assert_eq!(
            val, expected_val,
            "King {} -> {} should encode to val {}",
            from, to, expected_val
        );

        buf.back_to_start();
        let mut sm2 = SimpleMove::new();
        sm2.from = from;
        decode_king(val, &mut sm2).unwrap();
        assert_eq!(
            sm2.to, to,
            "Decoding val {} from {} should give {}",
            val, from, to
        );
    }
}

#[test]
fn test_encode_decode_roundtrip_knight() {
    let knight_moves = [
        (Square::B1, Square::C3, 8),
        (Square::B1, Square::A3, 7),
        (Square::G1, Square::F3, 7),
        (Square::G1, Square::H3, 8),
        (Square::B1, Square::D2, 6),
        (Square::G1, Square::E2, 5),
    ];

    for (from, to, expected_val) in knight_moves {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 2;
        sm.from = from;
        sm.to = to;

        encode_knight(&mut buf, &sm).unwrap();
        let byte = buf.as_slice()[0];
        let (piece_num, val) = parse_move_byte(byte);
        assert_eq!(
            val, expected_val,
            "Knight {} -> {} should encode to val {}",
            from, to, expected_val
        );
        assert_eq!(piece_num, 2);

        let mut sm2 = SimpleMove::new();
        sm2.from = from;
        decode_knight(val, &mut sm2).unwrap();
        assert_eq!(
            sm2.to, to,
            "Decoding val {} from {} should give {}",
            val, from, to
        );
    }
}

#[test]
fn test_encode_decode_roundtrip_rook() {
    let rook_moves = [
        (Square::A1, Square::H1, 7),
        (Square::A1, Square::A8, 15),
        (Square::H1, Square::A1, 0),
        (Square::H1, Square::H8, 15),
        (Square::D4, Square::D8, 15),
        (Square::D4, Square::D1, 8),
        (Square::D4, Square::A4, 0),
        (Square::D4, Square::H4, 7),
    ];

    for (from, to, expected_val) in rook_moves {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 1;
        sm.from = from;
        sm.to = to;

        encode_rook(&mut buf, &sm).unwrap();
        let byte = buf.as_slice()[0];
        let (_, val) = parse_move_byte(byte);
        assert_eq!(
            val, expected_val,
            "Rook {} -> {} should encode to val {}",
            from, to, expected_val
        );

        let mut sm2 = SimpleMove::new();
        sm2.from = from;
        decode_rook(val, &mut sm2).unwrap();
        assert_eq!(
            sm2.to, to,
            "Decoding val {} from {} should give {}",
            val, from, to
        );
    }
}

#[test]
fn test_encode_decode_roundtrip_pawn_white() {
    let pawn_moves = [
        (Square::E2, Square::E3, Piece::Empty, 1),
        (Square::E2, Square::E4, Piece::Empty, 15),
        (Square::E7, Square::E8, Piece::Queen, 4),
        (Square::E7, Square::D8, Piece::Queen, 3),
        (Square::E7, Square::F8, Piece::Queen, 5),
        (Square::E7, Square::E8, Piece::Rook, 7),
        (Square::E7, Square::E8, Piece::Bishop, 10),
        (Square::E7, Square::E8, Piece::Knight, 13),
    ];

    for (from, to, promo, expected_val) in pawn_moves {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 12;
        sm.from = from;
        sm.to = to;
        sm.promote = promo;

        encode_pawn(&mut buf, &sm).unwrap();
        let byte = buf.as_slice()[0];
        let (_, val) = parse_move_byte(byte);
        assert_eq!(
            val, expected_val,
            "White pawn {} -> {} promo {:?} should encode to val {}",
            from, to, promo, expected_val
        );

        let mut sm2 = SimpleMove::new();
        sm2.from = from;
        decode_pawn(val, &mut sm2, Color::White).unwrap();
        assert_eq!(
            sm2.to, to,
            "Decoding val {} from {} as white should give {}",
            val, from, to
        );
        assert_eq!(
            sm2.promote, promo,
            "Decoding val {} should give promotion {:?}",
            val, promo
        );
    }
}

#[test]
fn test_encode_decode_roundtrip_queen() {
    let queen_rook_moves = [
        (Square::D1, Square::D8, 15),
        (Square::D1, Square::D5, 12),
        (Square::D1, Square::A1, 0),
        (Square::D1, Square::H1, 7),
    ];

    for (from, to, expected_val) in queen_rook_moves {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 4;
        sm.from = from;
        sm.to = to;

        encode_queen(&mut buf, &sm).unwrap();
        let byte = buf.as_slice()[0];
        let (_, val) = parse_move_byte(byte);
        assert_eq!(
            val, expected_val,
            "Queen {} -> {} should encode to val {}",
            from, to, expected_val
        );

        buf.back_to_start();
        let mut sm2 = SimpleMove::new();
        sm2.from = from;
        let mut decode_buf = ByteBuffer::from_slice(&buf.as_slice()[1..]);
        decode_queen(&mut decode_buf, val, &mut sm2).unwrap();
        assert_eq!(
            sm2.to, to,
            "Decoding val {} from {} should give {}",
            val, from, to
        );
    }
}

#[test]
fn test_encode_decode_queen_diagonal() {
    let from = Square::D1;
    let to = Square::H5;

    let mut buf = ByteBuffer::new();
    let mut sm = SimpleMove::new();
    sm.piece_num = 4;
    sm.from = from;
    sm.to = to;

    encode_queen(&mut buf, &sm).unwrap();
    assert_eq!(buf.as_slice().len(), 2, "Queen diagonal should be 2 bytes");

    let (piece_num, val) = parse_move_byte(buf.as_slice()[0]);
    assert_eq!(piece_num, 4);
    assert_eq!(val, from.file(), "First byte val should be from file");

    let second_byte = buf.as_slice()[1];
    assert_eq!(
        second_byte,
        to.to_index() as u8 + 64,
        "Second byte should be to square + 64"
    );

    buf.back_to_start();
    let mut decode_buf = ByteBuffer::from_slice(&buf.as_slice()[1..]);
    let mut sm2 = SimpleMove::new();
    sm2.from = from;
    decode_queen(&mut decode_buf, val, &mut sm2).unwrap();
    assert_eq!(sm2.to, to);
}

#[test]
fn test_special_byte_detection() {
    assert!(is_special_byte(ENCODE_NAG));
    assert!(is_special_byte(ENCODE_COMMENT));
    assert!(is_special_byte(ENCODE_START_MARKER));
    assert!(is_special_byte(ENCODE_END_MARKER));
    assert!(is_special_byte(ENCODE_END_GAME));

    assert!(!is_special_byte(0));
    assert!(!is_special_byte(1));
    assert!(!is_special_byte(10));
    assert!(!is_special_byte(0x10));
    assert!(!is_special_byte(0xA0));
}

#[test]
fn test_move_byte_detection() {
    let move_bytes = [
        0x00, 0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xA0,
    ];
    for byte in move_bytes {
        assert!(is_move_byte(byte), "0x{:02X} should be a move byte", byte);
    }

    let special_bytes = [0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F];
    for byte in special_bytes {
        assert!(
            !is_move_byte(byte),
            "0x{:02X} should NOT be a move byte",
            byte
        );
    }
}

#[test]
fn test_skip_tags() {
    let tag_data = [
        0x0a, b'W', b'h', b'i', b't', b'e', b'T', b'i', b't', b'l', b'e', 0x02, b'G', b'M', 0x00,
    ];

    let mut buf = ByteBuffer::from_slice(&tag_data);
    skip_tags(&mut buf).unwrap();
    assert_eq!(
        buf.position(),
        tag_data.len(),
        "Should skip past all tags to null terminator"
    );
}

#[test]
#[should_panic(expected = "Kings MUST be piece number zero")]
fn test_king_always_piece_zero() {
    let mut buf = ByteBuffer::new();
    let mut sm = SimpleMove::new();
    sm.piece_num = 5;
    sm.from = Square::E1;
    sm.to = Square::E2;

    let _ = encode_king(&mut buf, &sm);
}

#[test]
fn test_standard_start_piece_numbers() {
    let board = Board::std_start();

    assert_eq!(
        board.piece_list.get_piece_num(Square::E1),
        0,
        "White king should be piece 0"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::E8),
        0,
        "Black king should be piece 0"
    );

    assert_eq!(
        board.piece_list.get_piece_num(Square::A1),
        1,
        "White QR should be piece 1"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::B1),
        2,
        "White QN should be piece 2"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::C1),
        3,
        "White QB should be piece 3"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::D1),
        4,
        "White Q should be piece 4"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::F1),
        5,
        "White KB should be piece 5"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::G1),
        6,
        "White KN should be piece 6"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::H1),
        7,
        "White KR should be piece 7"
    );

    assert_eq!(
        board.piece_list.get_piece_num(Square::A2),
        8,
        "White a-pawn should be piece 8"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::B2),
        9,
        "White b-pawn should be piece 9"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::E2),
        12,
        "White e-pawn should be piece 12"
    );
    assert_eq!(
        board.piece_list.get_piece_num(Square::H2),
        15,
        "White h-pawn should be piece 15"
    );
}

#[test]
fn test_first_move_e4() {
    let mut buf = ByteBuffer::new();
    let mut sm = SimpleMove::new();
    sm.piece_num = 12;
    sm.from = Square::E2;
    sm.to = Square::E4;
    sm.promote = Piece::Empty;

    encode_pawn(&mut buf, &sm).unwrap();
    let byte = buf.as_slice()[0];

    assert_eq!(byte, 0xCF, "1.e4 should encode as 0xCF (piece 12, val 15)");

    buf.back_to_start();
    let mut sm2 = SimpleMove::new();
    sm2.from = Square::E2;
    let val = byte & 0x0F;
    decode_pawn(val, &mut sm2, Color::White).unwrap();
    assert_eq!(sm2.to, Square::E4);
}

#[test]
fn test_pawn_capture_direction() {
    let mut sm = SimpleMove::new();
    sm.from = Square::E5;

    decode_pawn(0, &mut sm, Color::White).unwrap();
    assert_eq!(
        sm.to,
        Square::D6,
        "White pawn val 0 from e5 should capture to d6"
    );

    let mut sm = SimpleMove::new();
    sm.from = Square::E5;
    decode_pawn(2, &mut sm, Color::White).unwrap();
    assert_eq!(
        sm.to,
        Square::F6,
        "White pawn val 2 from e5 should capture to f6"
    );

    let mut sm = SimpleMove::new();
    sm.from = Square::E4;
    decode_pawn(0, &mut sm, Color::Black).unwrap();
    assert_eq!(
        sm.to,
        Square::F3,
        "Black pawn val 0 from e4 should capture to f3 (toward h-file)"
    );

    let mut sm = SimpleMove::new();
    sm.from = Square::E4;
    decode_pawn(2, &mut sm, Color::Black).unwrap();
    assert_eq!(
        sm.to,
        Square::D3,
        "Black pawn val 2 from e4 should capture to d3 (toward a-file)"
    );
}

#[test]
fn test_castling_encoding() {
    let mut buf = ByteBuffer::new();
    let mut sm = SimpleMove::new();
    sm.piece_num = 0;
    sm.from = Square::E1;
    sm.to = Square::G1;

    encode_king(&mut buf, &sm).unwrap();
    assert_eq!(
        buf.as_slice()[0],
        0x0A,
        "O-O should be 0x0A (piece 0, val 10)"
    );

    let mut buf = ByteBuffer::new();
    let mut sm = SimpleMove::new();
    sm.piece_num = 0;
    sm.from = Square::E1;
    sm.to = Square::C1;

    encode_king(&mut buf, &sm).unwrap();
    assert_eq!(
        buf.as_slice()[0],
        0x09,
        "O-O-O should be 0x09 (piece 0, val 9)"
    );

    let mut sm = SimpleMove::new();
    sm.from = Square::E8;
    decode_king(10, &mut sm).unwrap();
    assert_eq!(sm.to, Square::G8, "Black O-O should go to g8");

    let mut sm = SimpleMove::new();
    sm.from = Square::E8;
    decode_king(9, &mut sm).unwrap();
    assert_eq!(sm.to, Square::C8, "Black O-O-O should go to c8");
}

#[test]
fn test_all_king_directions() {
    let directions = [
        (Square::E4, Square::D3, 1),
        (Square::E4, Square::E3, 2),
        (Square::E4, Square::F3, 3),
        (Square::E4, Square::D4, 4),
        (Square::E4, Square::F4, 5),
        (Square::E4, Square::D5, 6),
        (Square::E4, Square::E5, 7),
        (Square::E4, Square::F5, 8),
    ];

    for (from, to, expected_val) in directions {
        let mut buf = ByteBuffer::new();
        let mut sm = SimpleMove::new();
        sm.piece_num = 0;
        sm.from = from;
        sm.to = to;

        encode_king(&mut buf, &sm).unwrap();
        let val = buf.as_slice()[0] & 0x0F;
        assert_eq!(val, expected_val, "King {} -> {}", from, to);

        let mut sm2 = SimpleMove::new();
        sm2.from = from;
        decode_king(val, &mut sm2).unwrap();
        assert_eq!(sm2.to, to);
    }
}
