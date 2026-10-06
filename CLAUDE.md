# Claude Code Context - Cochonnet

This file provides context for Claude Code to efficiently work with this codebase.

## Architecture Overview

This is a **Tauri 2** desktop application with:
- **Frontend**: React 19 + TypeScript + Vite + Tailwind CSS 4
- **Backend**: Rust with SQLite (rusqlite)
- **State Management**: Zustand
- **PDF Generation**: @react-pdf/renderer
- **i18n**: react-i18next (English and French)

## Key Files by Feature

### Tournament Configuration
- `src/features/tournaments/TournamentForm.tsx` - Create/edit tournament form
- `src-tauri/src/commands/tournaments.rs` - Tournament CRUD operations
- `src/types/index.ts` - Tournament interface definition

### Team Management
- `src/features/teams/TeamsList.tsx` - Team list and import UI
- `src-tauri/src/commands/teams.rs` - Team operations including CSV import
- CSV import expects: captain, player2, player3 (optional), region (optional), club (optional)

### Qualifying Rounds
- `src/features/pairing/QualifyingRounds.tsx` - Main qualifying rounds view
- `src/features/pairing/StandingsTable.tsx` - Standings display table
- `src-tauri/src/commands/qualifying.rs` - Core pairing and scoring logic

**Supported Pairing Methods:**

| Method | Generation | Ranking | Description |
|--------|------------|---------|-------------|
| **Swiss** | Round-by-round only | Buchholz | Teams with similar records play each other. Prior round must complete before generating next. |
| **Swiss Hotel** | All at once **or** round-by-round | Point Quotient | Random pairings with graduated constraints. |
| **Round Robin** | All at once **or** round-by-round | Point Quotient | Berger circle method - each team plays every other team. |
| **Pool Play** | Round-by-round only | Point Quotient | Fixed 3 rounds: R1 random, R2 winners vs winners, R3 only 1-1 teams play. Teams with 2 losses eliminated. |
| **Panaché** | All at once (redrawable) **or** round-by-round | Wins → Differential | Individual registration. Players are shuffled into fresh temporary teams each round. No bracket — one final game. |

Swiss and Pool Play build each round from the last one's results, so they can only be drawn
one at a time. The other three can be drawn either way, and `QualifyingRounds.tsx` offers both
buttons: drawing the whole schedule up front is what those formats are usually chosen for,
while one round at a time is what a director wants when the roster may still move.

That one fact — whether a format reads the scoreboard — is also what decides whether the next
round has to **wait** for the current one. `next_round_depends_on_results()` in `qualifying.rs`
is the single place it is stated, mirrored by `nextRoundDependsOnResults` in
`QualifyingRounds.tsx`, and it gates both the refusal and the button. Swiss and Pool Play wait.
The other three do not: they reshuffle against who has already met whom, and they write
`pairing_history` and `court_history` **as they draw, not as they score**, so a round drawn
mid-play still avoids repeat matchups and repeat courts. A director with a settled roster can
draw and print the next sheet while the current round is still on the ground, which is the
whole point of offering it.

### Withdrawal

A team that pulls out mid-tournament cannot be deleted — its played games and its opponents'
Buchholz depend on it — so `teams.is_withdrawn` flags it instead. The flag is forward-only:
completed results stand, the game already scheduled in the current round is left for the
operator to score however they decide, and only draws made from then on leave the team out.

Every query that *schedules* filters on `is_withdrawn = 0` (the team load in
`generate_single_round`, round robin's cycle cap, `load_players` in `panache.rs`, bracket
seeding in `brackets.rs`, and both roster-capacity checks). `get_teams` deliberately does not:
the roster page has to keep showing everyone.

### Panaché (individual format)

Unlike the other four formats, panaché registers **individuals** rather than teams. It reuses
the `teams` table for that: one row per player with `captain` set and `player2` empty, the same
shape singles registration already uses. That means `team_standings` becomes per-player standings
with no new table, and team CRUD, CSV import and the standings views all work unchanged.

What is new is the **temporary team**, drawn fresh each round:

