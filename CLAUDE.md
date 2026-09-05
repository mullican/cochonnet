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
| **Swiss** | Round-by-round | Buchholz | Teams with similar records play each other. Prior round must complete before generating next. |
| **Swiss Hotel** | All at once | Point Quotient | Random pairings pre-generated upfront with graduated constraints. |
| **Round Robin** | All at once | Point Quotient | Berger circle method - each team plays every other team. |
| **Pool Play** | Round-by-round | Point Quotient | Fixed 3 rounds: R1 random, R2 winners vs winners, R3 only 1-1 teams play. Teams with 2 losses eliminated. |
| **Panaché** | All at once, redrawable | Wins → Differential | Individual registration. Players are shuffled into fresh temporary teams each round. No bracket — one final game. |

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
  are costs, not filters, because MELEE.md phrases each as "unless unavoidable". Weights, in
  order: champions sharing a team (1000) > repeated teammates (100) > a non-champion who never
  meets a champion (50) > repeated opponents (10) > sit-out imbalance (5).

**Key Functions in `qualifying.rs`:**
- `generate_swiss_pairings()` - Pairs teams by similar win records
- `generate_swiss_hotel_pairings()` - Random pairing with constraints
- `generate_pool_play_round()` - Round-specific Pool Play logic
- `calculate_buchholz_and_ranks()` - Swiss tiebreaker calculation
- `calculate_point_quotient_ranks()` - Point quotient tiebreaker calculation
- `complete_round()` - Score processing and rank updates
- `apply_game_result()` - Adds one result to each competitor's standing (a team, or every member of a panaché temporary team)

**Key Functions in `panache.rs`:**
- `solve_panache_schedule()` - Draws the whole schedule; pure, unit-tested, no DB access
- `generate_panache_rounds()` / `redraw_panache_rounds()` / `generate_panache_final()` - Commands

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
2. Point Differential
3. Point Quotient - points_for / points_against
4. Random tiebreaker

### Elimination Brackets
- `src/features/brackets/BracketView.tsx` - Main bracket display
- `src-tauri/src/commands/brackets.rs` - Bracket generation and match updates
- Supports consolante (consolation) brackets

### PDF Export
- `src/features/export/ScoreSheetPDF.tsx` - Score cards for each round
- `src/features/export/StandingsPDF.tsx` - Standings table PDF
- `src/features/export/BracketPDF.tsx` - Bracket visualization PDF
- `src/features/export/ExportView.tsx` - Export UI and file saving
- Uses Tauri dialog plugin for save dialogs

## Database Schema

Located in `src-tauri/src/db/schema.rs`:

**Core Tables:**
- `tournaments` - Tournament configuration
- `teams` - Registered teams
- `qualifying_rounds` - Round metadata
- `qualifying_games` - Individual game results
- `team_standings` - Computed standings (denormalized)
- `brackets` - Elimination bracket metadata
- `bracket_matches` - Elimination match results
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

## Tests

`cd src-tauri && cargo test` — the only automated tests in the repo. They cover the panaché
scheduler (sit-out rotation, no repeated teammates, champion separation and exposure), the
panaché database round-trip (sides persist, a shared score lands on each member), and the
schema migrations.

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

## Known Issues / Warnings

- Large JS bundle (~2MB) could benefit from code splitting

## Tournament Form Defaults

- Tournament Type: Club
- Team Composition: Select
- Format: Double
- Pairing Method: Swiss
- All Teams Advance: Unchecked
