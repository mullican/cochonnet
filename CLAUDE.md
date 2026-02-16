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

### Swiss System Qualifying
- `src/features/pairing/QualifyingRounds.tsx` - Main qualifying rounds view
- `src/features/pairing/StandingsTable.tsx` - Standings display table
- `src-tauri/src/commands/qualifying.rs` - Core pairing and scoring logic
  - `generate_swiss_pairings()` - Swiss pairing algorithm (lines ~372-490)
  - `calculate_buchholz_and_ranks()` - Standings calculation (lines ~751-833)
  - `complete_round()` - Score processing (lines ~587-690)

### Tiebreaker Algorithm
Located in `calculate_buchholz_and_ranks()`:
1. **Wins** (descending)
2. **Buchholz Score** - sum of opponents' wins
3. **Fine Buchholz Score** - sum of opponents' Buchholz scores
4. **Point Differential** - total points scored minus points allowed
5. **Random** - random u64 for final tiebreaker

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
- buchholz_score (sum of opponent wins)
- fine_buchholz_score (sum of opponent buchholz scores)
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

- Some unused variables in `qualifying.rs` related to court history (lines 550-555)
- Some unused structs in `models/mod.rs` (StandingWithTeam, PairingHistory, CourtHistory)
- Large JS bundle (~2MB) could benefit from code splitting