- `panache_teams` / `panache_team_members` — the throwaway doubles or triples for one round
- `panache_sitouts` — surplus players resting that round (not a bye: no record is awarded)
- `qualifying_games.side1_id` / `side2_id` — point at temporary teams; `team1_id`/`team2_id` are NULL
- `teams.is_champion` — an expert the draw keeps off other champions' teams
- `qualifying_rounds.is_final` — the single championship game

Key points:

- **Team size** comes from the tournament `format` field (Double → 2, Triple → 3).
- **Roster capacity** is `number_of_courts × 2 × team_size`, not `× 2` (see `check_roster_capacity`
  in `teams.rs`) — a panaché game occupies a whole court.
- **Scoring** fans a temporary team's result out to every member individually
  (`apply_game_result` in `qualifying.rs`).
- **The final** records a winner but does not feed back into `team_standings`, the same way
  bracket results don't.
- **The scheduler** (`solve_panache_schedule` in `commands/panache.rs`) is a pure function:
  randomized greedy with restarts plus local search, over a weighted cost function. Constraints
  are costs, not filters, because rules phrased each as "unless unavoidable". Weights, in
  order: champions sharing a team (1000) > repeated teammates (100) > a non-champion who never
  meets a champion (50) > repeated opponents (10) > sit-out imbalance (5).

### Court Assignment

`courts.rs` holds a pure solver, in the same shape as the panaché scheduler: randomized greedy
with restarts over a weighted cost, no database access, unit-tested on its own.

Constraints are costs, not filters, because a round has to be drawn even when every choice
breaks something. A team drawing the same court **in consecutive rounds** costs 1000; the same
court **at any earlier point** costs 100. Weighting the consecutive repeat higher is what
encodes the rule: with courts to spare the solver reaches zero and nobody repeats, and when it
cannot it spends the cheap repeats first.

- Identity is the competitor, which for panaché means the **player** — the temporary team is
  gone by the next round, the person is not.
- `court_history` was written from the start and never read until now; `load_qualifying_history`
  is what it was for.
- Brackets reuse the solver per **wave** (`play_wave` in `brackets.rs`), seeded with the
  qualifying history and extended wave by wave. Bracket courts are *not* written to
  `court_history` — its `round_id` foreign-keys `qualifying_rounds` — they are derived from
  `bracket_matches` instead.
- `assign_bracket_courts` re-runs on every result, so a match whose teams have just arrived
  gets a court chosen against their history. Two things are never moved by that re-run: a
  match already played or hand-set (`bracket_matches.court_is_manual`) — it keeps its court
  and takes it out of what the rest of the wave can share — and **any wave that already has a
  score in it**, because the other games in that wave are on the ground right now, on the
  courts the printed sheet sent them to.
- Courts are editable **until the game is played**. After that the court is a record of where
  it happened, so the field is not offered and `move_game_to_court` / `move_match_to_court`
  refuse it outright. A double-booking, by contrast, is **highlighted, not refused**:
  shuffling games around means passing through one, and being blocked mid-shuffle is worse
  than the clash. Qualifying checks this client-side; brackets need
  `get_bracket_court_conflicts`, because a wave spans every bracket while the store only ever
  holds one.

**Key Functions in `courts.rs`:**
- `assign_courts()` / `assign_courts_from_slots()` - the pure draw
- `load_qualifying_history()` - the database adapter
- `court_slots()` - the courts one set of simultaneous games shares. Note it offers
  `1..=game_count`, so a venue with more courts than games only ever uses the low-numbered
  ones. Games stay compact, but rotation quality is bounded by the game count, not the court
  count - relevant if a field is ever much smaller than its venue.

Tests of the draw assert the *invariant*, and only the one that is actually reachable. "Never
the same court twice running" holds when each game has one court to avoid (fixed pairings, as
in the solver's own tests). It does not hold in general: pairings that reshuffle bring two
different courts to avoid into one game, and with only as many courts as games there may be no
assignment that satisfies everyone. Asserting it against live pairings produces a test that
passes most of the time, which is worse than no test.

