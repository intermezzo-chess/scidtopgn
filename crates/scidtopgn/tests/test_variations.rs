//! Nested variations, and the limits on how deep variations nest and how
//! many one move can have.

use scidtopgn::shakmaty::uci::UciMove;
use scidtopgn::{
    encode_move, parse_single_game, ByteBuffer, Database, Error, Game, GameMove, MoveNode,
    PgnParser, ScidPosition, ENCODE_COMMENT, ENCODE_END_GAME, ENCODE_END_MARKER,
    ENCODE_START_MARKER, GAME_DECODE_ALL,
};
use tempfile::TempDir;

/// Six levels of variations, with a NAG or a comment at each boundary.
const NESTED: &str = "1. e4 e5 2. Nf3 {m2} (2. f4 {L1} exf4 (2... d5 {L2} 3. exd5 \
(3. Nf3 {L3} dxe4 (3... Nf6 $1 {L4} 4. Nxe5 (4. d3 {L5} Bc5 (4... Bb4+ {L6} $2) \
$3 {after L6}) {after L5} $4) {after L4} $5) $6 {after L3}) {after L2} $7 3. Nf3) \
{after L1} $10 Nc6 *";

fn pgn_game(movetext: &str) -> String {
    format!("[Event \"x\"]\n[Result \"*\"]\n\n{movetext}\n")
}

/// The first node, in the order the moves were read, that plays `san`.
fn node<'a>(game: &'a Game, san: &str) -> &'a MoveNode {
    game.moves
        .iter()
        .find(|node| node.san == san)
        .unwrap_or_else(|| panic!("no move {san}"))
}

fn annotations(game: &Game, san: &str) -> (Vec<u8>, Option<String>) {
    let node = node(game, san);
    (node.nags.clone(), node.comment.clone())
}

/// Writes `game` to a new database and reads it back.
fn scid_round_trip(game: &Game) -> Game {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("variations");
    let mut db = Database::create(&path).unwrap();
    db.add_game(game).unwrap();
    db.flush().unwrap();
    Database::open(&path).unwrap().get_game(0).unwrap()
}

/// `count` levels of variations, each replacing the first move.
fn nested_movetext(count: usize) -> String {
    format!("1. e4 {}{} *", "(1. e4 ".repeat(count), ")".repeat(count))
}

/// `count` variations of the first move, each with a comment.
fn sibling_movetext(count: usize) -> String {
    let variations: String = (1..=count).map(|n| format!("(1. d4 {{v{n}}}) ")).collect();
    format!("1. e4 {variations}e5 *")
}

fn parse_error(movetext: &str) -> String {
    let mut parser = PgnParser::new(&pgn_game(movetext));
    match parser.next() {
        Some(Err(Error::PgnGame { source, .. })) => source.to_string(),
        _ => panic!("expected a parse error"),
    }
}

/// The byte that encodes `uci` from the standard starting position.
fn move_byte(uci: &str) -> u8 {
    let position = ScidPosition::standard();
    let mv = uci
        .parse::<UciMove>()
        .unwrap()
        .to_move(position.chess())
        .unwrap();
    let mut buf = ByteBuffer::new();
    encode_move(&mut buf, &position, GameMove::Move(mv)).unwrap();
    assert_eq!(buf.len(), 1);
    buf.as_slice()[0]
}

/// Decodes a game record with no tags, the standard starting position, the
/// encoded `moves`, and `comments`.
fn decode(moves: &[u8], comments: &[&str]) -> scidtopgn::Result<Game> {
    let mut record = vec![0, 0];
    record.extend(moves);
    record.push(ENCODE_END_GAME);
    for comment in comments {
        record.extend(comment.as_bytes());
        record.push(0);
    }
    let mut game = Game::new();
    game.decode(&mut ByteBuffer::from_slice(&record), GAME_DECODE_ALL)?;
    Ok(game)
}

#[test]
fn test_annotations_in_nested_variations() {
    let game = parse_single_game(&pgn_game(NESTED)).unwrap();
    let expected = [
        ("Nf3", vec![10], Some("m2 after L1")),
        ("f4", vec![], Some("L1")),
        ("exf4", vec![7], Some("after L2")),
        ("d5", vec![], Some("L2")),
        ("exd5", vec![6], Some("after L3")),
        ("dxe4", vec![5], Some("after L4")),
        ("Nf6", vec![1], Some("L4")),
        ("Nxe5", vec![4], Some("after L5")),
        ("d3", vec![], Some("L5")),
        ("Bc5", vec![3], Some("after L6")),
        ("Bb4+", vec![2], Some("L6")),
        ("Nc6", vec![], None),
    ];
    let check = |game: &Game, stage: &str| {
        for (san, nags, comment) in &expected {
            let expected = (nags.clone(), comment.map(String::from));
            assert_eq!(annotations(game, san), expected, "{san} after {stage}");
        }
    };
    check(&game, "PGN import");
    let reread = scid_round_trip(&game);
    check(&reread, "SCID round trip");

    let exported = reread.to_pgn();
    let reimported = parse_single_game(&exported).unwrap();
    check(&reimported, "PGN to SCID to PGN");
    assert_eq!(scid_round_trip(&reimported).to_pgn(), exported);
}

