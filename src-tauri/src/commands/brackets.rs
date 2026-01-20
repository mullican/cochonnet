use crate::db::Database;
use crate::models::{Bracket, BracketMatch, MatchWithTeams, Team};
use crate::commands::teams::get_team_by_id;
use chrono::Utc;
use rand::seq::SliceRandom;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_brackets(db: State<Database>, tournament_id: String) -> Result<Vec<Bracket>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, tournament_id, name, is_consolante, size, is_complete, created_at
            FROM brackets
            WHERE tournament_id = ?1
            ORDER BY name ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let brackets = stmt
        .query_map(params![tournament_id], |row| {
            Ok(Bracket {
                id: row.get(0)?,
                tournament_id: row.get(1)?,
                name: row.get(2)?,
                is_consolante: row.get::<_, i32>(3)? != 0,
                size: row.get(4)?,
                is_complete: row.get::<_, i32>(5)? != 0,
                created_at: row.get(6)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(brackets)
}

#[tauri::command]
pub fn get_matches_for_bracket(
    db: State<Database>,
    bracket_id: String,
) -> Result<Vec<MatchWithTeams>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, bracket_id, round_number, match_number, court_number, team1_id, team2_id,
                   team1_score, team2_score, winner_id, next_match_id, is_bye
            FROM bracket_matches
            WHERE bracket_id = ?1
            ORDER BY round_number DESC, match_number ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let matches: Vec<BracketMatch> = stmt
        .query_map(params![bracket_id], |row| {
            Ok(BracketMatch {
                id: row.get(0)?,
                bracket_id: row.get(1)?,
                round_number: row.get(2)?,
                match_number: row.get(3)?,
                court_number: row.get(4)?,
                team1_id: row.get(5)?,
                team2_id: row.get(6)?,
                team1_score: row.get(7)?,
                team2_score: row.get(8)?,
                winner_id: row.get(9)?,
                next_match_id: row.get(10)?,
                is_bye: row.get::<_, i32>(11)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Fetch team details
    let mut matches_with_teams = Vec::new();
    for m in matches {
        let team1 = if let Some(ref id) = m.team1_id {
            get_team_by_id(&conn, id)?
        } else {
            None
        };
        let team2 = if let Some(ref id) = m.team2_id {
            get_team_by_id(&conn, id)?
        } else {
            None
        };
        let winner = if let Some(ref id) = m.winner_id {
            get_team_by_id(&conn, id)?
        } else {
            None
        };

        matches_with_teams.push(MatchWithTeams {
            id: m.id,
            bracket_id: m.bracket_id,
            round_number: m.round_number,
            match_number: m.match_number,
            court_number: m.court_number,
            team1_id: m.team1_id,
            team2_id: m.team2_id,
            team1_score: m.team1_score,
            team2_score: m.team2_score,
            winner_id: m.winner_id,
            next_match_id: m.next_match_id,
            is_bye: m.is_bye,
            team1,
            team2,
            winner,
        });
    }

    Ok(matches_with_teams)
}

#[tauri::command]
pub fn delete_brackets(db: State<Database>, tournament_id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Get all bracket IDs first
    let bracket_ids: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT id FROM brackets WHERE tournament_id = ?1")
            .map_err(|e| format!("Failed to prepare bracket query: {}", e))?;
        let rows = stmt
            .query_map(params![tournament_id], |row| row.get(0))
            .map_err(|e| format!("Failed to query brackets: {}", e))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Failed to collect bracket IDs: {}", e))?
    };

    if bracket_ids.is_empty() {
        return Ok(());
    }

    // For each bracket, clear next_match_id references and delete matches
    for bracket_id in &bracket_ids {
        // Clear all next_match_id references within this bracket
        conn.execute(
            "UPDATE bracket_matches SET next_match_id = NULL WHERE bracket_id = ?1",
            params![bracket_id],
        )
        .map_err(|e| format!("Failed to clear next_match_id for bracket {}: {}", bracket_id, e))?;

        // Now delete all matches for this bracket
        conn.execute(
            "DELETE FROM bracket_matches WHERE bracket_id = ?1",
            params![bracket_id],
        )
        .map_err(|e| format!("Failed to delete matches for bracket {}: {}", bracket_id, e))?;
    }

    // Now delete the brackets themselves
    for bracket_id in &bracket_ids {
        conn.execute(
            "DELETE FROM brackets WHERE id = ?1",
            params![bracket_id],
        )
        .map_err(|e| format!("Failed to delete bracket {}: {}", bracket_id, e))?;
    }

    Ok(())
}

#[tauri::command]
pub fn generate_brackets(db: State<Database>, tournament_id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Get tournament settings
    let (advance_all, advance_count, bracket_size, number_of_courts): (bool, Option<i32>, i32, i32) =
        conn.query_row(
            "SELECT advance_all, advance_count, bracket_size, number_of_courts FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| {
                Ok((
                    row.get::<_, i32>(0)? != 0,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    // Get ranked teams
    let mut stmt = conn
        .prepare(
            r#"
            SELECT t.id, t.tournament_id, t.captain, t.player2, t.player3, t.region, t.club, t.created_at
            FROM teams t
            JOIN team_standings ts ON t.id = ts.team_id AND t.tournament_id = ts.tournament_id
            WHERE t.tournament_id = ?1
            ORDER BY ts.rank ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let teams: Vec<Team> = stmt
        .query_map(params![tournament_id], |row| {
            Ok(Team {
                id: row.get(0)?,
                tournament_id: row.get(1)?,
                captain: row.get(2)?,
                player2: row.get(3)?,
                player3: row.get(4)?,
                region: row.get(5)?,
                club: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    if teams.is_empty() {
        return Err("No teams to create brackets for".to_string());
    }

    // Determine how many teams advance
    let advancing_count = if advance_all {
        teams.len()
    } else {
        advance_count.unwrap_or(bracket_size) as usize
    };

    let advancing_teams: Vec<&Team> = teams.iter().take(advancing_count).collect();

    // Create brackets based on bracket size
    let bracket_names = ["A", "B", "C", "D", "E", "F", "G", "H"];
    let mut bracket_idx = 0;
    let mut start_idx = 0;
    let now = Utc::now().to_rfc3339();

    while start_idx < advancing_teams.len() {
        let end_idx = std::cmp::min(start_idx + bracket_size as usize, advancing_teams.len());
        let bracket_teams: Vec<&Team> = advancing_teams[start_idx..end_idx].to_vec();

        let bracket_name = if bracket_idx < bracket_names.len() {
            bracket_names[bracket_idx].to_string()
        } else {
            format!("Bracket {}", bracket_idx + 1)
        };

        // Create main bracket
        let bracket_id = Uuid::new_v4().to_string();
        let actual_size = bracket_teams.len() as i32;
        conn.execute(
            r#"
            INSERT INTO brackets (id, tournament_id, name, is_consolante, size, is_complete, created_at)
            VALUES (?1, ?2, ?3, 0, ?4, 0, ?5)
            "#,
            params![bracket_id, tournament_id, bracket_name, actual_size, now],
        )
        .map_err(|e| e.to_string())?;

        // Create matches for this bracket with random pairing and court assignment
        create_bracket_matches(&conn, &bracket_id, &bracket_teams, number_of_courts)?;

        start_idx = end_idx;
        bracket_idx += 1;
    }

    Ok(())
}

fn create_bracket_matches(
    conn: &rusqlite::Connection,
    bracket_id: &str,
    teams: &[&Team],
    number_of_courts: i32,
) -> Result<(), String> {
    let num_teams = teams.len();

    if num_teams < 2 {
        return Err("Need at least 2 teams for a bracket".to_string());
    }

    // Calculate bracket size (next power of 2)
    let bracket_size = (num_teams as f64).log2().ceil().exp2() as usize;
    let num_byes = bracket_size - num_teams;
    let num_rounds = (bracket_size as f64).log2() as i32;
    let first_round_match_count = bracket_size / 2;

    // Teams are already sorted by rank (from standings)
    // Top-ranked teams get BYEs, remaining teams play in round 1
    let bye_teams: Vec<&Team> = teams.iter().take(num_byes).cloned().collect();
    let playing_teams: Vec<&Team> = teams.iter().skip(num_byes).cloned().collect();

    // Shuffle playing teams randomly for first round pairing
    let mut rng = rand::thread_rng();
    let mut shuffled_playing: Vec<&Team> = playing_teams;
    shuffled_playing.shuffle(&mut rng);

    // Create match IDs for all rounds
    let mut match_ids: Vec<Vec<String>> = Vec::new();
    for round in 0..num_rounds {
        let matches_in_round = bracket_size >> (round + 1);
        let mut round_ids = Vec::new();
        for _ in 0..matches_in_round {
            round_ids.push(Uuid::new_v4().to_string());
        }
        match_ids.push(round_ids);
    }

    // Insert first round matches
    // Some are BYE matches (team vs BYE), some are real matches
    let mut playing_idx = 0;
    let mut bye_idx = 0;

    for match_idx in 0..first_round_match_count {
        let match_id = &match_ids[0][match_idx];
        let court_number = (match_idx as i32 % number_of_courts) + 1;

        // Determine if this is a BYE match
        // BYE matches are distributed: first num_byes matches have a BYE
        let is_bye_match = match_idx < num_byes;

        let (team1_id, team2_id, is_bye) = if is_bye_match {
            // BYE match: top-seeded team gets a bye
            let team = bye_teams[bye_idx];
            bye_idx += 1;
            (Some(team.id.clone()), None, true)
        } else {
            // Real match: two teams play
            let t1 = shuffled_playing[playing_idx];
            let t2 = shuffled_playing[playing_idx + 1];
            playing_idx += 2;
            (Some(t1.id.clone()), Some(t2.id.clone()), false)
        };

        conn.execute(
            r#"
            INSERT INTO bracket_matches (id, bracket_id, round_number, match_number, court_number, team1_id, team2_id, next_match_id, is_bye)
            VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, NULL, ?7)
            "#,
            params![
                match_id,
                bracket_id,
                match_idx as i32 + 1,
                court_number,
                team1_id,
                team2_id,
                if is_bye { 1 } else { 0 }
            ],
        )
        .map_err(|e| format!("Failed to insert first round match: {}", e))?;
    }

    // Insert subsequent round matches (empty, waiting for winners)
    for round_idx in 1..match_ids.len() {
        let round_number = (round_idx + 1) as i32;
        for (match_idx, match_id) in match_ids[round_idx].iter().enumerate() {
            let court_number = (match_idx as i32 % number_of_courts) + 1;
            conn.execute(
                r#"
                INSERT INTO bracket_matches (id, bracket_id, round_number, match_number, court_number, team1_id, team2_id, next_match_id, is_bye)
                VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, NULL, 0)
                "#,
                params![
                    match_id,
                    bracket_id,
                    round_number,
                    match_idx as i32 + 1,
                    court_number,
                ],
            )
            .map_err(|e| format!("Failed to insert round {} match: {}", round_number, e))?;
        }
    }

    // Set next_match_id links
    for round_idx in 0..(match_ids.len() - 1) {
        for (match_idx, match_id) in match_ids[round_idx].iter().enumerate() {
            let next_match_idx = match_idx / 2;
            if next_match_idx < match_ids[round_idx + 1].len() {
                let next_match_id = &match_ids[round_idx + 1][next_match_idx];
                conn.execute(
                    "UPDATE bracket_matches SET next_match_id = ?2 WHERE id = ?1",
                    params![match_id, next_match_id],
                )
                .map_err(|e| format!("Failed to set next_match_id: {}", e))?;
            }
        }
    }

    // Auto-advance BYE matches (score 13-7)
    for match_idx in 0..num_byes {
        let match_id = &match_ids[0][match_idx];

        let team1_id: Option<String> = conn
            .query_row(
                "SELECT team1_id FROM bracket_matches WHERE id = ?1",
                params![match_id],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to query BYE match: {}", e))?;

        if let Some(winner) = &team1_id {
            // BYE score is 13-7
            conn.execute(
                "UPDATE bracket_matches SET winner_id = ?2, team1_score = 13, team2_score = 7 WHERE id = ?1",
                params![match_id, winner],
            )
            .map_err(|e| format!("Failed to set BYE winner: {}", e))?;

            // Advance winner to next match
            if match_ids.len() > 1 {
                let next_match_idx = match_idx / 2;
                let next_match_id = &match_ids[1][next_match_idx];
                let is_top_half = match_idx % 2 == 0;
                if is_top_half {
                    conn.execute(
                        "UPDATE bracket_matches SET team1_id = ?2 WHERE id = ?1",
                        params![next_match_id, winner],
                    )
                    .map_err(|e| format!("Failed to advance BYE winner: {}", e))?;
                } else {
                    conn.execute(
                        "UPDATE bracket_matches SET team2_id = ?2 WHERE id = ?1",
                        params![next_match_id, winner],
                    )
                    .map_err(|e| format!("Failed to advance BYE winner: {}", e))?;
                }
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn update_match_score(
    db: State<Database>,
    match_id: String,
    team1_score: i32,
    team2_score: i32,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Get match details
    let (bracket_id, team1_id, team2_id, next_match_id, match_number, round_number): (
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        i32,
        i32,
    ) = conn
        .query_row(
            "SELECT bracket_id, team1_id, team2_id, next_match_id, match_number, round_number FROM bracket_matches WHERE id = ?1",
            params![match_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?)),
        )
        .map_err(|e| e.to_string())?;

    // Determine winner
    let winner_id = if team1_score > team2_score {
        team1_id.clone()
    } else {
        team2_id.clone()
    };

    // Update match
    conn.execute(
        "UPDATE bracket_matches SET team1_score = ?2, team2_score = ?3, winner_id = ?4 WHERE id = ?1",
        params![match_id, team1_score, team2_score, winner_id],
    )
    .map_err(|e| e.to_string())?;

    // Advance winner to next match
    if let (Some(next_id), Some(winner)) = (next_match_id, winner_id) {
        let is_top_half = (match_number - 1) % 2 == 0;
        if is_top_half {
            conn.execute(
                "UPDATE bracket_matches SET team1_id = ?2 WHERE id = ?1",
                params![next_id, winner],
            )
            .map_err(|e| e.to_string())?;
        } else {
            conn.execute(
                "UPDATE bracket_matches SET team2_id = ?2 WHERE id = ?1",
                params![next_id, winner],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Check if bracket is complete (final match has a winner)
    let has_final_winner: bool = conn
        .query_row(
            r#"
            SELECT COUNT(*) > 0
            FROM bracket_matches
            WHERE bracket_id = ?1 AND next_match_id IS NULL AND winner_id IS NOT NULL
            "#,
            params![bracket_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if has_final_winner {
        conn.execute(
            "UPDATE brackets SET is_complete = 1 WHERE id = ?1",
            params![bracket_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Check if first round of a main bracket is complete - create consolante if needed
    if round_number == 1 {
        check_and_create_consolante(&conn, &bracket_id)?;
    }

    Ok(())
}

fn check_and_create_consolante(conn: &rusqlite::Connection, bracket_id: &str) -> Result<(), String> {
    // Get bracket details
    let (tournament_id, bracket_name, is_consolante): (String, String, bool) = conn
        .query_row(
            "SELECT tournament_id, name, is_consolante FROM brackets WHERE id = ?1",
            params![bracket_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i32>(2)? != 0)),
        )
        .map_err(|e| e.to_string())?;

    // Only create consolante for main brackets
    if is_consolante {
        return Ok(());
    }

    // Check if tournament has consolante enabled and get number of courts
    let (has_consolante, number_of_courts): (bool, i32) = conn
        .query_row(
            "SELECT has_consolante, number_of_courts FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| Ok((row.get::<_, i32>(0)? != 0, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    if !has_consolante {
        return Ok(());
    }

    // Check if consolante bracket already exists
    let consolante_name = format!("{}A", bracket_name);
    let consolante_exists: bool = conn
        .query_row(
            "SELECT COUNT(*) > 0 FROM brackets WHERE tournament_id = ?1 AND name = ?2",
            params![tournament_id, consolante_name],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if consolante_exists {
        return Ok(());
    }

    // Check if all first round matches are complete (excluding BYEs which are auto-completed)
    let first_round_incomplete: i32 = conn
        .query_row(
            r#"
            SELECT COUNT(*)
            FROM bracket_matches
            WHERE bracket_id = ?1 AND round_number = 1 AND winner_id IS NULL AND is_bye = 0
            "#,
            params![bracket_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if first_round_incomplete > 0 {
        return Ok(());
    }

    // Get first round losers (non-BYE matches only)
    let mut stmt = conn
        .prepare(
            r#"
            SELECT
                CASE WHEN winner_id = team1_id THEN team2_id ELSE team1_id END as loser_id
            FROM bracket_matches
            WHERE bracket_id = ?1 AND round_number = 1 AND is_bye = 0 AND winner_id IS NOT NULL
            "#,
        )
        .map_err(|e| e.to_string())?;

    let loser_ids: Vec<String> = stmt
        .query_map(params![bracket_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    if loser_ids.len() < 2 {
        // Not enough losers for a consolante bracket
        return Ok(());
    }

    // Create consolante bracket
    let consolante_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO brackets (id, tournament_id, name, is_consolante, size, is_complete, created_at)
        VALUES (?1, ?2, ?3, 1, ?4, 0, ?5)
        "#,
        params![consolante_id, tournament_id, consolante_name, loser_ids.len() as i32, now],
    )
    .map_err(|e| e.to_string())?;

    // Create matches for consolante bracket with random pairing of losers
    create_consolante_matches(conn, &consolante_id, &loser_ids, number_of_courts)?;

    Ok(())
}

fn create_consolante_matches(
    conn: &rusqlite::Connection,
    bracket_id: &str,
    team_ids: &[String],
    number_of_courts: i32,
) -> Result<(), String> {
    let num_teams = team_ids.len();

    if num_teams < 2 {
        return Ok(());
    }

    // Calculate bracket size (next power of 2)
    let bracket_size = (num_teams as f64).log2().ceil().exp2() as usize;
    let num_byes = bracket_size - num_teams;
    let num_rounds = (bracket_size as f64).log2() as i32;
    let first_round_match_count = bracket_size / 2;

    // Consolante teams are already losers, no seeding - shuffle all
    let mut rng = rand::thread_rng();
    let mut shuffled: Vec<&String> = team_ids.iter().collect();
    shuffled.shuffle(&mut rng);

    // First num_byes teams get BYEs (randomly selected after shuffle)
    let bye_teams: Vec<&String> = shuffled.iter().take(num_byes).cloned().collect();
    let playing_teams: Vec<&String> = shuffled.iter().skip(num_byes).cloned().collect();

    // Create match IDs for all rounds
    let mut match_ids: Vec<Vec<String>> = Vec::new();
    for round in 0..num_rounds {
        let matches_in_round = bracket_size >> (round + 1);
        let mut round_ids = Vec::new();
        for _ in 0..matches_in_round {
            round_ids.push(Uuid::new_v4().to_string());
        }
        match_ids.push(round_ids);
    }

    // Insert first round matches
    let mut playing_idx = 0;
    let mut bye_idx = 0;

    for match_idx in 0..first_round_match_count {
        let match_id = &match_ids[0][match_idx];
        let court_number = (match_idx as i32 % number_of_courts) + 1;

        let is_bye_match = match_idx < num_byes;

        let (team1_id, team2_id, is_bye) = if is_bye_match {
            let team = bye_teams[bye_idx];
            bye_idx += 1;
            (Some(team.clone()), None, true)
        } else {
            let t1 = playing_teams[playing_idx];
            let t2 = playing_teams[playing_idx + 1];
            playing_idx += 2;
            (Some(t1.clone()), Some(t2.clone()), false)
        };

        conn.execute(
            r#"
            INSERT INTO bracket_matches (id, bracket_id, round_number, match_number, court_number, team1_id, team2_id, next_match_id, is_bye)
            VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, NULL, ?7)
            "#,
            params![match_id, bracket_id, match_idx as i32 + 1, court_number, team1_id, team2_id, if is_bye { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
    }

    // Insert subsequent rounds (empty) with court assignment
    for round_idx in 1..match_ids.len() {
        let round_number = (round_idx + 1) as i32;
        for (match_idx, match_id) in match_ids[round_idx].iter().enumerate() {
            let court_number = (match_idx as i32 % number_of_courts) + 1;
            conn.execute(
                r#"
                INSERT INTO bracket_matches (id, bracket_id, round_number, match_number, court_number, team1_id, team2_id, next_match_id, is_bye)
                VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, NULL, 0)
                "#,
                params![match_id, bracket_id, round_number, match_idx as i32 + 1, court_number],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Set next_match_id links
    for round_idx in 0..(match_ids.len() - 1) {
        for (match_idx, match_id) in match_ids[round_idx].iter().enumerate() {
            let next_match_idx = match_idx / 2;
            if next_match_idx < match_ids[round_idx + 1].len() {
                let next_match_id = &match_ids[round_idx + 1][next_match_idx];
                conn.execute(
                    "UPDATE bracket_matches SET next_match_id = ?2 WHERE id = ?1",
                    params![match_id, next_match_id],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    // Auto-advance BYE matches (score 13-7)
    for match_idx in 0..num_byes {
        let match_id = &match_ids[0][match_idx];

        let team1_id: Option<String> = conn
            .query_row(
                "SELECT team1_id FROM bracket_matches WHERE id = ?1",
                params![match_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        if let Some(winner) = &team1_id {
            conn.execute(
                "UPDATE bracket_matches SET winner_id = ?2, team1_score = 13, team2_score = 7 WHERE id = ?1",
                params![match_id, winner],
            )
            .map_err(|e| e.to_string())?;

            if match_ids.len() > 1 {
                let next_match_idx = match_idx / 2;
                let next_match_id = &match_ids[1][next_match_idx];
                let is_top_half = match_idx % 2 == 0;
                if is_top_half {
                    conn.execute(
                        "UPDATE bracket_matches SET team1_id = ?2 WHERE id = ?1",
                        params![next_match_id, winner],
                    )
                    .map_err(|e| e.to_string())?;
                } else {
                    conn.execute(
                        "UPDATE bracket_matches SET team2_id = ?2 WHERE id = ?1",
                        params![next_match_id, winner],
                    )
                    .map_err(|e| e.to_string())?;
                }
            }
        }
    }

    Ok(())
}
