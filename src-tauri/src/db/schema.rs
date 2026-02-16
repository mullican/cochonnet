use rusqlite::{Connection, Result};

pub fn create_tables(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        -- Tournaments table
        CREATE TABLE IF NOT EXISTS tournaments (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            team_composition TEXT NOT NULL CHECK (team_composition IN ('men', 'women', 'mixed', 'select')),
            tournament_type TEXT NOT NULL CHECK (tournament_type IN ('regional', 'national', 'open', 'club')),
            start_date TEXT NOT NULL,
            end_date TEXT NOT NULL,
            director TEXT NOT NULL,
            head_umpire TEXT NOT NULL,
            format TEXT NOT NULL CHECK (format IN ('single', 'double', 'triple')),
            day_type TEXT NOT NULL CHECK (day_type IN ('single', 'two')),
            number_of_courts INTEGER NOT NULL,
            number_of_qualifying_rounds INTEGER NOT NULL DEFAULT 5,
            has_consolante INTEGER NOT NULL DEFAULT 0,
            advance_all INTEGER NOT NULL DEFAULT 1,
            advance_count INTEGER,
            bracket_size INTEGER NOT NULL DEFAULT 16,
            pairing_method TEXT NOT NULL CHECK (pairing_method IN ('swiss', 'swissHotel', 'roundRobin', 'poolPlay')),
            region_avoidance INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- Additional umpires (one-to-many with tournaments)
        CREATE TABLE IF NOT EXISTS umpires (
            id TEXT PRIMARY KEY,
            tournament_id TEXT NOT NULL,
            name TEXT NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE
        );

        -- Teams table
        CREATE TABLE IF NOT EXISTS teams (
            id TEXT PRIMARY KEY,
            tournament_id TEXT NOT NULL,
            captain TEXT NOT NULL,
            player2 TEXT NOT NULL,
            player3 TEXT,
            region TEXT,
            club TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE
        );

        -- Qualifying rounds
        CREATE TABLE IF NOT EXISTS qualifying_rounds (
            id TEXT PRIMARY KEY,
            tournament_id TEXT NOT NULL,
            round_number INTEGER NOT NULL,
            is_complete INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE
        );

        -- Qualifying games (matches in qualifying rounds)
        CREATE TABLE IF NOT EXISTS qualifying_games (
            id TEXT PRIMARY KEY,
            round_id TEXT NOT NULL,
            court_number INTEGER NOT NULL,
            team1_id TEXT,
            team2_id TEXT,
            team1_score INTEGER,
            team2_score INTEGER,
            is_bye INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (round_id) REFERENCES qualifying_rounds(id) ON DELETE CASCADE,
            FOREIGN KEY (team1_id) REFERENCES teams(id) ON DELETE SET NULL,
            FOREIGN KEY (team2_id) REFERENCES teams(id) ON DELETE SET NULL
        );

        -- Team standings (denormalized for performance)
        CREATE TABLE IF NOT EXISTS team_standings (
            id TEXT PRIMARY KEY,
            tournament_id TEXT NOT NULL,
            team_id TEXT NOT NULL,
            wins INTEGER NOT NULL DEFAULT 0,
            losses INTEGER NOT NULL DEFAULT 0,
            points_for INTEGER NOT NULL DEFAULT 0,
            points_against INTEGER NOT NULL DEFAULT 0,
            differential INTEGER NOT NULL DEFAULT 0,
            buchholz_score REAL NOT NULL DEFAULT 0,
            fine_buchholz_score REAL NOT NULL DEFAULT 0,
            point_quotient REAL NOT NULL DEFAULT 0,
            is_eliminated INTEGER NOT NULL DEFAULT 0,
            rank INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE,
            FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
            UNIQUE(tournament_id, team_id)
        );

        -- Brackets table
        CREATE TABLE IF NOT EXISTS brackets (
            id TEXT PRIMARY KEY,
            tournament_id TEXT NOT NULL,
            name TEXT NOT NULL,
            is_consolante INTEGER NOT NULL DEFAULT 0,
            size INTEGER NOT NULL,
            is_complete INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE
        );

        -- Bracket matches
        CREATE TABLE IF NOT EXISTS bracket_matches (
            id TEXT PRIMARY KEY,
            bracket_id TEXT NOT NULL,
            round_number INTEGER NOT NULL,
            match_number INTEGER NOT NULL,
            court_number INTEGER,
            team1_id TEXT,
            team2_id TEXT,
            team1_score INTEGER,
            team2_score INTEGER,
            winner_id TEXT,
            next_match_id TEXT,
            is_bye INTEGER NOT NULL DEFAULT 0,
            FOREIGN KEY (bracket_id) REFERENCES brackets(id) ON DELETE CASCADE,
            FOREIGN KEY (team1_id) REFERENCES teams(id) ON DELETE SET NULL,
            FOREIGN KEY (team2_id) REFERENCES teams(id) ON DELETE SET NULL,
            FOREIGN KEY (winner_id) REFERENCES teams(id) ON DELETE SET NULL,
            FOREIGN KEY (next_match_id) REFERENCES bracket_matches(id) ON DELETE SET NULL
        );

        -- Pairing history (track who played whom)
        CREATE TABLE IF NOT EXISTS pairing_history (
            id TEXT PRIMARY KEY,
            tournament_id TEXT NOT NULL,
            team1_id TEXT NOT NULL,
            team2_id TEXT NOT NULL,
            round_id TEXT NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE,
            FOREIGN KEY (team1_id) REFERENCES teams(id) ON DELETE CASCADE,
            FOREIGN KEY (team2_id) REFERENCES teams(id) ON DELETE CASCADE,
            FOREIGN KEY (round_id) REFERENCES qualifying_rounds(id) ON DELETE CASCADE
        );

        -- Court history (track court assignments for rotation)
        CREATE TABLE IF NOT EXISTS court_history (
            id TEXT PRIMARY KEY,
            tournament_id TEXT NOT NULL,
            team_id TEXT NOT NULL,
            court_number INTEGER NOT NULL,
            round_id TEXT NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE,
            FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
            FOREIGN KEY (round_id) REFERENCES qualifying_rounds(id) ON DELETE CASCADE
        );

        -- Create indexes for better query performance
        CREATE INDEX IF NOT EXISTS idx_teams_tournament ON teams(tournament_id);
        CREATE INDEX IF NOT EXISTS idx_qualifying_rounds_tournament ON qualifying_rounds(tournament_id);
        CREATE INDEX IF NOT EXISTS idx_qualifying_games_round ON qualifying_games(round_id);
        CREATE INDEX IF NOT EXISTS idx_team_standings_tournament ON team_standings(tournament_id);
        CREATE INDEX IF NOT EXISTS idx_brackets_tournament ON brackets(tournament_id);
        CREATE INDEX IF NOT EXISTS idx_bracket_matches_bracket ON bracket_matches(bracket_id);
        CREATE INDEX IF NOT EXISTS idx_pairing_history_tournament ON pairing_history(tournament_id);
        CREATE INDEX IF NOT EXISTS idx_court_history_tournament ON court_history(tournament_id);
        "#,
    )?;

    // Migration: Add number_of_qualifying_rounds column if it doesn't exist
    let has_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('tournaments') WHERE name='number_of_qualifying_rounds'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_column {
        conn.execute(
            "ALTER TABLE tournaments ADD COLUMN number_of_qualifying_rounds INTEGER NOT NULL DEFAULT 5",
            [],
        ).ok();
    }

    // Migration: Add court_number column to bracket_matches if it doesn't exist
    let has_court_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('bracket_matches') WHERE name='court_number'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_court_column {
        conn.execute(
            "ALTER TABLE bracket_matches ADD COLUMN court_number INTEGER",
            [],
        ).ok();
    }

    // Migration: Add fine_buchholz_score column to team_standings if it doesn't exist
    let has_fine_buchholz_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('team_standings') WHERE name='fine_buchholz_score'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_fine_buchholz_column {
        conn.execute(
            "ALTER TABLE team_standings ADD COLUMN fine_buchholz_score REAL NOT NULL DEFAULT 0",
            [],
        ).ok();
    }

    // Migration: Add point_quotient column to team_standings if it doesn't exist
    let has_point_quotient_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('team_standings') WHERE name='point_quotient'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_point_quotient_column {
        conn.execute(
            "ALTER TABLE team_standings ADD COLUMN point_quotient REAL NOT NULL DEFAULT 0",
            [],
        ).ok();
    }

    // Migration: Add is_eliminated column to team_standings if it doesn't exist
    let has_is_eliminated_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('team_standings') WHERE name='is_eliminated'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_is_eliminated_column {
        conn.execute(
            "ALTER TABLE team_standings ADD COLUMN is_eliminated INTEGER NOT NULL DEFAULT 0",
            [],
        ).ok();
    }

    // Migration: Update pairing_method CHECK constraint to include new formats
    // SQLite doesn't support ALTER TABLE to modify constraints, so we need to recreate the table
    // Check if the old constraint exists by looking at the table schema
    let table_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type='table' AND name='tournaments'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();

    // If the table exists and doesn't include 'swissHotel' in the constraint, migrate it
    if !table_sql.is_empty() && !table_sql.contains("swissHotel") {
        conn.execute_batch(
            r#"
            -- Create new table with updated constraint
            CREATE TABLE tournaments_new (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                team_composition TEXT NOT NULL CHECK (team_composition IN ('men', 'women', 'mixed', 'select')),
                tournament_type TEXT NOT NULL CHECK (tournament_type IN ('regional', 'national', 'open', 'club')),
                start_date TEXT NOT NULL,
                end_date TEXT NOT NULL,
                director TEXT NOT NULL,
                head_umpire TEXT NOT NULL,
                format TEXT NOT NULL CHECK (format IN ('single', 'double', 'triple')),
                day_type TEXT NOT NULL CHECK (day_type IN ('single', 'two')),
                number_of_courts INTEGER NOT NULL,
                number_of_qualifying_rounds INTEGER NOT NULL DEFAULT 5,
                has_consolante INTEGER NOT NULL DEFAULT 0,
                advance_all INTEGER NOT NULL DEFAULT 1,
                advance_count INTEGER,
                bracket_size INTEGER NOT NULL DEFAULT 16,
                pairing_method TEXT NOT NULL CHECK (pairing_method IN ('swiss', 'swissHotel', 'roundRobin', 'poolPlay')),
                region_avoidance INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Copy data from old table
            INSERT INTO tournaments_new SELECT * FROM tournaments;

            -- Drop old table
            DROP TABLE tournaments;

            -- Rename new table
            ALTER TABLE tournaments_new RENAME TO tournaments;
            "#,
        ).ok();
    }

    Ok(())
}
