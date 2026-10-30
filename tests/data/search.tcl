# Sets SCID's flags on some games of the `search` database, which PGN can't
# record, and then runs SCID vs. PC's `sc_search header` with each set of
# options below, and prints the games it selects, counting from 1. Run it
# with `tcscid`:
#
#     tcscid tests/data/search.tcl tests/data/search
#
# Setting a flag that is already set changes nothing, so the database stays
# the same after the first run.

set path [lindex $argv 0]
set base [sc_base open $path]

# The flags of each game, as `sc_game flag` names them: D is deleted, W to U
# are SCID's flags, and 1 to 6 the custom flags.
set flags {
    1 {W 1} 2 {B 2} 3 {M 3} 4 {E 4} 5 {N 5} 6 {D} 7 {P W} 8 {T} 9 {K}
    10 {Q} 11 {!} 12 {D} 19 {?} 20 {U 6}
}
foreach {game letters} $flags {
    foreach flag $letters {
        sc_game flag $flag $game 1
    }
}

set runs {
    default {}
    white {-white Carlsen}
    white-case {-white CARLSEN}
    white-spaces {-white "Carl sen"}
    white-space-end {-white "Carlsen "}
    white-space-start {-white " Carlsen"}
    white-only-spaces {-white "  "}
    white-empty {-white ""}
    white-comma {-white "Doe, John"}
    white-not-ascii {-white müller}
    black-not-ascii {-black müller}
    wild-prefix {-white {"Carlsen*"}}
    wild-case {-white {"carlsen*"}}
    wild-whole {-white {"Carlsen"}}
    wild-question {-white {"Anand, ?iswanathan"}}
    wild-class {-white {"[A-D]*"}}
    wild-class-reversed {-white {"[D-A]*"}}
    wild-class-list {-white {"[CS]*"}}
    wild-escape {-white {"Smith \[A\]"}}
    wild-bracket {-white {"Smith [A]"}}
    wild-not-ascii {-black {"M?ller, Hans"}}
    wild-not-ascii-upper {-white {"M?LLER*"}}
    wild-empty {-white {""}}
    wild-quote {-white {"}}
    wild-unknown {-white {"?"}}
    wild-star {-white {"*"}}
    wild-unclosed {-white {"[A-C*"}}
    black {-black Carlsen}
    event {-event championship}
    event-wild {-event {"World Championship*"}}
    site {-site "wijk aan zee"}
    round {-round 1}
    round-wild {-round {"1"}}
    date {-date {2023.01.01 2023.12.31}}
    date-year {-date {2023.00.00 2023.12.31}}
    date-inverted {-date {2023.12.31 2023.01.01}}
    date-unknown {-date {0000.00.00 0000.00.00}}
    results-white {-results {1 0 0 0}}
    results-draw-none {-results {0 1 0 1}}
    welo {-welo {2840 4000}}
    welo-zero {-welo {0 0}}
    belo {-belo {2800 2900}}
    delo {-delo {-20 20}}
    delo-inverted {-delo {20 -20}}
    delo-positive {-delo {100 4000}}
    eco {-eco {B20 B99 Yes}}
    eco-exclude {-eco {B20 B99 No}}
    eco-subcodes {-eco {B07 B07 No}}
    eco-subcode-upper {-eco {B07 B07a No}}
    eco-subcode-lower {-eco {B07z B08 No}}
    eco-inverted {-eco {B99 B20 No}}
    length {-length {5 10}}
    length-zero {-length {0 0}}
    length-long {-length {1000 1023}}
    length-max {-length {0 1023}}
    to-move-white {-toMove w}
    to-move-black {-toMove b}
    game-number {-gameNumber {5 10}}
    game-number-end {-gameNumber {-5 -1}}
    game-number-swapped {-gameNumber {10 5}}
    game-number-clamped {-gameNumber {-100 3}}
    game-number-past {-gameNumber {20 100}}
    flip-results {-white Carlsen -results {1 0 0 0} -flip 1}
    flip-elo {-welo {2750 4000} -belo {0 2700} -flip 1}
    flip-elo-diff {-delo {30 4000} -flip 1}
    flip-to-move {-white Carlsen -toMove w -flip 1}
    flip-names {-white {"Smith*"} -black {"Smith, Bob"} -flip 1}
    combined {-white Carlsen -date {2019.01.01 2023.12.31} -eco {B20 B99 No}
        -welo {2800 4000}}
}

foreach flag {
    fStdStart fPromotions fUnderPromo fComments fVariations fAnnotations
    fDelete fWhiteOpening fBlackOpening fMiddlegame fEndgame fNovelty
    fPawnStructure fTactics fKingside fQueenside fBrilliancy fBlunder fUser
    fCustom1 fCustom2 fCustom3 fCustom4 fCustom5 fCustom6
} {
    lappend runs $flag-yes [list -$flag yes] $flag-no [list -$flag no]
}

foreach {name options} $runs {
    sc_search header {*}$options
    set games {}
    for {set game 1} {$game <= [sc_base numGames]} {incr game} {
        if {[sc_filter value $game]} {
            lappend games $game
        }
    }
    puts "$name: $games"
}
sc_base close
