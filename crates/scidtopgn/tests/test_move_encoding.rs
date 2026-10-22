use scidtopgn::private::{is_move_byte, is_special_byte, skip_tags};
use scidtopgn::shakmaty::uci::UciMove;
use scidtopgn::{
    decode_move, encode_move, ByteBuffer, GameMove, ScidPosition, ENCODE_COMMENT, ENCODE_END_GAME,
    ENCODE_END_MARKER, ENCODE_NAG, ENCODE_START_MARKER,
};

/// Encodes the moves, given in UCI, from the standard starting position,
/// checks that they decode back, and returns the bytes.
fn encode_game(moves: &[&str]) -> Vec<u8> {
    let mut pos = ScidPosition::standard();
    let mut bytes = Vec::new();
    for uci in moves {
        let mv = match *uci {
            "--" => GameMove::Null,
            uci => GameMove::Move(
                uci.parse::<UciMove>()
                    .unwrap()
                    .to_move(pos.chess())
                    .unwrap(),
            ),
        };
        let mut buf = ByteBuffer::new();
        encode_move(&mut buf, &pos, mv).unwrap();
        let encoded = buf.as_slice().to_vec();

        let mut rest = ByteBuffer::from_slice(&encoded[1..]);
        assert_eq!(decode_move(&mut rest, &pos, encoded[0]).unwrap(), mv);

        pos.play(mv).unwrap();
        bytes.extend(encoded);
    }
    bytes
}

#[test]
fn test_first_move_e4() {
    assert_eq!(encode_game(&["e2e4"]), [0xCF], "1.e4 is piece 12, value 15");
}

#[test]
fn test_castling_encoding() {
    // 1.e4 e5 2.Nf3 Nc6 3.Bc4 Bc5 4.O-O
    let bytes = encode_game(&["e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "f8c5", "e1g1"]);
    assert_eq!(bytes.last(), Some(&0x0A), "O-O is piece 0, value 10");

    // 1.d4 d5 2.Nc3 Nc6 3.Bf4 Bf5 4.Qd2 Qd7 5.O-O-O O-O-O
    let bytes = encode_game(&[
        "d2d4", "d7d5", "b1c3", "b8c6", "c1f4", "c8f5", "d1d2", "d8d7", "e1c1", "e8c8",
    ]);
    assert_eq!(&bytes[bytes.len() - 2..], &[0x09, 0x09], "O-O-O is value 9");
}

#[test]
fn test_queen_diagonal_takes_two_bytes() {
    // 1.e4 e5 2.Qh5: the queen (4) on the d-file, then h5 + 64.
    let bytes = encode_game(&["e2e4", "e7e5", "d1h5"]);
    assert_eq!(&bytes[2..], &[0x43, 39 + 64]);
}

#[test]
fn test_moves_after_captures_use_new_numbers() {
    // 1.e4 d5 2.exd5 Qxd5 3.Nc3 Qa5 4.d4 h6: Black's h-pawn takes number
    // 11 from the captured d-pawn, and its queen keeps 4.
    let bytes = encode_game(&[
        "e2e4", "d7d5", "e4d5", "d8d5", "b1c3", "d5a5", "d2d4", "h7h6",
    ]);
    assert_eq!(bytes[3], 0x4C, "Qxd5 is piece 4, rank 5 (8 + 4)");
    assert_eq!(bytes.last(), Some(&0xB1), "h6 is piece 11, one square");
}

#[test]
fn test_null_move_is_a_king_move_to_its_square() {
    assert_eq!(encode_game(&["e2e4", "--", "d2d4"]), [0xCF, 0x00, 0xBF]);
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
