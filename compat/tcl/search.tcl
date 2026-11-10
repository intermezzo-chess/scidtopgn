# Runs SCID vs. PC's board search (`sc_search board`) and tree search
# (`sc_tree search`) for each position in a file of FENs, one per line, and
# prints the number of games each finds. With `write-cache`, it then writes
# the database's tree cache file (.stc), as SCID does when it closes a base.
#
#     tcscid compat/tcl/search.tcl <positions file> <database> [write-cache]

lassign $argv positions path cache
set base [sc_base open $path]
sc_base switch $base
set file [open $positions]
set fens [split [string trim [read $file]] "\n"]
close $file
foreach fen $fens {
    if {$fen eq ""} continue
    sc_game new
    sc_game startBoard $fen
    sc_search board RESET Exact false false
    set board [sc_filter count]
    set tree [sc_tree search -time 0 -epd 0 -adjust 1 -base $base]
    # SCID leaves out the total when no game reaches the position.
    if {![regexp {TOTAL:\s+(\d+)} $tree -> total]} { set total 0 }
    puts "board=$board tree=$total $fen"
}
if {$cache eq "write-cache"} { sc_tree write $base }
sc_base close $base
