# Prints the positions compat/run searches for, as FEN: the position after
# 2, 6, and 12 half-moves of each game's main line, each once.
#
#     tcscid compat/tcl/positions.tcl <database>

set base [sc_base open [lindex $argv 0]]
sc_base switch $base
set seen [dict create]
for {set game 1} {$game <= [sc_base numGames $base]} {incr game} {
    sc_game load $game
    set ply 0
    foreach target {2 6 12} {
        while {$ply < $target && ![sc_pos isAt end]} {
            sc_move forward
            incr ply
        }
        if {$ply < $target} break
        set fen [sc_pos fen]
        if {![dict exists $seen $fen]} {
            dict set seen $fen 1
            puts $fen
        }
    }
}
sc_base close $base