**Key Functions in `qualifying.rs`:**
- `generate_swiss_pairings()` - Pairs teams by similar win records
- `generate_swiss_hotel_pairings()` - Random pairing with constraints
- `generate_pool_play_round()` - Round-specific Pool Play logic
- `calculate_buchholz_and_ranks()` - Swiss tiebreaker calculation
- `calculate_point_quotient_ranks()` - Point quotient tiebreaker calculation
- `generate_single_round()` - Draws one round. It takes no "is this one round on its own" flag:
  whether the previous round must be scored first is `next_round_depends_on_results()`'s
  answer, so the one-at-a-time and all-at-once callers are identical
- `next_round_depends_on_results()` - True for Swiss and Pool Play only. The one statement of
  which formats consume results; do not re-spell the method list anywhere else
- `complete_round()` - Score processing and rank updates. **It adds, so it must never run
  twice**: each result is `wins = wins + 1` against a running total, and a second application
  silently doubles that round - every winner in it gains a win and the field's points inflate,
  surfacing as a standings table with more wins than there were rounds and nothing in
  `qualifying_games` to explain it. `complete_round_inner` returns early when the round is
  already flagged complete, and that check is the protection: a double-clicked button, a retried
  command and a replayed backup all arrive through it. The frontend also disables the button
  while the call is in flight, because saving a 67-game round is one round-trip per game and the
  pause was long enough to click through twice
- `apply_game_result()` - Adds one result to each competitor's standing (a team, or every member of a panaché temporary team)

**Key Functions in `panache.rs`:**
- `solve_panache_schedule()` - Draws a schedule; pure, unit-tested, no DB access. Takes already
  drawn rounds as `fixed`, which is what lets one round be added at a time
- `generate_panache_rounds()` / `generate_panache_round()` / `redraw_panache_rounds()` /
  `generate_panache_final()` - Commands
- `set_team_withdrawn()` - Flags an entrant as having pulled out (lives here beside
  `set_team_champion`)

Also `check_roster_capacity()` in `teams.rs` - the format-aware entrant cap.

### Tiebreaker Algorithms

**Swiss System** (Buchholz-based):
1. Wins (descending)
2. Buchholz Score - sum of opponents' wins
3. Fine Buchholz Score - sum of opponents' Buchholz scores
4. Point Differential
5. Random tiebreaker

**Swiss Hotel / Round Robin / Pool Play** (Point Quotient-based):
1. Wins (descending)
2. **Head-to-head** - Swiss Hotel and Round Robin only (see below)
3. Point Differential
4. Point Quotient - points_for / points_against
5. Random tiebreaker

**Head-to-head** (`HeadToHead` in `qualifying.rs`) is the regulations' tiebreak for
**Round Robin and Swiss Hotel ("Rounds") only** — not Swiss System, which ranks on Buchholz,
and not Pool Play or Panaché, which the regulations do not mention. Those three pass `None`
and are untouched; `swiss_system_ignores_head_to_head` is the test that holds Swiss out.

The regulations apply it "when only 2 teams are involved", and that clause is load-bearing
rather than decorative:

- The group is a **win count**, and the rule fires only when **exactly two** teams share it
  **and** they met during the qualifiers. Then the winner of that game ranks ahead, before
  differential. Three or more teams on the same number of wins is left entirely to the
  existing rules, *even when two of them played* — `head_to_head_is_skipped_when_more_than_two_teams_are_tied`.
- That restriction is what keeps the ranking an ordering at all. Among three teams the rule
  would have to answer A beat B, B beat C, C beat A — an ordinary weekend — and no ordering
  satisfies that. Sorting by a comparator that contradicts itself leaves the published table
  depending on the order rows came out of SQLite. Restricted to a pair, there is no third team
  and the question never arises. **Do not "generalise" this to a mini-league of the tied group**
  without re-reading this paragraph: an earlier attempt did exactly that and reordered 125 of
  the 174 teams in the AIO 2024 fixture, moving 3 teams across the 32-team bracket cut.
- It reads the rounds already flagged complete **plus the round being completed right now**,
  which is not flagged until after the ranking has run. Rounds drawn ahead but unplayed are
  excluded on purpose: their scores are not in the standings, so they must not sway a tiebreak.
