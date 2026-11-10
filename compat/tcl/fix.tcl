# Rewrites a database's index and name files with SCID vs. PC's repair of a
# corrupted base (`sc_base fixCorrupted`), which counts each name's uses.
# compat/run repairs the databases with names that aren't UTF-8 this way:
# the reference converter that wrote them counts no names.
#
#     tcscid compat/tcl/fix.tcl <database>

sc_base fixCorrupted [lindex $argv 0]
