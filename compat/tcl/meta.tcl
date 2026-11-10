# Sets, or prints, the header fields and game flags that compat/run checks
# an append keeps. PGN can't record them, so SCID vs. PC sets them.
#
#     tcscid compat/tcl/meta.tcl set <database>
#     tcscid compat/tcl/meta.tcl get <database> <games>
#
# `set` flags the first three games, or as many as there are. `get` prints
# the flags of the first three of the first <games> games.

lassign $argv action path games
set base [sc_base open $path]
sc_base switch $base
if {$games eq ""} { set games [sc_base numGames $base] }
set games [expr {min(3, $games)}]
# Each game's flags: W is White opening, 1 is custom flag 1, and so on.
set flags {1 {W 1} 2 {E 3} 3 {T 6}}

if {$action eq "set"} {
    sc_base description "Checked by compat/run"
    sc_base type $base 5
    sc_base autoload $games
    sc_game flag 1 setdescription "Prepared"
    sc_game flag 6 setdescription "Lost"
    foreach {game letters} $flags {
        if {$game > $games} break
        foreach flag $letters { sc_game flag $flag $game 1 }
    }
} else {
    puts "description=\"[sc_base description]\" type=[sc_base type $base] autoload=[sc_base autoload]"
    foreach custom {1 2 3 4 5 6} {
        puts "flag $custom=\"[sc_game flag $custom description]\""
    }
    for {set game 1} {$game <= $games} {incr game} {
        set on {}
        foreach flag {D W B M E N P T Q K ! ? U 1 2 3 4 5 6} {
            if {[sc_game flag $flag $game]} { append on $flag }
        }
        puts "game $game flags=$on"
    }
}
sc_base close $base
