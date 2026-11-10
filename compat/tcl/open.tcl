# Opens each database with SCID vs. PC and prints its number of games.
# Exits with 1 if one can't be opened.
#
#     tcscid compat/tcl/open.tcl <database>...

set status 0
foreach path $argv {
    if {[catch {sc_base open $path} base]} {
        puts "$path: cannot open: $base"
        set status 1
    } else {
        puts "$path: [sc_base numGames $base] games"
        sc_base close $base
    }
}
exit $status
