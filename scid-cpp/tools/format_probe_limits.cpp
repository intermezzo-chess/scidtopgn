// Exercises limits of SCID's code that take too large a database to reach
// through its files. docs/SCID_FORMAT_AUDIT.md says how to build and run it.
//
//     format_probe_limits <scratch name file, without .sn4> <database whose first bytes are out of order>

#include <cstdio>
#include <cstring>

#define private public
#define protected public
#include "common.h"
#include "game.h"
#include "index.h"
#include "namebase.h"

static int failures = 0;

static void check(bool ok, const char* what) {
    printf("%s: %s\n", ok ? "yes" : "NO", what);
    if (!ok) failures++;
}

int main(int argc, char* argv[]) {
    if (argc != 3) return 2;

    Index idx;
    IndexEntry ie;
    ie.Init();
    gameNumberT number;
    idx.Header.numGames = MAX_GAMES;
    check(idx.AddGame(&number, &ie) == ERROR_IndexFull, "no game is added after 16,777,214");
    idx.Header.numGames = 0;

    NameBase nb;
    idNumberT id;
    const uint most[NUM_NAME_TYPES] = {1048575, 524287, 524287, 262143};
    for (nameT t = 0; t < NUM_NAME_TYPES; t++) {
        nb.Header.numNames[t] = most[t];
        check(nb.AddName(t, "x", &id) == ERROR_NameBaseFull, "no name is added after the most");
        nb.Header.numNames[t] = 0;
    }
    nb.AddName(NAME_PLAYER, "A", &id);
    nb.IncFrequency(NAME_PLAYER, id, 300);
    nb.IncFrequency(NAME_PLAYER, id, -300);
    check(nb.GetFrequency(NAME_PLAYER, id) == 0 && nb.Header.maxFrequency[NAME_PLAYER] == 300,
          "the maximum frequency isn't lowered with a frequency");
    nb.ZeroAllFrequencies(NAME_PLAYER);
    check(nb.Header.maxFrequency[NAME_PLAYER] == 300, "nor when all are set to 0");

    NameBase order;
    order.SetFileName(argv[2]);
    check(order.ReadNameFile() == OK, "a file whose first bytes are out of order opens");
    check(order.GetNumNames(NAME_PLAYER) == 2 && order.FindExactName(NAME_PLAYER, "A", &id) != OK &&
              order.FindExactName(NAME_PLAYER, "B", &id) != OK,
          "its names aren't found");

    NameBase big;
    big.AddName(NAME_PLAYER, "A", &id);
    for (uint i = 0; i < 1u << 23; i++) big.IncFrequency(NAME_PLAYER, id, 2);
    big.SetFileName(argv[1]);
    check(big.WriteNameFile() == OK, "a name with frequency 2^24 is written");
    NameBase again;
    again.SetFileName(argv[1]);
    check(again.ReadNameFile() == OK && again.Header.maxFrequency[NAME_PLAYER] == 0 &&
              strcmp(again.GetName(NAME_PLAYER, 0), "A") != 0,
          "and read back without an error, with the maximum 0 and another name");

    Index header;
    header.CreateMemoryOnly();
    memset(header.Header.description, 0xa5, sizeof header.Header.description);
    header.SetDescription("X");
    check(header.Header.description[2] == 0 && (byte)header.Header.description[107] == 0xa5,
          "setting a description clears all but its field's last byte");
    return failures ? 1 : 0;
}
