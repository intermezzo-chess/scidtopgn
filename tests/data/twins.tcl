# Runs SCID vs. PC's `sc_base duplicates` on copies of the `twins` database
# with each set of options below, and prints the games it deletes, counting
# from 1, and the ratings of each game afterwards. Run it with `tcscid`:
#
#     tcscid tests/data/twins.tcl tests/data/twins <scratch directory>
#
# With `-eco yes`, SCID compares uninitialized bytes for two games without an
# ECO code, so only the games with one, 46 to 49, show what `eco` does.

set source [lindex $argv 0]
set scratch [lindex $argv 1]

# The GUI's defaults, from `tcl/start.tcl`.
set gui {-players yes -colors yes -event no -site yes -round yes -year yes
    -month yes -day no -result no -eco no -moves yes -skipshort yes
    -comments yes -variations yes -delete shorter}

set runs [list \
    gui $gui \
    older [concat $gui -delete older] \
    newer [concat $gui -delete newer] \
    command {} \
    placeholders [concat $gui -skipshort no] \
    every [concat $gui -event yes -day yes -result yes -skipshort no] \
    eco [concat $gui -eco yes] \
    fewest {-players no -colors no -event no -site no -round no -year no
        -month no -day no -result no -eco no -moves yes -skipshort no
        -comments no -variations no} \
    unprotected [concat $gui -comments no -variations no -copyratings yes] \
    nomoves [concat $gui -moves no -skipshort no] \
]

foreach {name options} $runs {
    set path [file join $scratch $name]
    foreach ext {si4 sn4 sg4} {
        file copy -force $source.$ext $path.$ext
    }
    set base [sc_base open $path]
    set count [sc_base duplicates {*}$options]
    set deleted {}
    set ratings {}
    for {set game 1} {$game <= [sc_base numGames]} {incr game} {
        if {[sc_game flag delete $game]} {
            lappend deleted $game
        }
        sc_game load $game
        set white [sc_game tags get WhiteElo]
        set black [sc_game tags get BlackElo]
        if {$white != 0 || $black != 0} {
            lappend ratings "$game:$white/$black"
        }
    }
    puts "$name ($count): $deleted"
    puts "$name ratings: $ratings"
    sc_base close
}
