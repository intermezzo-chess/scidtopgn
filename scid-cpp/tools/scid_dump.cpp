// Prints every index entry and name of a SCID database, then checks the
// database with SCID's own code. compat/run uses it; see docs/SCID_COMPAT.md.
//
// Build it with CMake, as the reference converter (CONTRIBUTING.md), and run
// it from the repository root:
//
//   scid-cpp/build/scid-dump tests/data/five
//
// Output lines:
//   H ...   index and name file headers
//   N ...   one name: type, id, frequency, name
//   G ...   one index entry, with names resolved
//   ! ...   a failed check
//   S ...   summary of check counts
//
// Checks:
//   - Each game record reads and decodes.
//   - SCID's Game::Encode of the decoded game gives the same record bytes,
//     and the same derived index fields (counts, flags, material, home pawn
//     data, half-moves) as the stored entry.
//   - Each entry's name IDs are in range.
//   - Each name's stored frequency equals the number of entries that use it
//     (deleted games included, as in SCID).
//   - FindExactName finds each name at its own ID.
//
// It exits with 1 if a check fails or the database can't be opened, and with
// 2 on a usage error.

#include <cstdarg>
#include <cstdio>
#include <cstring>
#include <string>
#include <vector>

#include "common.h"
#include "game.h"
#include "gfile.h"
#include "index.h"
#include "namebase.h"

static const char* TYPE_NAMES[NUM_NAME_TYPES] = {"player", "event", "site", "round"};
static uint failures = 0;

static void fail(const char* fmt, ...) {
    va_list ap;
    va_start(ap, fmt);
    printf("! ");
    vprintf(fmt, ap);
    printf("\n");
    va_end(ap);
    failures++;
}

static std::string hex(const byte* data, uint len) {
    std::string out;
    char buf[3];
    for (uint i = 0; i < len; i++) {
        snprintf(buf, sizeof buf, "%02x", data[i]);
        out += buf;
    }
    return out;
}