- Because a win count has to hold exactly two teams, the rule is **a small-field rule in
  practice**. It never fires once in AIO 2024, whose six win counts hold 17 to 39 teams each —
  which is why `the_2024_amelia_island_open_reproduces_its_published_standings` still passes
  unchanged, and is the evidence that this reading does not disturb a real event.

### Elimination Brackets
- `src/features/brackets/BracketView.tsx` - Main bracket display
- `src-tauri/src/commands/brackets.rs` - Bracket generation and match updates
- Supports consolante (consolation) brackets

### PDF Export
- `src/features/export/pdfPage.tsx` - Paper sizes and the logo box, shared by all three documents
- `src/features/export/CourtAssignmentsPDF.tsx` - Court sheet for each round
- `src/features/export/StandingsPDF.tsx` - Standings table PDF
- `src/features/export/BracketPDF.tsx` - Bracket visualization PDF
- `src/features/export/ExportView.tsx` - Export UI and file saving
- Uses Tauri dialog plugin for save dialogs

**Paper size is a property of the print run, not of the tournament.** Each card on the
export page carries its own size picker, and passes the choice to the document as a
`paperSize` prop. A 32-team bracket wants 13x19 on the same afternoon the standings want
Letter, and the same standings go on Letter for the noticeboard and A3 for the wall — one
stored setting could serve none of that. `ExportView` holds the three choices in state
(keyed by document) rather than in the database, so they last as long as the operator is on
the tab and start again at Letter next time. Note `PdfCard` is redefined on every render of
`ExportView`, so it cannot hold that state itself.

- `pageProps(paperSize, orientation)` supplies `<Page size>`; the six sizes are US Letter
  (the default), US Legal, US Tabloid, 13x19, A4 and A3. Never hard-code `size="A4"` again.
- `PAPER_SIZES` stores **portrait dimensions, not @react-pdf's size names**. The names are
  only a lookup into the same numbers, 13x19 has no name at all, and holding both a name and
  a width/height invites the two to disagree. `orientation="landscape"` flips whatever it is
  given, so portrait is the only form worth storing.
- `BracketPDF` lays itself out by hand and so also needs `contentSize(...)`: its column width
  and row spacing are derived from the chosen page, which is what lets a deep bracket
  genuinely benefit from bigger paper instead of being squeezed to the minimum.
- **The logo stays a tournament setting** — it identifies the event, not the print run — so
  it is still on the tournament form and still read off the `tournament` prop.
- `<PdfLogo tournament />` draws `tournaments.logo` (a data URI) absolutely in the top-right,
  fitted inside a bounding box; each header reserves that much padding on its right. The box
  is **absolutely positioned, so it pushes nothing out of its way** — its height is sized to
  the least room any variant of that header has, which is measured, not guessed. The three
  tight spots: the court sheet's "nothing to show" page has no game title, so its header
  carries a `minHeight` to stop the rule riding up through the logo; and a 32-team bracket
  stretches its columns to the page edge, putting the "Final" round label directly under the
  logo, which is why `BracketPDF`'s header margin and `VERTICAL_RESERVE` move together. The
  upload accepts **PNG and JPEG only** — those are the formats `@react-pdf`'s `Image` can
  draw, so an SVG would silently come out blank — and caps at 2 MB, because the data URI
  travels in the database and in every backup file.
- The court sheet numbers its pages **"Game 1"**, not "Round 1" — a round is what a bracket
  has, and `pdf.round` is still what `BracketPDF` labels its columns with. They are separate
  keys (`pdf.game` / `pdf.round`) for that reason.
- With **All Teams Seeded into Brackets** on, a consolante prints as "Concours AA" rather than
  "Consolante AA": each bracket is a concours in its own right. Internally it is still
  `is_consolante`, and the on-screen labels are unchanged.

## Database Schema

Located in `src-tauri/src/db/schema.rs`:

**Core Tables:**
- `tournaments` - Tournament configuration (incl. `logo`, printed on every document). A
  `paper_size` column here was retired when paper became a per-print choice; databases that
  already have it keep it, unread, and every INSERT relies on its `DEFAULT 'letter'`
