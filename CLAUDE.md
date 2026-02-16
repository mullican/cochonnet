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

**Key Functions in `qualifying.rs`:**
- `generate_swiss_pairings()` - Pairs teams by similar win records
- `generate_swiss_hotel_pairings()` - Random pairing with constraints
- `generate_pool_play_round()` - Round-specific Pool Play logic
- `calculate_buchholz_and_ranks()` - Swiss tiebreaker calculation
- `calculate_point_quotient_ranks()` - Point quotient tiebreaker calculation
- `complete_round()` - Score processing and rank updates

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

## Build Notes

- Requires Node.js 20+ (uses nvm, default is old v0.12)
- Use `source ~/.nvm/nvm.sh && nvm use 20.20.0` before running npm commands
- `npm run tauri dev` for development
- `npm run tauri build` for production build
- GitHub Actions workflow in `.github/workflows/release.yml` builds on tag push

## Known Issues / Warnings

- Some unused variables in `qualifying.rs` related to court history in `assign_courts()`
- Some unused structs in `models/mod.rs` (StandingWithTeam, PairingHistory, CourtHistory)
- Large JS bundle (~2MB) could benefit from code splitting

## Tournament Form Defaults

- Tournament Type: Club
- Team Composition: Select
- Format: Double
- Pairing Method: Swiss
- All Teams Advance: Unchecked