#[test]
fn test_variation_depth_limit_in_pgn() {
    // Runs on the test thread's 2 MiB stack.
    let game = parse_single_game(&pgn_game(&nested_movetext(100))).unwrap();
    let reread = scid_round_trip(&game);
    assert_eq!(reread.moves.len(), game.moves.len());
    assert_eq!(reread.to_pgn(), game.to_pgn());
    assert_eq!(
        parse_single_game(&game.to_pgn()).unwrap().to_pgn(),
        game.to_pgn()
    );

    assert_eq!(
        parse_error(&nested_movetext(101)),
        "Parse error at line 4: Variation nested more than 100 levels deep"
    );
}

#[test]
fn test_variation_count_limit_in_pgn() {
    let game = parse_single_game(&pgn_game(&sibling_movetext(255))).unwrap();
    assert_eq!(node(&game, "e4").num_variations, 255);
    let reread = scid_round_trip(&game);
    let comments = reread
        .moves
        .iter()
        .filter(|node| node.san == "d4")
        .map(|node| node.comment.clone().unwrap())
        .collect::<Vec<_>>();
    let expected = (1..=255).map(|n| format!("v{n}")).collect::<Vec<_>>();
    assert_eq!(comments, expected);
    assert_eq!(reread.to_pgn(), game.to_pgn());

    assert_eq!(
        parse_error(&sibling_movetext(256)),
        "Parse error at line 4: More than 255 variations of one move"
    );
}

#[test]
fn test_games_after_a_rejected_variation_are_read() {
    let input = [
        nested_movetext(101),
        sibling_movetext(256),
        "1. d4 *".into(),
    ]
    .map(|movetext| pgn_game(&movetext))
    .concat();
    let results: Vec<_> = PgnParser::new(&input).collect();
    assert_eq!(results.len(), 3);
    assert!(results[0].is_err());
    assert!(results[1].is_err());
    assert_eq!(results[2].as_ref().unwrap().moves[1].san, "d4");
}

#[test]
fn test_variation_depth_limit_in_scid() {
    let e4 = move_byte("e2e4");
    let nested = |count: usize| {
        let mut moves = vec![e4];
        moves.extend([ENCODE_START_MARKER, e4].repeat(count));
        moves.extend(vec![ENCODE_END_MARKER; count]);
        moves
    };

    let game = decode(&nested(100), &[]).unwrap();
    assert_eq!(
        game.moves.iter().filter(|node| node.san == "e4").count(),
        101
    );

    match decode(&nested(101), &[]) {
        Err(Error::Decode(message)) => {
            assert_eq!(message, "Variation nested more than 100 levels deep");
        }
        other => panic!("expected a decode error, got {other:?}"),
    }
}

#[test]
fn test_variation_count_limit_in_scid() {
    let (e4, d4) = (move_byte("e2e4"), move_byte("d2d4"));
    let siblings = |count: usize| {
        let mut moves = vec![e4];
        moves.extend([ENCODE_START_MARKER, d4, ENCODE_COMMENT, ENCODE_END_MARKER].repeat(count));
        moves
    };
    let comments: Vec<String> = (1..=256).map(|n| format!("v{n}")).collect();
    let comments: Vec<&str> = comments.iter().map(String::as_str).collect();

    let game = decode(&siblings(255), &comments[..255]).unwrap();
    assert_eq!(game.moves[1].num_variations, 255);
    let last = game.moves.iter().rfind(|node| node.san == "d4").unwrap();
    assert_eq!(last.comment.as_deref(), Some("v255"));

    match decode(&siblings(256), &comments) {
        Err(Error::Decode(message)) => {
            assert_eq!(message, "More than 255 variations of one move");
        }
        other => panic!("expected a decode error, got {other:?}"),
    }
}

#[test]
fn test_add_variation_stops_at_255() {
    let mut game = parse_single_game(&pgn_game(&sibling_movetext(255))).unwrap();
    game.current_move = Some(1);
    assert!(matches!(game.add_variation(), Err(Error::Decode(_))));
    assert_eq!(game.moves[1].num_variations, 255);
}
