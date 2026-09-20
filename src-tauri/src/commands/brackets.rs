use super::courts;
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
                   team1_score, team2_score, winner_id, next_match_id, is_bye, court_is_manual
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
                court_is_manual: row.get::<_, i32>(12)? != 0,
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
            court_is_manual: m.court_is_manual,
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
    let mut conn = db.conn.lock().map_err(|e| e.to_string())?;

    // One transaction for the whole draw. Brackets are inserted before their
    // matches, so a failure partway through used to leave bracket rows with no
    // matches behind - visible in the UI, impossible to play, and blocking a
    // clean retry.
    let tx = conn.transaction().map_err(|e| e.to_string())?;
    build_brackets(&tx, &tournament_id)?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(())
}

fn build_brackets(conn: &rusqlite::Connection, tournament_id: &str) -> Result<(), String> {
    let pairing_method: String = conn
        .query_row(
            "SELECT pairing_method FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if pairing_method == "panache" {
        return Err(
            "Panaché has no elimination bracket. Draw the final game from the standings instead."
                .to_string(),
        );
    }

    // Get tournament settings
    let (advance_all, advance_count, bracket_size, has_consolante): (bool, Option<i32>, i32, bool) =
        conn.query_row(
            "SELECT advance_all, advance_count, bracket_size, has_consolante FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| {
                Ok((
                    row.get::<_, i32>(0)? != 0,
                    row.get(1)?,
                    row.get(2)?,
                    row.get::<_, i32>(3)? != 0,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    // Get ranked teams
    let mut stmt = conn
        .prepare(
            r#"
            SELECT t.id, t.tournament_id, t.team_number, t.captain, t.player2, t.player3, t.region, t.club,
                   t.is_champion, t.is_withdrawn, t.created_at
            FROM teams t
            JOIN team_standings ts ON t.id = ts.team_id AND t.tournament_id = ts.tournament_id
            WHERE t.tournament_id = ?1 AND t.is_withdrawn = 0
            ORDER BY ts.rank ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let teams: Vec<Team> = stmt
        .query_map(params![tournament_id], |row| {
            Ok(Team {
                id: row.get(0)?,
                tournament_id: row.get(1)?,
                team_number: row.get(2)?,
                captain: row.get(3)?,
                player2: row.get(4)?,
                player3: row.get(5)?,
                region: row.get(6)?,
                club: row.get(7)?,
                is_champion: row.get::<_, i32>(8)? != 0,
                is_withdrawn: row.get::<_, i32>(9)? != 0,
                created_at: row.get(10)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    if teams.is_empty() {
        return Err("No teams to create brackets for".to_string());
    }

    let now = Utc::now().to_rfc3339();

    // Check if we should use the simultaneous bracket formation (FPUSA standard)
    // This happens when: advance_all = false AND has_consolante = true
    if !advance_all && has_consolante {
        // FPUSA standard format: Create Concours and Consolante brackets simultaneously
        // Top bracket_size teams go to Concours, next teams go to Consolante
        let concours_teams: Vec<&Team> = teams.iter().take(bracket_size as usize).collect();
        let consolante_teams: Vec<&Team> = teams.iter().skip(bracket_size as usize).take(bracket_size as usize).collect();

        if concours_teams.len() < 2 {
            return Err("Not enough teams for Concours bracket".to_string());
        }

        // Create Concours bracket
        let concours_id = Uuid::new_v4().to_string();
        let concours_power_of_2 = (concours_teams.len() as f64).log2().ceil().exp2() as i32;
        conn.execute(
            r#"
            INSERT INTO brackets (id, tournament_id, name, is_consolante, size, is_complete, created_at)
            VALUES (?1, ?2, 'A', 0, ?3, 0, ?4)
            "#,
            params![concours_id, tournament_id, concours_power_of_2, now],
        )
        .map_err(|e| e.to_string())?;

        create_bracket_matches(conn, &concours_id, &concours_teams)?;

        // Create Consolante bracket if there are enough teams
        if consolante_teams.len() >= 2 {
            let consolante_id = Uuid::new_v4().to_string();
            let consolante_power_of_2 = (consolante_teams.len() as f64).log2().ceil().exp2() as i32;
            conn.execute(
                r#"
                INSERT INTO brackets (id, tournament_id, name, is_consolante, size, is_complete, created_at)
                VALUES (?1, ?2, 'AA', 1, ?3, 0, ?4)
                "#,
                params![consolante_id, tournament_id, consolante_power_of_2, now],
            )
            .map_err(|e| e.to_string())?;

            create_bracket_matches(conn, &consolante_id, &consolante_teams)?;
        }

        assign_bracket_courts(conn, tournament_id)?;
        return Ok(());
    }

    // Original behavior: advance_all or no consolante
    // Determine how many teams advance
    let advancing_count = if advance_all {
        teams.len()
    } else {
        advance_count.unwrap_or(bracket_size) as usize
    };

    let advancing_teams: Vec<&Team> = teams.iter().take(advancing_count).collect();

    if advancing_teams.len() < 2 {
        return Err(format!(
            "Only {} team(s) would advance - a bracket needs at least 2.",
            advancing_teams.len()
        ));
    }

    // Create brackets based on bracket size
    let bracket_names = ["A", "B", "C", "D", "E", "F", "G", "H"];
    let mut bracket_idx = 0;
    let mut start_idx = 0;

    while start_idx < advancing_teams.len() {
        let end_idx = std::cmp::min(start_idx + bracket_size as usize, advancing_teams.len());
        let bracket_teams: Vec<&Team> = advancing_teams[start_idx..end_idx].to_vec();

        // A final chunk of one cannot be a bracket - there is nobody to play.
        // Leave it out, the same way the consolante path already drops a
        // remainder it cannot pair, rather than inserting a bracket row and
        // then failing on its matches.
        if bracket_teams.len() < 2 {
            break;
        }

        let bracket_name = if bracket_idx < bracket_names.len() {
            bracket_names[bracket_idx].to_string()
        } else {
            format!("Bracket {}", bracket_idx + 1)
        };

        // Create main bracket
        let bracket_id = Uuid::new_v4().to_string();
        // Store power-of-2 bracket size for proper round calculation in UI
        let power_of_2_size = (bracket_teams.len() as f64).log2().ceil().exp2() as i32;
        conn.execute(
            r#"
            INSERT INTO brackets (id, tournament_id, name, is_consolante, size, is_complete, created_at)
            VALUES (?1, ?2, ?3, 0, ?4, 0, ?5)
            "#,
            params![bracket_id, tournament_id, bracket_name, power_of_2_size, now],
        )
        .map_err(|e| e.to_string())?;

        // Create matches for this bracket with random pairing
        create_bracket_matches(conn, &bracket_id, &bracket_teams)?;

        start_idx = end_idx;
        bracket_idx += 1;
    }

    assign_bracket_courts(conn, tournament_id)?;

    Ok(())
}

/// The wave of play a bracket match belongs to.
///
/// A consolante is drawn from its main bracket's first-round losers, so it
/// starts one round behind: main-bracket round N is on court at the same time
/// as its consolante's round N-1. Matches sharing a wave are played
/// simultaneously and so must not share a court.
fn play_wave(round_number: i32, is_consolante: bool) -> i32 {
    if is_consolante {
        round_number + 1
    } else {
        round_number
    }
}

/// One bracket match, as court assignment sees it.
struct CourtCandidate {
    id: String,
    wave: i32,
    is_bye: bool,
    /// Already played, or set by hand: its court is not ours to move.
    is_fixed: bool,
    /// A score has been entered, which also means its wave is under way.
    played: bool,
    court: Option<i32>,
    /// The teams on court, where they are known. A match still waiting on its
    /// feeders has nobody yet, so it has no history to avoid.
    teams: Vec<String>,
}

/// Numbers the courts across every bracket in the tournament.
///
/// Courts used to be numbered within each bracket, which sent four different
/// games to court 1 as soon as more than one bracket ran. A court number only
/// means something tournament-wide, so this renumbers every bracket together
/// and is re-run whenever a bracket is added or a result comes in.
///
/// Within a wave the courts are drawn against what the teams have already had,
/// counting the qualifying rounds - a team that spent the morning on court 3
/// should not be sent back to it for their quarter-final. A match that has been
/// played, or whose court the operator set by hand, keeps the court it has and
/// simply takes it out of what the rest of the wave can use.
///
/// BYEs are left without a court: nobody plays them, so they take up no space.
fn assign_bracket_courts(conn: &rusqlite::Connection, tournament_id: &str) -> Result<(), String> {
    let number_of_courts: i32 = conn
        .query_row(
            "SELECT number_of_courts FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if number_of_courts < 1 {
        return Ok(());
    }

    let mut stmt = conn
        .prepare(
            r#"
            SELECT m.id, m.round_number, b.is_consolante, m.is_bye, m.court_is_manual,
                   m.court_number, m.team1_id, m.team2_id, m.team1_score, m.team2_score
            FROM bracket_matches m
            JOIN brackets b ON b.id = m.bracket_id
            WHERE b.tournament_id = ?1
            ORDER BY b.name ASC, m.round_number ASC, m.match_number ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let matches: Vec<CourtCandidate> = stmt
        .query_map(params![tournament_id], |row| {
            let is_manual: bool = row.get::<_, i32>(4)? != 0;
            let court: Option<i32> = row.get(5)?;
            let team1: Option<String> = row.get(6)?;
            let team2: Option<String> = row.get(7)?;
            let score1: Option<i32> = row.get(8)?;
            let score2: Option<i32> = row.get(9)?;
            let played = score1.is_some() || score2.is_some();

            Ok(CourtCandidate {
                id: row.get(0)?,
                wave: play_wave(row.get(1)?, row.get::<_, i32>(2)? != 0),
                is_bye: row.get::<_, i32>(3)? != 0,
                is_fixed: court.is_some() && (is_manual || played),
                played,
                court,
                teams: team1.into_iter().chain(team2).collect(),
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // What the qualifying rounds already put these teams on. Earlier waves add
    // to it as they are settled, so a bracket run reads as one continuation of
    // the morning rather than starting over.
    let mut history = courts::load_qualifying_history(conn, tournament_id)?;

    let mut waves: Vec<i32> = matches
        .iter()
        .filter(|m| !m.is_bye)
        .map(|m| m.wave)
        .collect();
    waves.sort_unstable();
    waves.dedup();

    for wave in &waves {
        let playing: Vec<&CourtCandidate> = matches
            .iter()
            .filter(|m| !m.is_bye && m.wave == *wave)
            .collect();

        // Once any game in a wave has been scored the wave is on the ground:
        // the other games are being played right now, on the courts the sheet
        // sent them to. Renumbering under them would be worse than any repeat,
        // so a wave that has started keeps every court it has - it only feeds
        // the history the later waves are drawn against.
        let under_way = playing.iter().any(|m| m.played);
        if under_way {
            history.previous.clear();
            for candidate in &playing {
                if let Some(court) = candidate.court {
                    for team in &candidate.teams {
                        history.record(team, court, true);
                    }
                }
            }
            continue;
        }

        // A wave with more games than the venue has courts has to double up:
        // the extra games wait for a court to free up.
        let mut pool = courts::court_slots(playing.len(), number_of_courts);
        for fixed in playing.iter().filter(|m| m.is_fixed) {
            if let Some(court) = fixed.court {
                if let Some(idx) = pool.iter().position(|slot| *slot == court) {
                    pool.remove(idx);
                } else if !pool.is_empty() {
                    // The operator put a game on a court outside the pool. It
                    // stays there; the wave just has one fewer slot to share.
                    pool.pop();
                }
            }
        }

        let movable: Vec<&CourtCandidate> = playing.iter().copied().filter(|m| !m.is_fixed).collect();
        let sides: Vec<Vec<String>> = movable.iter().map(|m| m.teams.clone()).collect();
        let assigned = courts::assign_courts_from_slots(&sides, &history, &pool);

        for (candidate, court) in movable.iter().zip(&assigned) {
            conn.execute(
                "UPDATE bracket_matches SET court_number = ?2 WHERE id = ?1",
                params![candidate.id, court],
            )
            .map_err(|e| e.to_string())?;
        }

        // Only the wave just settled counts as "the round before" the next one.
        history.previous.clear();
        let settled = movable
            .iter()
            .zip(assigned.iter().copied())
            .chain(playing.iter().filter(|m| m.is_fixed).filter_map(|m| m.court.map(|c| (m, c))));
        for (candidate, court) in settled {
            for team in &candidate.teams {
                history.record(team, court, true);
            }
        }
    }

    // Byes never hold a court, however they were left by an earlier run.
    for candidate in matches.iter().filter(|m| m.is_bye) {
        conn.execute(
            "UPDATE bracket_matches SET court_number = NULL WHERE id = ?1",
            params![candidate.id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

fn create_bracket_matches(
    conn: &rusqlite::Connection,
    bracket_id: &str,
    teams: &[&Team],
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
            VALUES (?1, ?2, 1, ?3, NULL, ?4, ?5, NULL, ?6)
            "#,
            params![
                match_id,
                bracket_id,
                match_idx as i32 + 1,
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
            conn.execute(
                r#"
                INSERT INTO bracket_matches (id, bracket_id, round_number, match_number, court_number, team1_id, team2_id, next_match_id, is_bye)
                VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, NULL, 0)
                "#,
                params![match_id, bracket_id, round_number, match_idx as i32 + 1],
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

    // The winner has just moved into the next round, so a match that had nobody
    // in it now has teams and a court history to keep clear of. Played and
    // hand-set courts are left where they are.
    let tournament_id: String = conn
        .query_row(
            "SELECT tournament_id FROM brackets WHERE id = ?1",
            params![bracket_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    assign_bracket_courts(&conn, &tournament_id)?;

    Ok(())
}

/// Moves one bracket match to another court.
///
/// Flags it as hand-set, which is what keeps the automatic renumbering - it
/// re-runs on every result - from putting it straight back.
#[tauri::command]
pub fn update_match_court(
    db: State<Database>,
    match_id: String,
    court_number: i32,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    move_match_to_court(&conn, &match_id, court_number)
}

/// Moves a bracket match to another court. Same rule as the qualifying games:
/// a clash is allowed through, a played match is not moved.
fn move_match_to_court(
    conn: &rusqlite::Connection,
    match_id: &str,
    court_number: i32,
) -> Result<(), String> {
    if court_number < 1 {
        return Err("A court number starts at 1.".to_string());
    }

    let (played, is_bye): (bool, bool) = conn
        .query_row(
            "SELECT team1_score IS NOT NULL OR team2_score IS NOT NULL, is_bye \
             FROM bracket_matches WHERE id = ?1",
            params![match_id],
            |row| Ok((row.get::<_, i32>(0)? != 0, row.get::<_, i32>(1)? != 0)),
        )
        .map_err(|e| e.to_string())?;

    if is_bye {
        return Err("A bye is not played on a court.".to_string());
    }
    if played {
        return Err("This match has been played; its court can no longer be changed.".to_string());
    }

    conn.execute(
        "UPDATE bracket_matches SET court_number = ?2, court_is_manual = 1 WHERE id = ?1",
        params![match_id, court_number],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// The bracket matches currently sharing a court with another match playing at
/// the same time.
///
/// A clash spans every bracket in the tournament, and the frontend only ever
/// holds the matches of the bracket being viewed, so the check belongs here
/// where `play_wave` already defines what "at the same time" means.
#[tauri::command]
pub fn get_bracket_court_conflicts(
    db: State<Database>,
    tournament_id: String,
) -> Result<Vec<String>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT m.id, m.round_number, b.is_consolante, m.court_number
            FROM bracket_matches m
            JOIN brackets b ON b.id = m.bracket_id
            WHERE b.tournament_id = ?1 AND m.is_bye = 0 AND m.court_number IS NOT NULL
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, i32, i32)> = stmt
        .query_map(params![tournament_id], |row| {
            Ok((
                row.get(0)?,
                play_wave(row.get(1)?, row.get::<_, i32>(2)? != 0),
                row.get(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut by_slot: std::collections::HashMap<(i32, i32), Vec<String>> =
        std::collections::HashMap::new();
    for (id, wave, court) in rows {
        by_slot.entry((wave, court)).or_default().push(id);
    }

    let mut clashing: Vec<String> = by_slot
        .into_values()
        .filter(|ids| ids.len() > 1)
        .flatten()
        .collect();
    clashing.sort();
    Ok(clashing)
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

    // Check if tournament has consolante enabled and get advance_all setting
    let (has_consolante, advance_all): (bool, bool) = conn
        .query_row(
            "SELECT has_consolante, advance_all FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| Ok((row.get::<_, i32>(0)? != 0, row.get::<_, i32>(1)? != 0)),
        )
        .map_err(|e| e.to_string())?;

    if !has_consolante {
        return Ok(());
    }

    // Only create consolante from first-round losers when advance_all is true
    // When advance_all is false, consolante was created simultaneously at bracket generation
    if !advance_all {
        return Ok(());
    }

    // Check if consolante bracket already exists
    // Consolante name doubles the bracket letter: A -> AA, B -> BB, etc.
    let consolante_name = format!("{}{}", bracket_name, bracket_name);
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
    // Store power-of-2 bracket size for proper round calculation in UI
    let power_of_2_size = (loser_ids.len() as f64).log2().ceil().exp2() as i32;

    conn.execute(
        r#"
        INSERT INTO brackets (id, tournament_id, name, is_consolante, size, is_complete, created_at)
        VALUES (?1, ?2, ?3, 1, ?4, 0, ?5)
        "#,
        params![consolante_id, tournament_id, consolante_name, power_of_2_size, now],
    )
    .map_err(|e| e.to_string())?;

    // Create matches for consolante bracket with random pairing of losers
    create_consolante_matches(conn, &consolante_id, &loser_ids)?;

    // The new bracket shares courts with the rounds still to play, so every
    // bracket has to be renumbered together.
    assign_bracket_courts(conn, &tournament_id)?;

    Ok(())
}

fn create_consolante_matches(
    conn: &rusqlite::Connection,
    bracket_id: &str,
    team_ids: &[String],
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
            VALUES (?1, ?2, 1, ?3, NULL, ?4, ?5, NULL, ?6)
            "#,
            params![match_id, bracket_id, match_idx as i32 + 1, team1_id, team2_id, if is_bye { 1 } else { 0 }],
        )
        .map_err(|e| e.to_string())?;
    }

    // Insert subsequent rounds (empty); courts are numbered tournament-wide later
    for round_idx in 1..match_ids.len() {
        let round_number = (round_idx + 1) as i32;
        for (match_idx, match_id) in match_ids[round_idx].iter().enumerate() {
            conn.execute(
                r#"
                INSERT INTO bracket_matches (id, bracket_id, round_number, match_number, court_number, team1_id, team2_id, next_match_id, is_bye)
                VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, NULL, 0)
                "#,
                params![match_id, bracket_id, round_number, match_idx as i32 + 1],
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;
    use rusqlite::Connection;

    const TOURNAMENT_ID: &str = "t1";

    fn setup(number_of_courts: i32) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();
        conn.execute(
            r#"
            INSERT INTO tournaments (id, name, team_composition, tournament_type, start_date,
                end_date, director, head_umpire, format, day_type, number_of_courts,
                number_of_qualifying_rounds, has_consolante, advance_all, bracket_size,
                pairing_method, region_avoidance, created_at, updated_at)
            VALUES (?1, 'T', 'select', 'open', '2026-01-01', '2026-01-01', 'd', 'u',
                'double', 'single', ?2, 5, 1, 1, 32, 'swissHotel', 0, 'now', 'now')
            "#,
            params![TOURNAMENT_ID, number_of_courts],
        )
        .unwrap();
        conn
    }

    /// Adds a bracket shaped like a real one: `size` entrants, so round N holds
    /// size / 2^N matches. `byes` first-round matches are walkovers.
    fn add_bracket(conn: &Connection, name: &str, is_consolante: bool, size: i32, byes: i32) {
        let bracket_id = format!("b-{}", name);
        conn.execute(
            r#"
            INSERT INTO brackets (id, tournament_id, name, is_consolante, size, is_complete, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, 0, 'now')
            "#,
            params![bracket_id, TOURNAMENT_ID, name, if is_consolante { 1 } else { 0 }, size],
        )
        .unwrap();

        let rounds = (size as f64).log2() as i32;
        for round in 1..=rounds {
            let count = size >> round;
            for match_number in 1..=count {
                let is_bye = round == 1 && match_number <= byes;
                conn.execute(
                    r#"
                    INSERT INTO bracket_matches (id, bracket_id, round_number, match_number,
                        court_number, team1_id, team2_id, next_match_id, is_bye)
                    VALUES (?1, ?2, ?3, ?4, NULL, NULL, NULL, NULL, ?5)
                    "#,
                    params![
                        format!("{}-r{}-m{}", name, round, match_number),
                        bracket_id,
                        round,
                        match_number,
                        if is_bye { 1 } else { 0 }
                    ],
                )
                .unwrap();
            }
        }
    }

    /// (wave, court) for every match that was given a court.
    fn assigned_waves(conn: &Connection) -> Vec<(i32, i32)> {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT m.round_number, b.is_consolante, m.court_number
                FROM bracket_matches m
                JOIN brackets b ON b.id = m.bracket_id
                WHERE m.court_number IS NOT NULL
                "#,
            )
            .unwrap();
        stmt.query_map([], |row| {
            Ok((
                play_wave(row.get(0)?, row.get::<_, i32>(1)? != 0),
                row.get(2)?,
            ))
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    }

    /// The regression this whole allocator exists for: numbering courts within
    /// each bracket sent bracket A, B, C and D all to court 1 at the same time.
    #[test]
    fn concurrent_brackets_never_share_a_court() {
        let conn = setup(64);
        for name in ["A", "B", "C", "D"] {
            add_bracket(&conn, name, false, 32, 0);
        }
        for name in ["AA", "BB", "CC", "DD"] {
            add_bracket(&conn, name, true, 16, 0);
        }

        assign_bracket_courts(&conn, TOURNAMENT_ID).unwrap();

        let mut seen = assigned_waves(&conn);
        let total = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), total, "two games in one wave landed on one court");
    }

    /// A consolante's first round runs alongside its main bracket's second, so
    /// the two must not be numbered as if each had the courts to itself.
    #[test]
    fn a_consolante_shares_the_wave_of_its_main_brackets_next_round() {
        let conn = setup(64);
        add_bracket(&conn, "A", false, 32, 0);
        add_bracket(&conn, "AA", true, 16, 0);

        assign_bracket_courts(&conn, TOURNAMENT_ID).unwrap();

        let courts = |bracket: &str, round: i32| -> Vec<i32> {
            let mut stmt = conn
                .prepare(
                    "SELECT m.court_number FROM bracket_matches m JOIN brackets b ON b.id = m.bracket_id
                     WHERE b.name = ?1 AND m.round_number = ?2 ORDER BY m.match_number",
                )
                .unwrap();
            stmt.query_map(params![bracket, round], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<i32>, _>>()
                .unwrap()
        };

        // A's round 2 (8 games) and AA's round 1 (8 games) play together, so
        // between them they need sixteen courts, not eight used twice. Which
        // game gets which court is the draw's business - it spreads teams off
        // courts they have already had - so only the sharing is asserted here.
        let mut wave: Vec<i32> = courts("A", 2);
        wave.extend(courts("AA", 1));
        assert_eq!(wave.len(), 16);

        let mut distinct = wave.clone();
        distinct.sort_unstable();
        distinct.dedup();
        assert_eq!(
            distinct.len(),
            16,
            "the consolante was numbered as if it had the courts to itself: {:?}",
            wave
        );
    }

    #[test]
    fn byes_are_left_without_a_court() {
        let conn = setup(8);
        add_bracket(&conn, "A", false, 8, 2);

        assign_bracket_courts(&conn, TOURNAMENT_ID).unwrap();

        let byes_with_courts: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM bracket_matches WHERE is_bye = 1 AND court_number IS NOT NULL",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(byes_with_courts, 0);

        // The two real first-round games take courts 1 and 2, not 3 and 4:
        // a walkover should not hold a court open. Which of them gets which is
        // the draw's business, so the courts are compared as a set.
        let mut first_round: Vec<i32> = {
            let mut stmt = conn
                .prepare(
                    "SELECT court_number FROM bracket_matches
                     WHERE round_number = 1 AND is_bye = 0 ORDER BY match_number",
                )
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        first_round.sort_unstable();
        assert_eq!(first_round, vec![1, 2]);
    }

    /// Courts are renumbered on every result and whenever a bracket is added.
    /// A court the operator set by hand has to survive all of that, or moving a
    /// game would last only until the next score was entered.
    #[test]
    fn a_hand_set_court_survives_the_renumbering() {
        let conn = setup(8);
        add_bracket(&conn, "A", false, 8, 0);

        assign_bracket_courts(&conn, TOURNAMENT_ID).unwrap();

        conn.execute(
            "UPDATE bracket_matches SET court_number = 7, court_is_manual = 1 WHERE id = 'A-r1-m1'",
            [],
        )
        .unwrap();

        assign_bracket_courts(&conn, TOURNAMENT_ID).unwrap();

        let court: i32 = conn
            .query_row(
                "SELECT court_number FROM bracket_matches WHERE id = 'A-r1-m1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(court, 7, "the hand-set court was renumbered away");

        // And the court it was moved onto is not handed out twice in that wave.
        let mut wave: Vec<i32> = {
            let mut stmt = conn
                .prepare(
                    "SELECT court_number FROM bracket_matches
                     WHERE round_number = 1 AND is_bye = 0 AND court_number IS NOT NULL",
                )
                .unwrap();
            stmt.query_map([], |row| row.get(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
        };
        let total = wave.len();
        wave.sort_unstable();
        wave.dedup();
        assert_eq!(wave.len(), total, "the hand-set court was handed out again");
    }

    /// Courts are redrawn whenever a result comes in, so that a match whose
    /// teams have only just arrived gets a court chosen against their history.
    /// The games alongside it are on the ground at that moment, though, and
    /// moving them out from under the printed sheet would be worse than any
    /// repeat: once a wave has a score in it, every court in it stands.
    #[test]
    fn a_wave_that_has_started_is_not_renumbered() {
        let conn = setup(8);
        add_bracket(&conn, "A", false, 8, 0);

        assign_bracket_courts(&conn, TOURNAMENT_ID).unwrap();

        let before = |id: &str| -> i32 {
            conn.query_row(
                "SELECT court_number FROM bracket_matches WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .unwrap()
        };
        let untouched = before("A-r1-m2");

        // One game in the wave is scored; the rest are being played right now.
        conn.execute(
            "UPDATE bracket_matches SET team1_score = 13, team2_score = 7 WHERE id = 'A-r1-m1'",
            [],
        )
        .unwrap();

        assign_bracket_courts(&conn, TOURNAMENT_ID).unwrap();

        assert_eq!(
            before("A-r1-m2"),
            untouched,
            "a game in progress was moved to another court"
        );
    }

    /// More games than courts is a real possibility for a small club; the
    /// numbering wraps rather than handing out a court that does not exist.
    #[test]
    fn a_wave_larger_than_the_venue_wraps_within_the_court_count() {
        let conn = setup(4);
        add_bracket(&conn, "A", false, 32, 0);

        assign_bracket_courts(&conn, TOURNAMENT_ID).unwrap();

        let out_of_range: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM bracket_matches WHERE court_number < 1 OR court_number > 4",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(out_of_range, 0);
    }

    /// Registers `n` ranked teams so bracket generation has a field to draw from.
    fn add_ranked_teams(conn: &Connection, n: i32) {
        for i in 1..=n {
            let id = format!("team{}", i);
            conn.execute(
                r#"INSERT INTO teams (id, tournament_id, team_number, captain, player2, created_at)
                   VALUES (?1, ?2, ?3, ?4, '', 'now')"#,
                params![id, TOURNAMENT_ID, i, format!("C{}", i)],
            )
            .unwrap();
            conn.execute(
                r#"INSERT INTO team_standings (id, tournament_id, team_id, wins, losses,
                       points_for, points_against, differential, buchholz_score,
                       fine_buchholz_score, point_quotient, is_eliminated, rank)
                   VALUES (?1, ?2, ?3, 0, 0, 0, 0, 0, 0, 0, 0, 0, ?4)"#,
                params![format!("s{}", i), TOURNAMENT_ID, id, i],
            )
            .unwrap();
        }
    }

    fn configure(conn: &Connection, advance_all: bool, has_consolante: bool, bracket_size: i32) {
        conn.execute(
            "UPDATE tournaments SET advance_all = ?2, has_consolante = ?3, bracket_size = ?4 WHERE id = ?1",
            params![TOURNAMENT_ID, advance_all as i32, has_consolante as i32, bracket_size],
        )
        .unwrap();
    }

    fn bracket_rows(conn: &Connection) -> Vec<(String, i32, i32)> {
        let mut stmt = conn
            .prepare(
                r#"SELECT b.name, b.size, (SELECT COUNT(*) FROM bracket_matches m WHERE m.bracket_id = b.id)
                   FROM brackets b WHERE b.tournament_id = ?1 ORDER BY b.name"#,
            )
            .unwrap();
        stmt.query_map(params![TOURNAMENT_ID], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    /// Nine advancing teams with a bracket size of eight leaves one team who
    /// has nobody to play. That used to insert a bracket row and only then
    /// fail on its matches, leaving an unplayable bracket behind.
    #[test]
    fn a_stranded_team_does_not_leave_an_empty_bracket() {
        let conn = setup(16);
        add_ranked_teams(&conn, 9);
        configure(&conn, true, false, 8);

        build_brackets(&conn, TOURNAMENT_ID).unwrap();

        let rows = bracket_rows(&conn);
        assert_eq!(rows.len(), 1, "expected one bracket, got {:?}", rows);
        assert_eq!(rows[0].0, "A");
        assert!(rows[0].2 > 0, "bracket A has no matches");
        // The 9th team is left out rather than put in a bracket of one.
        let placed: i32 = conn
            .query_row(
                r#"SELECT COUNT(DISTINCT t) FROM (
                       SELECT team1_id AS t FROM bracket_matches WHERE team1_id IS NOT NULL
                       UNION SELECT team2_id FROM bracket_matches WHERE team2_id IS NOT NULL)"#,
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(placed, 8, "expected 8 teams placed, got {}", placed);
    }

    /// Whatever the field size, no bracket may exist without matches.
    #[test]
    fn no_bracket_is_ever_left_without_matches() {
        for n in [2, 3, 5, 8, 9, 15, 17, 33] {
            let conn = setup(64);
            add_ranked_teams(&conn, n);
            configure(&conn, true, false, 8);

            let result = build_brackets(&conn, TOURNAMENT_ID);
            let rows = bracket_rows(&conn);

            if result.is_err() {
                assert!(rows.is_empty(), "{} teams: failed but left {:?}", n, rows);
                continue;
            }
            for (name, size, matches) in &rows {
                assert!(*matches > 0, "{} teams: bracket {} has no matches", n, name);
                assert!(*size >= 2, "{} teams: bracket {} has size {}", n, name, size);
            }
        }
    }

    /// A field too small to fill any bracket fails cleanly instead of
    /// silently producing nothing.
    #[test]
    fn a_field_too_small_for_a_bracket_is_rejected() {
        let conn = setup(8);
        add_ranked_teams(&conn, 1);
        configure(&conn, true, false, 8);

        let err = build_brackets(&conn, TOURNAMENT_ID).unwrap_err();
        assert!(err.contains("at least 2"), "unhelpful error: {}", err);
        assert!(bracket_rows(&conn).is_empty(), "left brackets behind after failing");
    }
}