- `teams` - Registered teams (`is_withdrawn` flags one that pulled out mid-tournament)
- `qualifying_rounds` - Round metadata
- `qualifying_games` - Individual game results
- `team_standings` - Computed standings (denormalized)
- `brackets` - Elimination bracket metadata
- `bracket_matches` - Elimination match results (`court_is_manual` protects a hand-set court
  from the automatic renumbering)
- `pairing_history` - Tracks previous matchups
- `court_history` - Court assignment tracking
- `panache_teams` / `panache_team_members` - Panaché temporary teams, per round
- `panache_sitouts` - Panaché players resting a round

**Key Standings Fields:**
- wins, losses, points_for, points_against
- differential (computed: points_for - points_against)
- buchholz_score (sum of opponent wins) - used by Swiss
- fine_buchholz_score (sum of opponent buchholz scores) - used by Swiss
- point_quotient (points_for / points_against) - used by Swiss Hotel, Round Robin, Pool Play
- is_eliminated (boolean) - used by Pool Play for teams with 2 losses
- rank (final computed rank)

## State Management

`src/stores/tournamentStore.ts` - Single Zustand store containing:
- Current tournament data
- Teams list
- Qualifying rounds and games
- Standings
- Brackets and matches
- All fetch/update actions that call Tauri commands

## i18n Structure

Translation files in `src/i18n/locales/`:
- `en.json` - English translations
- `fr.json` - French translations

Key namespaces: common, nav, tournaments, teams, pairing, brackets, export, pdf, validation

## Common Patterns

### Adding a new field to standings:
1. Add column to `team_standings` table in `schema.rs` with migration
2. Add field to `TeamStanding` struct in `models/mod.rs`
3. Update `calculate_buchholz_and_ranks()` in `qualifying.rs`
4. Update `get_standings()` query in `teams.rs`
5. Add field to `TeamStanding` interface in `types/index.ts`
6. Update `StandingsTable.tsx` and `StandingsPDF.tsx`
7. Add translations in `en.json` and `fr.json`

### Adding a Tauri command:
1. Create function in appropriate `commands/*.rs` file with `#[tauri::command]`
2. Register in `lib.rs` invoke_handler
3. Call from frontend using `invoke<ReturnType>('command_name', { args })`

### Adding a column to `tournaments` or `teams`:
1. Add it to the `CREATE TABLE` in `schema.rs` **and** to an `add_column_if_missing` call at
   the end of `create_tables`. Leave the CHECK-constraint rebuild DDL alone — it runs *before*
   those ALTERs and copies an explicit column list out of the old table, so naming a new
   column there would abort the whole rebuild silently.
2. Add the field to the struct in `models/mod.rs`, with **`#[serde(default)]`**:
   `TournamentBackup` embeds `Tournament` and `Team` bare, so without it every backup file
   written before the column existed fails to import.
3. Extend every positional SQL site: `tournaments.rs` / `teams.rs` (select, insert, update)
   and **`backup.rs`** (both the export read and the restore insert).
4. Add it to `types/index.ts`, then to `TournamentForm.tsx` *and* the four hand-written
   mapping sites in `TournamentCreate.tsx` / `TournamentEdit.tsx` — they cast with `as any`,
   so a missed field is dropped silently rather than caught by `tsc`.
5. Add the `(table, column)` pair to `migrations_add_the_panache_columns` in `schema.rs`.

## Tests

`cd src-tauri && cargo test` — the only automated tests in the repo. They cover the panaché
scheduler (sit-out rotation, no repeated teammates, champion separation and exposure), the
panaché database round-trip (sides persist, a shared score lands on each member), the court
solver, round generation against a real database (withdrawal, byes, the configured-round cap,
and both sides of the results-dependency guard — Swiss waits, Swiss Hotel draws ahead),
the idempotency of `complete_round`, bracket court assignment, the backup round-trip, and the
schema migrations.

Court and bracket tests assert the *invariant*, never the exact numbering: which game gets
which court is the draw's business, so a test that pins the old sequential order is testing
the wrong thing.

The migration test matters most: the `tournaments` CHECK-constraint rebuild is invoked with
`.ok()`, so a failure is **silent** — the data survives but the table keeps its old constraint
and the new pairing method is rejected later with no clue why. Any test of that rebuild must
assert the new constraint is present, not just that the data is intact.

