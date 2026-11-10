# Sets or clears one flag of some games with SCID vs. PC, as `sc_game flag`
# names it. compat/run deletes games this way, to compare with scidtopgn.
#
#     tcscid compat/tcl/flag.tcl <database> <flag> <0|1> <game>...

set games [lassign $argv path flag value]
set base [sc_base open $path]
sc_base switch $base
foreach game $games { sc_game flag $flag $game $value }
sc_base close $base
