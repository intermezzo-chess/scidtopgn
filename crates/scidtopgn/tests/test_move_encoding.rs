use scidtopgn::shakmaty::uci::UciMove;
use scidtopgn::{
    decode_move, encode_move, parse_move_byte, ByteBuffer, Game, GameMove, ScidPosition,
    ENCODE_COMMENT, ENCODE_END_GAME, ENCODE_END_MARKER, ENCODE_NAG, ENCODE_START_MARKER,
    GAME_DECODE_COMMENTS, GAME_DECODE_NONE,
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

/// The bytes the decoder reads as markers rather than moves.
const MARKERS: [u8; 5] = [
    ENCODE_NAG,
    ENCODE_COMMENT,
    ENCODE_START_MARKER,
    ENCODE_END_MARKER,
    ENCODE_END_GAME,
];

#[test]
fn test_special_byte_detection() {
    // Each marker is piece 0, the king, with a value no king move uses.
    for marker in MARKERS {
        let (piece, value) = parse_move_byte(marker);
        assert_eq!(piece, 0, "0x{marker:02X} should be piece 0");
        assert!(value >= ENCODE_NAG, "0x{marker:02X} shouldn't be a move");
    }
    assert_eq!(MARKERS, [11, 12, 13, 14, 15]);

    for byte in [0, 1, 10, 0x10, 0xA0, 0x1B, 0xAF] {
        assert!(
            !MARKERS.contains(&byte),
            "0x{byte:02X} shouldn't be a marker"
        );
    }
}

/// Decodes a record with no tags, the standard start, and `body`.
fn decode_body(body: &[u8]) -> Game {
    let mut record = vec![0x00, 0x00];
    record.extend(body);
    let mut game = Game::new();
    game.decode(&mut ByteBuffer::from_slice(&record), GAME_DECODE_COMMENTS)
        .unwrap();
    game
}

/// The SAN of the main line's moves.
fn main_line(game: &Game) -> Vec<&str> {
    let mut sans = Vec::new();
    let mut idx = game.first_move;
    while let Some(i) = idx {
        if game.moves[i].move_data.is_some() {
            sans.push(game.moves[i].san.as_str());
        }
        idx = game.moves[i].next;
    }
    sans
}

#[test]
fn test_moves_with_a_marker_like_low_nibble() {
    // Only piece 0 has marker bytes, so another piece's value of 11 to 15 is
    // a move. Here, the c-pawn's double step.
    assert_eq!(encode_game(&["c2c4"]), [0xAF]);
    assert_eq!(main_line(&decode_body(&[0xAF, 0x0F])), ["c4"]);

    // Piece 1, the rook on a1, moves along an open file.
    let open_file = [
        "a2a4", "b7b5", "a4b5", "a7a6", "b5a6", "c8b7", "a6b7", "h7h6",
    ];
    for (uci, byte, san) in [
        ("a1a4", 0x1B, "Ra4"),
        ("a1a5", 0x1C, "Ra5"),
        ("a1a6", 0x1D, "Ra6"),
        ("a1a7", 0x1E, "Ra7"),
        ("a1a8", 0x1F, "Rxa8"),
    ] {
        let mut moves = open_file.to_vec();
        moves.push(uci);
        let mut body = encode_game(&moves);
        assert_eq!(body.last(), Some(&byte), "{uci}");

        body.push(0x0F);
        let game = decode_body(&body);
        assert_eq!(main_line(&game).len(), moves.len(), "{uci}");
        assert_eq!(main_line(&game).last(), Some(&san), "{uci}");
    }
}

#[test]
fn test_marker_bytes_in_a_record() {
    let main = encode_game(&["e2e4", "e7e5"]);
    let d4 = encode_game(&["d2d4"]);
    // 1.e4 with a NAG, a comment, and the variation 1.d4, then 1...e5 and the
    // end of the game. The comment's text follows that.
    let mut body = vec![main[0], 0x0B, 1, 0x0C, 0x0D, d4[0], 0x0E, main[1], 0x0F];
    body.extend(b"note\0");

    let game = decode_body(&body);
    assert_eq!(main_line(&game), ["e4", "e5"]);
    let node = |san: &str| game.moves.iter().find(|node| node.san == san).unwrap();
    assert_eq!(node("e4").nags, [1]);
    assert_eq!(node("e4").comment.as_deref(), Some("note"));
    assert_eq!(node("e4").num_variations, 1);
    assert!(node("d4").nags.is_empty());
    assert_eq!(node("d4").comment, None);
    assert!(node("e5").nags.is_empty());
    assert_eq!(node("e5").comment, None);

    let variation = node("e4").var_child.unwrap();
    let first = game.moves[variation].next.unwrap();
    assert_eq!(game.moves[first].san, "d4");
    assert_eq!(game.moves[variation].var_child, None);
}

#[test]
fn test_skip_tags() {
    // A WhiteTitle tag, the end of the tags, the standard start, 1.e4, and
    // the end of the game.
    let mut record = vec![
        0x0a, b'W', b'h', b'i', b't', b'e', b'T', b'i', b't', b'l', b'e', 0x02, b'G', b'M', 0x00,
    ];
    record.extend([0x00, 0xCF, ENCODE_END_GAME]);

    let mut game = Game::new();
    game.decode(&mut ByteBuffer::from_slice(&record), GAME_DECODE_NONE)
        .unwrap();
    assert!(game.tags.is_empty(), "tags should be skipped");
    assert_eq!(game.moves[1].san, "e4", "the moves follow the tags");
}

#[test]
fn test_decoding_keeps_the_first_7_nags_of_a_move() {
    // As in SCID, a move keeps its first 7 NAGs, and the record is still
    // read in full after the NAGs dropped.
    let main_line = encode_game(&["e2e4", "e7e5"]);
    let d4 = encode_game(&["d2d4"]);
    // No tags, the standard start, and 1.e4 with NAGs 1 to 10 and a comment.
    let mut record = vec![0x00, 0x00, main_line[0]];
    for nag in 1..=10 {
        record.extend([ENCODE_NAG, nag]);
    }
    record.push(ENCODE_COMMENT);
    // A variation, 1.d4 with NAG 11, then 1...e5 with NAG 12.
    record.extend([
        ENCODE_START_MARKER,
        d4[0],
        ENCODE_NAG,
        11,
        ENCODE_END_MARKER,
    ]);
    record.extend([main_line[1], ENCODE_NAG, 12, ENCODE_END_GAME]);
    record.extend(b"note\0");

    let mut game = Game::new();
    game.decode(&mut ByteBuffer::from_slice(&record), GAME_DECODE_COMMENTS)
        .unwrap();
    let nodes: Vec<(&str, &[u8], Option<&str>)> = game
        .moves
        .iter()
        .filter(|node| !node.san.is_empty())
        .map(|node| (node.san.as_str(), &node.nags[..], node.comment.as_deref()))
        .collect();
    assert_eq!(
        nodes,
        [
            ("e4", &[1, 2, 3, 4, 5, 6, 7][..], Some("note")),
            ("d4", &[11][..], None),
            ("e5", &[12][..], None),
        ]
    );
}
