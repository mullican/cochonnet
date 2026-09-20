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
            pairing_method TEXT NOT NULL CHECK (pairing_method IN ('swiss', 'swissHotel', 'roundRobin', 'poolPlay', 'panache')),
            region_avoidance INTEGER NOT NULL DEFAULT 0,
            paper_size TEXT NOT NULL DEFAULT 'letter',
            logo TEXT,
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
            team_number INTEGER NOT NULL DEFAULT 0,
            captain TEXT NOT NULL,
            player2 TEXT NOT NULL,
            player3 TEXT,
            region TEXT,
            club TEXT,
            is_withdrawn INTEGER NOT NULL DEFAULT 0,
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
            court_is_manual INTEGER NOT NULL DEFAULT 0,
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

        -- Panache: temporary teams drawn fresh each round.
        -- Individuals are stored as `teams` rows (captain only); these tables
        -- hold the throwaway doubles/triples they are shuffled into.
        CREATE TABLE IF NOT EXISTS panache_teams (
            id TEXT PRIMARY KEY,
            tournament_id TEXT NOT NULL,
            round_id TEXT NOT NULL,
            team_index INTEGER NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE,
            FOREIGN KEY (round_id) REFERENCES qualifying_rounds(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS panache_team_members (
            id TEXT PRIMARY KEY,
            panache_team_id TEXT NOT NULL,
            team_id TEXT NOT NULL,
            position INTEGER NOT NULL,
            FOREIGN KEY (panache_team_id) REFERENCES panache_teams(id) ON DELETE CASCADE,
            FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
        );

        -- Players sitting out a round because the roster does not divide evenly.
        -- A sit-out is not a bye: it leaves the player's record untouched.
        CREATE TABLE IF NOT EXISTS panache_sitouts (
            id TEXT PRIMARY KEY,
            tournament_id TEXT NOT NULL,
            round_id TEXT NOT NULL,
            team_id TEXT NOT NULL,
            FOREIGN KEY (tournament_id) REFERENCES tournaments(id) ON DELETE CASCADE,
            FOREIGN KEY (round_id) REFERENCES qualifying_rounds(id) ON DELETE CASCADE,
            FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE
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
        CREATE INDEX IF NOT EXISTS idx_panache_teams_round ON panache_teams(round_id);
        CREATE INDEX IF NOT EXISTS idx_panache_team_members_team ON panache_team_members(panache_team_id);
        CREATE INDEX IF NOT EXISTS idx_panache_sitouts_round ON panache_sitouts(round_id);
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

    // If the table exists and doesn't include 'panache' in the constraint, migrate it.
    // The gate names the newest value, so a single rebuild upgrades every prior schema
    // version (pre-swissHotel databases included).
    if !table_sql.is_empty() && !table_sql.contains("panache") {
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
                pairing_method TEXT NOT NULL CHECK (pairing_method IN ('swiss', 'swissHotel', 'roundRobin', 'poolPlay', 'panache')),
                region_avoidance INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            -- Copy data from old table. Columns are listed explicitly: a database old
            -- enough to have received number_of_qualifying_rounds via ALTER has it
            -- appended last, so a positional SELECT * would shift every later column.
            INSERT INTO tournaments_new (
                id, name, team_composition, tournament_type, start_date, end_date,
                director, head_umpire, format, day_type, number_of_courts,
                number_of_qualifying_rounds, has_consolante, advance_all, advance_count,
                bracket_size, pairing_method, region_avoidance, created_at, updated_at
            )
            SELECT
                id, name, team_composition, tournament_type, start_date, end_date,
                director, head_umpire, format, day_type, number_of_courts,
                number_of_qualifying_rounds, has_consolante, advance_all, advance_count,
                bracket_size, pairing_method, region_avoidance, created_at, updated_at
            FROM tournaments;

            -- Drop old table
            DROP TABLE tournaments;

            -- Rename new table
            ALTER TABLE tournaments_new RENAME TO tournaments;
            "#,
        ).ok();
    }

    // Migration: Add team_number column to teams if it doesn't exist
    let has_team_number_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info('teams') WHERE name='team_number'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_team_number_column {
        conn.execute(
            "ALTER TABLE teams ADD COLUMN team_number INTEGER NOT NULL DEFAULT 0",
            [],
        ).ok();

        // Backfill existing teams with sequential numbers, per tournament,
        // ordered by creation order so pre-existing rosters get stable numbers.
        let mut tournament_stmt = conn.prepare("SELECT DISTINCT tournament_id FROM teams")?;
        let tournament_ids: Vec<String> = tournament_stmt
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(tournament_stmt);

        for tournament_id in tournament_ids {
            let mut team_stmt = conn.prepare(
                "SELECT id FROM teams WHERE tournament_id = ?1 ORDER BY created_at ASC",
            )?;
            let team_ids: Vec<String> = team_stmt
                .query_map(rusqlite::params![tournament_id], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
            drop(team_stmt);

            for (idx, team_id) in team_ids.iter().enumerate() {
                conn.execute(
                    "UPDATE teams SET team_number = ?1 WHERE id = ?2",
                    rusqlite::params![(idx as i32) + 1, team_id],
                )?;
            }
        }
    }

    // Migration: Add is_champion column to teams if it doesn't exist.
    // Panache lets an operator flag expert players; the flag is what switches the
    // champion constraints on, so no tournament-level toggle is needed.
    add_column_if_missing(
        conn,
        "teams",
        "is_champion",
        "ALTER TABLE teams ADD COLUMN is_champion INTEGER NOT NULL DEFAULT 0",
    );

    // Migration: Add is_final column to qualifying_rounds if it doesn't exist.
    // Panache ends with a single final game rather than a bracket; it is stored as
    // one extra round so score entry and the court-assignment export are reused.
    add_column_if_missing(
        conn,
        "qualifying_rounds",
        "is_final",
        "ALTER TABLE qualifying_rounds ADD COLUMN is_final INTEGER NOT NULL DEFAULT 0",
    );

    // Migration: Add panache side columns to qualifying_games if they don't exist.
    // For panache games team1_id/team2_id are NULL and these point at panache_teams
    // instead. No REFERENCES clause: cleanup already happens via the round_id cascade.
    add_column_if_missing(
        conn,
        "qualifying_games",
        "side1_id",
        "ALTER TABLE qualifying_games ADD COLUMN side1_id TEXT",
    );
    add_column_if_missing(
        conn,
        "qualifying_games",
        "side2_id",
        "ALTER TABLE qualifying_games ADD COLUMN side2_id TEXT",
    );

    // Migration: Add the printing settings to tournaments.
    //
    // These must stay below the CHECK-constraint rebuild above: that rebuild
    // copies an explicit column list out of the old table, so naming a column
    // there that an older database has not got yet would abort the whole batch
    // - silently, because it runs under `.ok()`. Rebuilding first and adding
    // the columns afterwards upgrades every schema version the same way.
    add_column_if_missing(
        conn,
        "tournaments",
        "paper_size",
        "ALTER TABLE tournaments ADD COLUMN paper_size TEXT NOT NULL DEFAULT 'letter'",
    );
    // A data URI rather than a path: the logo has to survive a backup export and
    // land intact on whatever machine restores it.
    add_column_if_missing(
        conn,
        "tournaments",
        "logo",
        "ALTER TABLE tournaments ADD COLUMN logo TEXT",
    );

    // Migration: Add is_withdrawn to teams. A team that pulls out mid-tournament
    // cannot be deleted - its played games and its opponents' Buchholz depend on
    // it - so it is flagged instead and skipped by later draws.
    add_column_if_missing(
        conn,
        "teams",
        "is_withdrawn",
        "ALTER TABLE teams ADD COLUMN is_withdrawn INTEGER NOT NULL DEFAULT 0",
    );

    // Migration: Add court_is_manual to bracket_matches. Bracket courts are
    // renumbered tournament-wide whenever a bracket is added or a match is
    // scored; this flag is what keeps an operator's hand-edited court from being
    // overwritten by the next run.
    add_column_if_missing(
        conn,
        "bracket_matches",
        "court_is_manual",
        "ALTER TABLE bracket_matches ADD COLUMN court_is_manual INTEGER NOT NULL DEFAULT 0",
    );

    Ok(())
}

/// Adds a column when the table doesn't already have it.
///
/// The older migrations above inline this same pragma_table_info check; new ones
/// share this helper rather than repeating it.
fn add_column_if_missing(conn: &Connection, table: &str, column: &str, alter_sql: &str) {
    let has_column: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM pragma_table_info(?1) WHERE name = ?2",
            rusqlite::params![table, column],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !has_column {
        conn.execute(alter_sql, []).ok();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The oldest tournaments table, before number_of_qualifying_rounds existed and
    /// before swissHotel/poolPlay/panache were allowed pairing methods.
    const LEGACY_TOURNAMENTS_DDL: &str = r#"
        CREATE TABLE tournaments (
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
            has_consolante INTEGER NOT NULL DEFAULT 0,
            advance_all INTEGER NOT NULL DEFAULT 1,
            advance_count INTEGER,
            bracket_size INTEGER NOT NULL DEFAULT 16,
            pairing_method TEXT NOT NULL CHECK (pairing_method IN ('swiss', 'roundRobin')),
            region_avoidance INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
    "#;

    /// Reproduces the worst-case upgrade: a database old enough that
    /// number_of_qualifying_rounds was appended by ALTER, so its column order does
    /// not match the rebuild DDL.
    ///
    /// A positional `INSERT ... SELECT *` shifts every column after it, which lands
    /// region_avoidance in pairing_method and trips that column's CHECK. The whole
    /// rebuild batch then aborts, and because it is invoked with `.ok()` the failure
    /// is silent: the tournament keeps its data but the table keeps its OLD
    /// constraint, so creating a panache tournament fails later with no clue why.
    /// Asserting the new constraint is present is therefore what gives this test
    /// teeth; the data assertions alone pass even when the migration no-ops.
    #[test]
    fn legacy_database_survives_the_pairing_method_rebuild() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(LEGACY_TOURNAMENTS_DDL).unwrap();
        conn.execute(
            "ALTER TABLE tournaments ADD COLUMN number_of_qualifying_rounds INTEGER NOT NULL DEFAULT 5",
            [],
        )
        .unwrap();

        conn.execute(
            r#"
            INSERT INTO tournaments (
                id, name, team_composition, tournament_type, start_date, end_date,
                director, head_umpire, format, day_type, number_of_courts,
                has_consolante, advance_all, advance_count, bracket_size,
                pairing_method, region_avoidance, created_at, updated_at,
                number_of_qualifying_rounds
            ) VALUES (
                'tid', 'Old Open', 'mixed', 'club', '2026-01-01', '2026-01-02',
                'Director', 'Umpire', 'double', 'single', 7,
                1, 0, 16, 32,
                'swiss', 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z',
                9
            )
            "#,
            [],
        )
        .unwrap();

        create_tables(&conn).unwrap();

        let (name, courts, rounds, bracket_size, consolante, advance_all, advance_count, region):
            (String, i32, i32, i32, i32, i32, i32, i32) = conn
            .query_row(
                r#"
                SELECT name, number_of_courts, number_of_qualifying_rounds, bracket_size,
                       has_consolante, advance_all, advance_count, region_avoidance
                FROM tournaments WHERE id = 'tid'
                "#,
                [],
                |row| {
                    Ok((
                        row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?,
                        row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?,
                    ))
                },
            )
            .unwrap();

        assert_eq!(name, "Old Open");
        assert_eq!(courts, 7);
        assert_eq!(rounds, 9);
        assert_eq!(bracket_size, 32);
        assert_eq!(consolante, 1);
        assert_eq!(advance_all, 0);
        assert_eq!(advance_count, 16);
        assert_eq!(region, 1);

        // The rebuild must have actually completed, not silently aborted.
        let table_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='tournaments'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(
            table_sql.contains("panache"),
            "rebuild did not run; tournaments still has the old CHECK constraint"
        );

        // And nothing may be left half-built.
        let leftover: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = 'tournaments_new'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(leftover, 0, "tournaments_new was left behind");
    }

    #[test]
    fn migrated_database_accepts_panache() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(LEGACY_TOURNAMENTS_DDL).unwrap();
        create_tables(&conn).unwrap();

        conn.execute(
            r#"
            INSERT INTO tournaments (
                id, name, team_composition, tournament_type, start_date, end_date,
                director, head_umpire, format, day_type, number_of_courts,
                number_of_qualifying_rounds, has_consolante, advance_all, advance_count,
                bracket_size, pairing_method, region_avoidance, created_at, updated_at
            ) VALUES (
                'p', 'Melee', 'mixed', 'club', '2026-01-01', '2026-01-02',
                'D', 'U', 'triple', 'single', 4, 5, 0, 1, NULL, 16,
                'panache', 0, 'now', 'now'
            )
            "#,
            [],
        )
        .unwrap();

        let method: String = conn
            .query_row(
                "SELECT pairing_method FROM tournaments WHERE id = 'p'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(method, "panache");
    }

    #[test]
    fn migrations_add_the_panache_columns() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();

        for (table, column) in [
            ("teams", "is_champion"),
            ("qualifying_rounds", "is_final"),
            ("qualifying_games", "side1_id"),
            ("qualifying_games", "side2_id"),
            ("tournaments", "paper_size"),
            ("tournaments", "logo"),
            ("teams", "is_withdrawn"),
            ("bracket_matches", "court_is_manual"),
        ] {
            let present: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM pragma_table_info(?1) WHERE name = ?2",
                    rusqlite::params![table, column],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(present, "{}.{} missing", table, column);
        }
    }

    #[test]
    fn create_tables_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        create_tables(&conn).unwrap();
        create_tables(&conn).unwrap();
    }
}
