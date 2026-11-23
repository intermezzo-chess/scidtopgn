// Prints everything in a SCID database as SCID's own code reads it: the
// headers, each index entry, each name, and each game's record. The output
// is the same as that of format_probe.py, which reads the files only as
// docs/SCID_FORMAT.md and the documents it links to describe them, so the
// two can be compared. docs/SCID_FORMAT_AUDIT.md says how to build and run
// both, and what they were run on.
//
// It exits with 1 if the database can't be opened, printing SCID's error
// number. A game that can't be read is printed as its error.

#include <cstdio>
#include <cstring>
#include <string>

// The probe prints fields SCID's classes don't give access to.
#define private public
#define protected public
#include "common.h"
#include "game.h"
#include "gfile.h"
#include "index.h"
#include "namebase.h"

static std::string hex(const char* s, size_t len) {
    std::string out;
    char buf[3];
    for (size_t i = 0; i < len; i++) {
        snprintf(buf, sizeof buf, "%02x", (unsigned char)s[i]);
        out += buf;
    }
    return out.empty() ? "-" : out;
}

static std::string hex(const char* s) { return hex(s, strlen(s)); }

// Prints a line of moves and its variations. `start` is its start marker.
static void printLine(moveT* start) {
    if (start->comment) printf(" {%s}", hex(start->comment).c_str());
    for (moveT* m = start->next; m->marker != END_MARKER; m = m->next) {
        simpleMoveT& sm = m->moveData;
        if (sm.from == sm.to) {
            printf(" null");
        } else {
            printf(" %u-%u", sm.from, sm.to);
            if (sm.promote != EMPTY) printf("=%u", piece_Type(sm.promote));
        }
        for (int i = 0; i < m->nagCount; i++) printf(" $%u", m->nags[i]);
        if (m->comment) printf(" {%s}", hex(m->comment).c_str());
        moveT* var = m->varChild;
        for (int i = 0; i < m->numVariations; i++, var = var->varChild) {
            printf(" (");
            printLine(var);
            printf(" )");
        }
    }
}

int main(int argc, char* argv[]) {
    if (argc != 2) {
        fprintf(stderr, "usage: format_probe <database>\n");
        return 2;
    }
    Index idx;
    idx.SetFileName(argv[1]);
    errorT err = idx.OpenIndexFile(FMODE_ReadOnly);
    if (err != OK) { printf("index error %u\n", err); return 1; }
    NameBase nb;
    nb.SetFileName(argv[1]);
    err = nb.ReadNameFile();
    if (err != OK) { printf("name error %u\n", err); return 1; }
    GFile gf;
    err = gf.Open(argv[1], FMODE_ReadOnly);
    if (err != OK) { printf("game error %u\n", err); return 1; }

    indexHeaderT& h = idx.Header;
    printf("index version=%u type=%u games=%u autoload=%u description=%s\n", h.version,
           h.baseType, h.numGames, h.autoLoad, hex(h.description).c_str());
    for (uint i = 0; i < CUSTOM_FLAG_MAX; i++) {
        printf("custom %u %s\n", i + 1, hex(h.customFlagDesc[i]).c_str());
    }
    printf("names time=%u", nb.Header.timeStamp);
    for (nameT t = 0; t < NUM_NAME_TYPES; t++) {
        printf(" %u:%u", nb.Header.numNames[t], nb.Header.maxFrequency[t]);
    }
    printf("\n");
    for (nameT t = 0; t < NUM_NAME_TYPES; t++) {
        nb.IterateStart(t);
        idNumberT id;
        while (nb.Iterate(t, &id) == OK) {
            printf("name %u id=%u frequency=%u %s\n", t, id, nb.GetFrequency(t, id),
                   hex(nb.GetName(t, id)).c_str());
        }
    }

    Game g;
    ByteBuffer bb;
    bb.SetBufferSize(BBUF_SIZE);
    for (gameNumberT n = 0; n < idx.GetNumGames(); n++) {
        IndexEntry ie = *idx.FetchEntry(n);
        printf("entry %u offset=%u length=%u flags=%04x custom=%02x names=%u,%u,%u,%u,%u", n + 1,
               ie.GetOffset(), ie.GetLength(), ie.Flags, ie.Length_High & 0x3F, ie.GetWhite(),
               ie.GetBlack(), ie.GetEvent(), ie.GetSite(), ie.GetRound());
        dateT e = ie.GetEventDate();
        printf(" result=%u counts=%u,%u,%u eco=%u date=%u.%u.%u event=%u.%u.%u", ie.GetResult(),
               ie.GetVariationCount(), ie.GetCommentCount(), ie.GetNagCount(), ie.GetEcoCode(),
               ie.GetYear(), ie.GetMonth(), ie.GetDay(), date_GetYear(e), date_GetMonth(e),
               date_GetDay(e));
        printf(" elo=%u/%u,%u/%u material=%06x line=%u moves=%u pawns=%s\n", ie.GetWhiteElo(),
               ie.GetWhiteRatingType(), ie.GetBlackElo(), ie.GetBlackRatingType(),
               ie.GetFinalMatSig(), ie.GetStoredLineCode(), ie.GetNumHalfMoves(),
               hex((const char*)ie.GetHomePawnData(), HPSIG_SIZE).c_str());
        bb.Empty();
        err = gf.ReadGame(&bb, ie.GetOffset(), ie.GetLength());
        if (err != OK) { printf("game %u read error %u\n", n + 1, err); continue; }
        bb.BackToStart();
        g.Clear();
        err = g.Decode(&bb, GAME_DECODE_ALL);
        if (err != OK) { printf("game %u decode error %u\n", n + 1, err); continue; }
        printf("game %u end=%u", n + 1, bb.ReadPos);
        if (g.EventDate != ZERO_DATE) {
            printf(" eventdate=%u.%u.%u", date_GetYear(g.EventDate), date_GetMonth(g.EventDate),
                   date_GetDay(g.EventDate));
        }
        for (uint i = 0; i < g.NumTags; i++) {
            printf(" [%s %s]", hex(g.TagList[i].tag).c_str(), hex(g.TagList[i].value).c_str());
        }
        printf(" flags=%u", (g.NonStandardStart ? 1 : 0) | (g.PromotionsFlag ? 2 : 0) |
                                (g.UnderPromosFlag ? 4 : 0));
        if (g.NonStandardStart) {
            char fen[256];
            g.StartPos->PrintFEN(fen, FEN_ALL_FIELDS);
            printf(" fen=%s", hex(fen).c_str());
        }
        printLine(g.FirstMove);
        printf("\n");
    }
    return 0;
}