int main(int argc, char* argv[]) {
    if (argc != 2) {
        fprintf(stderr, "usage: scid-dump <database>\n");
        return 2;
    }
    const char* db = argv[1];

    Index idx;
    NameBase nb;
    GFile gf;
    idx.SetFileName(db);
    nb.SetFileName(db);
    if (idx.OpenIndexFile(FMODE_ReadOnly) != OK) {
        printf("! cannot open index file\n");
        return 1;
    }
    if (nb.ReadNameFile() != OK) {
        printf("! cannot read name file\n");
        return 1;
    }
    if (gf.Open(db, FMODE_ReadOnly) != OK) {
        printf("! cannot open game file\n");
        return 1;
    }

    gameNumberT numGames = idx.GetNumGames();
    printf("H index version=%u games=%u type=%u autoload=%u description=\"%s\"\n",
           (uint)idx.GetVersion(), (uint)numGames, idx.GetType(), (uint)idx.GetAutoLoad(),
           idx.GetDescription());
    for (uint c = 1; c <= CUSTOM_FLAG_MAX; c++) {
        char desc[CUSTOM_FLAG_DESC_LENGTH + 1];
        idx.GetCustomFlagDesc(desc, c);
        if (desc[0]) printf("H flag%u=\"%s\"\n", c, desc);
    }
    for (nameT nt = 0; nt < NUM_NAME_TYPES; nt++) {
        printf("H names %s=%u\n", TYPE_NAMES[nt], (uint)nb.GetNumNames(nt));
    }

    // Name uses counted from the index, to check stored frequencies.
    std::vector<std::vector<uint>> uses(NUM_NAME_TYPES);
    for (nameT nt = 0; nt < NUM_NAME_TYPES; nt++) uses[nt].assign(nb.GetNumNames(nt), 0);

    uint records = 0, recordDiffs = 0, fieldDiffs = 0, unreadable = 0, deleted = 0;
    for (gameNumberT g = 0; g < numGames; g++) {
        uint n = g + 1;
        IndexEntry* ie = idx.FetchEntry(g);
        if (!ie) {
            fail("game %u: no index entry", n);
            continue;
        }
        if (ie->GetDeleteFlag()) deleted++;

        idNumberT ids[NUM_NAME_TYPES + 1] = {ie->GetWhite(), ie->GetBlack(), ie->GetEvent(),
                                             ie->GetSite(), ie->GetRound()};
        nameT types[NUM_NAME_TYPES + 1] = {NAME_PLAYER, NAME_PLAYER, NAME_EVENT, NAME_SITE,
                                           NAME_ROUND};
        bool idsOk = true;
        for (int i = 0; i < 5; i++) {
            if (ids[i] >= nb.GetNumNames(types[i])) {
                fail("game %u: %s id %u out of range (%u names)", n, TYPE_NAMES[types[i]],
                     (uint)ids[i], (uint)nb.GetNumNames(types[i]));
                idsOk = false;
            } else {
                uses[types[i]][ids[i]]++;
            }
        }

        char flags[32];
        ie->GetFlagStr(flags, NULL);
        printf("G %u white=\"%s\" black=\"%s\" event=\"%s\" site=\"%s\" round=\"%s\" "
               "date=%u eventdate=%u result=%u welo=%u/%u belo=%u/%u eco=%u halfmoves=%u "
               "flags=%s start=%d promo=%d upromo=%d vars=%u comments=%u nags=%u "
               "matsig=%06x storedline=%u homepawn=%s length=%u offset=%u\n",
               n, idsOk ? ie->GetWhiteName(&nb) : "?", idsOk ? ie->GetBlackName(&nb) : "?",
               idsOk ? ie->GetEventName(&nb) : "?", idsOk ? ie->GetSiteName(&nb) : "?",
               idsOk ? ie->GetRoundName(&nb) : "?", (uint)ie->GetDate(),
               (uint)ie->GetEventDate(), (uint)ie->GetResult(), (uint)ie->GetWhiteElo(),
               (uint)ie->GetWhiteRatingType(), (uint)ie->GetBlackElo(),
               (uint)ie->GetBlackRatingType(), (uint)ie->GetEcoCode(),
               (uint)ie->GetNumHalfMoves(), flags, ie->GetStartFlag(), ie->GetPromotionsFlag(),
               ie->GetUnderPromoFlag(), ie->GetVariationCount(), ie->GetCommentCount(),
               ie->GetNagCount(), (uint)ie->GetFinalMatSig(), (uint)ie->GetStoredLineCode(),
               hex(ie->GetHomePawnData(), HPSIG_SIZE).c_str(), ie->GetLength(),
               ie->GetOffset());
        if (!idsOk) continue;

        ByteBuffer bb;
        if (gf.ReadGame(&bb, ie->GetOffset(), ie->GetLength()) != OK) {
            fail("game %u: cannot read record", n);
            unreadable++;
            continue;
        }
        std::string stored((const char*)bb.GetBuffer(), bb.GetByteCount());
        Game game;
        if (game.Decode(&bb, GAME_DECODE_ALL) != OK) {
            fail("game %u: cannot decode record", n);
            unreadable++;
            continue;
        }
        if (game.LoadStandardTags(ie, &nb) != OK) {
            fail("game %u: cannot load tags", n);
            unreadable++;
            continue;
        }
        records++;

        // Re-encode with SCID's encoder into a copy of the stored entry.
        IndexEntry fresh = *ie;
        ByteBuffer out;
        out.SetBufferSize(stored.size() + 65536);
        if (game.Encode(&out, &fresh) != OK) {
            fail("game %u: SCID cannot re-encode the game", n);
            continue;
        }
        std::string encoded((const char*)out.GetBuffer(), out.GetByteCount());
        if (encoded != stored) {
            recordDiffs++;
            size_t at = 0;
            while (at < stored.size() && at < encoded.size() && stored[at] == encoded[at]) at++;
            fail("game %u: record differs from SCID's encoding (stored %zu bytes, SCID %zu, "
                 "first difference at byte %zu)",
                 n, stored.size(), encoded.size(), at);
        }

#define CHECK_FIELD(name, getter)                                                       \
    if ((uint)ie->getter() != (uint)fresh.getter()) {                                   \
        fieldDiffs++;                                                                   \
        fail("game %u: %s stored=%u scid=%u", n, name, (uint)ie->getter(),              \
             (uint)fresh.getter());                                                     \
    }
        // The index file keeps 10 bits of the half-move count, as SCID
        // writes it, so a game over 1,023 half-moves reads back cut.
        if ((uint)ie->GetNumHalfMoves() != (uint)(fresh.GetNumHalfMoves() & 0x3FF)) {
            fieldDiffs++;
            fail("game %u: halfmoves stored=%u scid=%u", n, (uint)ie->GetNumHalfMoves(),
                 (uint)(fresh.GetNumHalfMoves() & 0x3FF));
        }
        CHECK_FIELD("start", GetStartFlag);
        CHECK_FIELD("promo", GetPromotionsFlag);
        CHECK_FIELD("upromo", GetUnderPromoFlag);
        CHECK_FIELD("vars", GetVariationCount);
        CHECK_FIELD("comments", GetCommentCount);
        CHECK_FIELD("nags", GetNagCount);
        CHECK_FIELD("matsig", GetFinalMatSig);
        CHECK_FIELD("result", GetResult);
        CHECK_FIELD("eco", GetEcoCode);
        if (memcmp(ie->GetHomePawnData(), fresh.GetHomePawnData(), HPSIG_SIZE) != 0) {
            fieldDiffs++;
            fail("game %u: homepawn stored=%s scid=%s", n,
                 hex(ie->GetHomePawnData(), HPSIG_SIZE).c_str(),
                 hex(fresh.GetHomePawnData(), HPSIG_SIZE).c_str());
        }
    }

    uint freqDiffs = 0, lookupFails = 0;
    for (nameT nt = 0; nt < NUM_NAME_TYPES; nt++) {
        for (idNumberT id = 0; id < nb.GetNumNames(nt); id++) {
            const char* name = nb.GetName(nt, id);
            uint freq = nb.GetFrequency(nt, id);
            printf("N %s %u freq=%u \"%s\"\n", TYPE_NAMES[nt], (uint)id, freq, name);
            if (freq != uses[nt][id]) {
                freqDiffs++;
                fail("%s %u \"%s\": frequency stored=%u used=%u", TYPE_NAMES[nt], (uint)id, name,
                     freq, uses[nt][id]);
            }
            idNumberT found;
            if (nb.FindExactName(nt, name, &found) != OK || found != id) {
                lookupFails++;
                fail("%s %u \"%s\": FindExactName gives %d", TYPE_NAMES[nt], (uint)id, name,
                     nb.FindExactName(nt, name, &found) == OK ? (int)found : -1);
            }
        }
    }

    printf("S games=%u deleted=%u decoded=%u unreadable=%u record_diffs=%u field_diffs=%u "
           "freq_diffs=%u lookup_fails=%u failures=%u\n",
           (uint)numGames, deleted, records, unreadable, recordDiffs, fieldDiffs, freqDiffs,
           lookupFails, failures);
    gf.Close();
    return failures ? 1 : 0;
}