## Build Notes

- Requires Node.js 20+ (uses nvm, default is old v0.12)
- Use `source ~/.nvm/nvm.sh && nvm use 20.20.0` before running npm commands
- `npm run tauri dev` for development
- `npm run tauri build` for production build
- GitHub Actions workflow in `.github/workflows/release.yml` builds on tag push

### Printing

`commands/printing.rs` raises the system print path, and every platform reaches it
differently — the module doc has the detail. The shape to keep in mind:

| Platform | Route | What the operator gets |
|---|---|---|
| macOS | PDFKit `NSPrintOperation` | The standard print panel: printer, range, copies |
| iPadOS | `UIPrintInteractionController` | The AirPrint sheet (anchored, or it throws on iPad) |
| Windows | `ShellExecuteW`, `print` verb | Whatever owns PDFs — Edge shows its preview; some handlers go straight to the default printer |
| Linux | CUPS `lp` | Silent, to the default printer |

Windows has **no CUPS**, so `lpstat`/`lp` are not a fallback there — they are simply
missing, and the command fails before it does anything. That is why the CUPS branch is
gated `not(target_os = "windows")` as well as `not(target_os = "macos")`: anything reading
"every desktop but macOS" as "Linux" will break Windows silently, because the code still
compiles.

`safe_file_name` guards the spooled name. A tournament title is free text and goes straight
into a file name, and Windows rejects `\ / : * ? " < > |` and strips trailing dots and
spaces — so `Doubles 2026: Spring/Fall` is an ordinary title that cannot be written to disk.
It is compiled on macOS too, where nothing uses it, because that is the only platform the
tests run on.

### Demo mode

`src/features/demo/` fills invented scores in, for walking an audience through a whole
tournament without typing several hundred results. It is **armed, not always on**:
`Cmd/Ctrl+Shift+D` toggles it and is the only way in, nothing persists the flag (`demoStore`
is in-memory, so every launch starts disarmed), and while it is on a loud amber badge sits in
the corner naming the shortcut and reporting what the last fill did. That is deliberate — this
app runs real tournaments, and a three-key press must not be able to find demo mode already
switched on, or leave invented scores behind without saying so.

`Cmd/Ctrl+Shift+R` then fills, and **only ever fills a blank**: a score already saved, or typed
and not yet blurred, is left alone. That is what makes the feature safe rather than merely
unlikely to misfire — it cannot overwrite an operator's work, so the worst a stray press can
do is add results that were not there.

- **Qualifying** (`RoundGames.tsx`) fills every unscored game in the round, not just the ones
  the search box is showing: a round cannot be completed until all of its games have a score,
  so filling the filtered subset would leave the demo stuck. It then needs **Complete Round**
  as usual — the fill does not score the round for you.
- **Brackets** (`BracketDisplay.tsx`) fills one wave per press. `canEditMatch` requires both
  teams, so the next round only becomes fillable once these results have advanced into it;
  pressing again walks down the bracket the way the real thing progresses. Updates are strictly
  sequential because every result re-runs `assign_bracket_courts` for the wave.
- Only one of the two is ever mounted (Radix unmounts inactive tabs, and the rounds view holds
  a single `selectedRoundId`), so one press reaches one surface.

`scoreSampler.ts` is where the scores come from, and the weights are **measured, not invented**:
they are the observed frequencies of the 435 games of `tests/fixtures/aio_2024.json`, the same
workbook the ranking test replays. 13-x covers 87.8% of that field; the other 12% ran out of
time with the winner on 7-12, so a generator that only ever produces 13-x both looks wrong to
anyone who has run an event and never exercises the ranking code's handling of a low-scoring
win. The loser's score is drawn from the same distribution conditioned on staying under the
winner's. The module is pure and has no test — there is no JS test runner in this repo.

## Known Issues / Warnings

- Large JS bundle (~2MB) could benefit from code splitting

## Tournament Form Defaults

- Tournament Type: Club
- Team Composition: Select
- Format: Double
- Pairing Method: Swiss
- All Teams Advance: Unchecked
