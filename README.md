# Cochonnet - Petanque Tournament Manager

A desktop application for managing petanque tournaments, built with Tauri 2, React, and TypeScript.

## Features

- **Tournament Management**: Create and configure tournaments with various formats (singles, doubles, triples)
- **Team Registration**: Import teams from CSV or add them manually
- **Swiss System Qualifying Rounds**: Automatic pairing generation with configurable number of rounds
- **Elimination Brackets**: Generate single or double elimination brackets
- **Consolante Support**: Optional consolation bracket for eliminated teams
- **Region Avoidance**: Option to avoid same-region matchups in qualifying
- **Multi-language Support**: English and French translations
- **PDF Export**: Generate score sheets, standings, and bracket PDFs for printing

## Swiss System Tiebreaker Order

When teams are tied on wins in the Swiss system qualifying rounds, ties are broken in this order:

1. **Wins** - Total number of games won
2. **Buchholz Score** - Sum of all opponents' wins (strength of schedule)
3. **Fine Buchholz Score** - Sum of all opponents' Buchholz scores (second-order strength of schedule)
4. **Point Differential** - Points scored minus points allowed across all games
5. **Random Draw** - If still tied, a random tiebreaker is applied

## Development

### Prerequisites

- Node.js 20+
- Rust (latest stable)
- Tauri CLI v2

### Setup

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
```

### Project Structure

```
petanque-tournament/
├── src/                      # React frontend
│   ├── components/           # Reusable UI components
│   ├── features/             # Feature-specific components
│   │   ├── brackets/         # Elimination bracket views
│   │   ├── export/           # PDF generation
│   │   ├── pairing/          # Qualifying rounds & standings
│   │   ├── teams/            # Team management
│   │   └── tournaments/      # Tournament CRUD
│   ├── i18n/                 # Internationalization
│   ├── stores/               # Zustand state management
│   └── types/                # TypeScript type definitions
├── src-tauri/                # Rust backend
│   ├── src/
│   │   ├── commands/         # Tauri command handlers
│   │   ├── db/               # SQLite database schema
│   │   └── models/           # Data models
│   └── Cargo.toml
└── package.json
```

## Building Releases

Releases are built automatically via GitHub Actions when you push a version tag:

```bash
git tag v1.0.0
git push origin v1.0.0
```

This creates draft releases with installers for:
- macOS (Intel and Apple Silicon)
- Windows (64-bit)

## License

Private - All rights reserved
