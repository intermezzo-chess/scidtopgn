// Prints how SCID's PGN parser reads each game in a PGN file: the parse
// result, the game as SCID would store it, and the messages SCID logged for
// it. docs/PGN_IMPORT.md compares this output with scidtopgn's behavior.
//
// Build and run from the repository root:
//
//   c++ -std=c++17 -Wno-register -w -Iscid-cpp/scid \
//       scid-cpp/tools/pgn_probe.cpp scid-cpp/scid/{bytebuf,date,dstring,game,gfile,index,matsig,mfile,misc,movelist,myassert,namebase,pgnparse,position,stralloc,textbuf}.cpp \
//       -lz -o pgn_probe
//   ./pgn_probe tests/data/notation.pgn

#include "game.h"
#include "pgnparse.h"
#include "textbuf.h"

#include <cstdio>
#include <cstring>
#include <fstream>
#include <sstream>
#include <string>

int main(int argc, char ** argv) {
    if (argc != 2) {
        std::fprintf(stderr, "usage: %s <pgn-file>\n", argv[0]);
        return 1;
    }
    std::ifstream file(argv[1], std::ios::binary);
    if (!file) {
        std::fprintf(stderr, "cannot read %s\n", argv[1]);
        return 1;
    }
    std::stringstream contents;
    contents << file.rdbuf();
    std::string pgn = contents.str();

    PgnParser parser(pgn.c_str());
    size_t messagesSeen = 0;
    for (int number = 1;; number++) {
        Game game;
        game.Clear();
        errorT err = parser.ParseGame(&game);
        if (err == ERROR_NotFound) { break; }

        TextBuffer text;
        text.SetBufferSize(1 << 20);
        game.SetPgnStyle(PGN_STYLE_TAGS | PGN_STYLE_COMMENTS | PGN_STYLE_VARS, true);
        game.WriteToPGN(&text);

        // One line per game, without the tags SCID writes as "?".
        std::stringstream lines(text.GetBuffer());
        std::string line, out;
        while (std::getline(lines, line)) {
            if (line.empty() || line.find(" \"?\"]") != std::string::npos
                || line.find("[Date \"????.??.??\"]") != std::string::npos) {
                continue;
            }
            out += (out.empty() ? "" : " ") + line;
        }
        std::printf("game %d: %s | %s\n", number,
                    err == OK ? "OK" : "ERROR (SCID skips this game)", out.c_str());

        const char * messages = parser.ErrorMessages();
        size_t length = std::strlen(messages);
        if (length > messagesSeen) {
            std::string newMessages(messages + messagesSeen, length - messagesSeen);
            std::stringstream logged(newMessages);
            while (std::getline(logged, line)) {
                std::printf("    logged: %s\n", line.c_str());
            }
            messagesSeen = length;
        }
    }
    return 0;
}
