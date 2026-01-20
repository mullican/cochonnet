use crate::db::Database;
use crate::models::{GameWithTeams, QualifyingGame, QualifyingRound, Team, TeamStanding};
use crate::commands::teams::get_team_by_id;
use chrono::Utc;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rusqlite::params;
use std::collections::{HashMap, HashSet};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_qualifying_rounds(
    db: State<Database>,
    tournament_id: String,
) -> Result<Vec<QualifyingRound>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, tournament_id, round_number, is_complete, created_at
            FROM qualifying_rounds
            WHERE tournament_id = ?1
            ORDER BY round_number ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rounds = stmt
        .query_map(params![tournament_id], |row| {
            Ok(QualifyingRound {
                id: row.get(0)?,
                tournament_id: row.get(1)?,
                round_number: row.get(2)?,
                is_complete: row.get::<_, i32>(3)? != 0,
                created_at: row.get(4)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(rounds)
}

#[tauri::command]
pub fn get_games_for_round(
    db: State<Database>,
    round_id: String,
) -> Result<Vec<GameWithTeams>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, round_id, court_number, team1_id, team2_id, team1_score, team2_score, is_bye
            FROM qualifying_games
            WHERE round_id = ?1
            ORDER BY court_number ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let games: Vec<QualifyingGame> = stmt
        .query_map(params![round_id], |row| {
            Ok(QualifyingGame {
                id: row.get(0)?,
                round_id: row.get(1)?,
                court_number: row.get(2)?,
                team1_id: row.get(3)?,
                team2_id: row.get(4)?,
                team1_score: row.get(5)?,
                team2_score: row.get(6)?,
                is_bye: row.get::<_, i32>(7)? != 0,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Fetch team details
    let mut games_with_teams = Vec::new();
    for game in games {
        let team1 = if let Some(ref id) = game.team1_id {
            get_team_by_id(&conn, id)?
        } else {
            None
        };
        let team2 = if let Some(ref id) = game.team2_id {
            get_team_by_id(&conn, id)?
        } else {
            None
        };

        games_with_teams.push(GameWithTeams {
            id: game.id,
            round_id: game.round_id,
            court_number: game.court_number,
            team1_id: game.team1_id,
            team2_id: game.team2_id,
            team1_score: game.team1_score,
            team2_score: game.team2_score,
            is_bye: game.is_bye,
            team1,
            team2,
        });
    }

    Ok(games_with_teams)
}

#[tauri::command]
pub fn generate_pairings(
    db: State<Database>,
    tournament_id: String,
) -> Result<QualifyingRound, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Get tournament info
    let (pairing_method, number_of_courts, region_avoidance): (String, i32, bool) = conn
        .query_row(
            "SELECT pairing_method, number_of_courts, region_avoidance FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i32>(2)? != 0)),
        )
        .map_err(|e| e.to_string())?;

    // Get current round number
    let current_round: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(round_number), 0) FROM qualifying_rounds WHERE tournament_id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let new_round_number = current_round + 1;

    // Get all teams
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, tournament_id, captain, player2, player3, region, club, created_at
            FROM teams
            WHERE tournament_id = ?1
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
        return Err("No teams registered for this tournament".to_string());
    }

    // Get pairing history
    let mut pairing_stmt = conn
        .prepare("SELECT team1_id, team2_id FROM pairing_history WHERE tournament_id = ?1")
        .map_err(|e| e.to_string())?;

    let pairing_history: HashSet<(String, String)> = pairing_stmt
        .query_map(params![tournament_id], |row| {
            let t1: String = row.get(0)?;
            let t2: String = row.get(1)?;
            Ok((t1, t2))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .flat_map(|(t1, t2)| vec![(t1.clone(), t2.clone()), (t2, t1)])
        .collect();

    // Get court history for rotation
    let mut court_stmt = conn
        .prepare("SELECT team_id, court_number FROM court_history WHERE tournament_id = ?1")
        .map_err(|e| e.to_string())?;

    let court_history: HashMap<String, Vec<i32>> = {
        let mut map: HashMap<String, Vec<i32>> = HashMap::new();
        let rows = court_stmt
            .query_map(params![tournament_id], |row| {
                let team_id: String = row.get(0)?;
                let court: i32 = row.get(1)?;
                Ok((team_id, court))
            })
            .map_err(|e| e.to_string())?;

        for row in rows {
            if let Ok((team_id, court)) = row {
                map.entry(team_id).or_default().push(court);
            }
        }
        map
    };

    // Get standings for Swiss pairing
    let mut standings_stmt = conn
        .prepare(
            r#"
            SELECT team_id, wins, losses, points_for, points_against, differential, buchholz_score
            FROM team_standings
            WHERE tournament_id = ?1
            ORDER BY wins DESC, differential DESC, points_for DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let standings: HashMap<String, TeamStanding> = standings_stmt
        .query_map(params![tournament_id], |row| {
            Ok(TeamStanding {
                id: String::new(),
                tournament_id: tournament_id.clone(),
                team_id: row.get(0)?,
                wins: row.get(1)?,
                losses: row.get(2)?,
                points_for: row.get(3)?,
                points_against: row.get(4)?,
                differential: row.get(5)?,
                buchholz_score: row.get(6)?,
                rank: 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .map(|s| (s.team_id.clone(), s))
        .collect();

    // Generate pairings based on method
    let pairings = if pairing_method == "swiss" {
        generate_swiss_pairings(&teams, &standings, &pairing_history, region_avoidance)?
    } else {
        generate_round_robin_pairings(&teams, new_round_number)?
    };

    // Assign courts with rotation
    let games = assign_courts(pairings, number_of_courts, &court_history)?;

    // Create the round
    let round_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO qualifying_rounds (id, tournament_id, round_number, is_complete, created_at)
        VALUES (?1, ?2, ?3, 0, ?4)
        "#,
        params![round_id, tournament_id, new_round_number, now],
    )
    .map_err(|e| e.to_string())?;

    // Insert games and track history
    for (court_number, (team1_id, team2_id)) in games.iter().enumerate() {
        let game_id = Uuid::new_v4().to_string();
        let court = (court_number as i32) + 1;
        let is_bye = team2_id.is_none();

        conn.execute(
            r#"
            INSERT INTO qualifying_games (id, round_id, court_number, team1_id, team2_id, is_bye)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                game_id,
                round_id,
                court,
                team1_id,
                team2_id,
                if is_bye { 1 } else { 0 }
            ],
        )
        .map_err(|e| e.to_string())?;

        // Record pairing history
        if let (Some(t1), Some(t2)) = (team1_id, team2_id) {
            let history_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO pairing_history (id, tournament_id, team1_id, team2_id, round_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![history_id, tournament_id, t1, t2, round_id],
            )
            .map_err(|e| e.to_string())?;
        }

        // Record court history
        if let Some(t1) = team1_id {
            let history_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO court_history (id, tournament_id, team_id, court_number, round_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![history_id, tournament_id, t1, court, round_id],
            )
            .map_err(|e| e.to_string())?;
        }
        if let Some(t2) = team2_id {
            let history_id = Uuid::new_v4().to_string();
            conn.execute(
                "INSERT INTO court_history (id, tournament_id, team_id, court_number, round_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![history_id, tournament_id, t2, court, round_id],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(QualifyingRound {
        id: round_id,
        tournament_id,
        round_number: new_round_number,
        is_complete: false,
        created_at: now,
    })
}

fn generate_swiss_pairings(
    teams: &[Team],
    standings: &HashMap<String, TeamStanding>,
    pairing_history: &HashSet<(String, String)>,
    region_avoidance: bool,
) -> Result<Vec<(String, Option<String>)>, String> {
    let mut rng = thread_rng();

    // Sort teams by standings
    let mut sorted_teams: Vec<&Team> = teams.iter().collect();
    sorted_teams.sort_by(|a, b| {
        let sa = standings.get(&a.id);
        let sb = standings.get(&b.id);

        match (sa, sb) {
            (Some(sa), Some(sb)) => {
                sb.wins
                    .cmp(&sa.wins)
                    .then(sb.differential.cmp(&sa.differential))
                    .then(sb.points_for.cmp(&sa.points_for))
            }
            _ => std::cmp::Ordering::Equal,
        }
    });

    let mut pairings: Vec<(String, Option<String>)> = Vec::new();
    let mut paired: HashSet<String> = HashSet::new();

    // If odd number of teams, handle BYE
    let needs_bye = sorted_teams.len() % 2 == 1;

    // Try to pair teams from similar score groups
    for i in 0..sorted_teams.len() {
        let team = sorted_teams[i];
        if paired.contains(&team.id) {
            continue;
        }

        // Find best opponent
        let mut best_opponent: Option<&Team> = None;

        for j in (i + 1)..sorted_teams.len() {
            let opponent = sorted_teams[j];
            if paired.contains(&opponent.id) {
                continue;
            }

            // Check if already played
            if pairing_history.contains(&(team.id.clone(), opponent.id.clone())) {
                continue;
            }

            // Check region avoidance
            if region_avoidance {
                if let (Some(r1), Some(r2)) = (&team.region, &opponent.region) {
                    if !r1.is_empty() && !r2.is_empty() && r1 == r2 {
                        continue;
                    }
                }
            }

            best_opponent = Some(opponent);
            break;
        }

        // If no valid opponent found, try again without constraints
        if best_opponent.is_none() {
            for j in (i + 1)..sorted_teams.len() {
                let opponent = sorted_teams[j];
                if !paired.contains(&opponent.id) {
                    best_opponent = Some(opponent);
                    break;
                }
            }
        }

        if let Some(opponent) = best_opponent {
            pairings.push((team.id.clone(), Some(opponent.id.clone())));
            paired.insert(team.id.clone());
            paired.insert(opponent.id.clone());
        }
    }

    // Handle BYE
    if needs_bye {
        for team in &sorted_teams {
            if !paired.contains(&team.id) {
                pairings.push((team.id.clone(), None));
                break;
            }
        }
    }

    // Shuffle pairings to randomize court assignment
    pairings.shuffle(&mut rng);

    Ok(pairings)
}

fn generate_round_robin_pairings(
    teams: &[Team],
    round_number: i32,
) -> Result<Vec<(String, Option<String>)>, String> {
    let n = teams.len();
    if n < 2 {
        return Err("Need at least 2 teams for round-robin".to_string());
    }

    // Use Berger tables / circle method
    let mut team_ids: Vec<String> = teams.iter().map(|t| t.id.clone()).collect();

    // Add a dummy for odd number of teams
    let has_bye = n % 2 == 1;
    if has_bye {
        team_ids.push("BYE".to_string());
    }

    let total = team_ids.len();
    let round_index = ((round_number - 1) as usize) % (total - 1);

    // Rotate teams (keep first fixed for circle method)
    let mut rotated = vec![team_ids[0].clone()];
    for i in 1..total {
        let idx = 1 + (i - 1 + round_index * (total - 1)) % (total - 1);
        if idx < total {
            rotated.push(team_ids[idx].clone());
        }
    }

    // Generate pairings
    let mut pairings = Vec::new();
    let half = total / 2;

    for i in 0..half {
        let t1 = rotated[i].clone();
        let t2 = rotated[total - 1 - i].clone();

        if t1 == "BYE" {
            pairings.push((t2, None));
        } else if t2 == "BYE" {
            pairings.push((t1, None));
        } else {
            pairings.push((t1, Some(t2)));
        }
    }

    Ok(pairings)
}

fn assign_courts(
    pairings: Vec<(String, Option<String>)>,
    number_of_courts: i32,
    court_history: &HashMap<String, Vec<i32>>,
) -> Result<Vec<(Option<String>, Option<String>)>, String> {
    let mut games: Vec<(Option<String>, Option<String>)> = Vec::new();

    for (i, (t1, t2)) in pairings.iter().enumerate() {
        let court = ((i as i32) % number_of_courts) + 1;

        // Try to avoid courts teams have used recently
        // This is a simplified version - could be optimized
        let t1_courts = court_history.get(t1).cloned().unwrap_or_default();
        let t2_courts = t2
            .as_ref()
            .and_then(|id| court_history.get(id))
            .cloned()
            .unwrap_or_default();

        // For now, just use sequential assignment with shuffle from Swiss
        games.push((Some(t1.clone()), t2.clone()));
    }

    Ok(games)
}

#[tauri::command]
pub fn update_game_score(
    db: State<Database>,
    game_id: String,
    team1_score: i32,
    team2_score: i32,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        "UPDATE qualifying_games SET team1_score = ?2, team2_score = ?3 WHERE id = ?1",
        params![game_id, team1_score, team2_score],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn complete_round(db: State<Database>, round_id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Get tournament ID
    let tournament_id: String = conn
        .query_row(
            "SELECT tournament_id FROM qualifying_rounds WHERE id = ?1",
            params![round_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Get all games for this round
    let mut stmt = conn
        .prepare(
            r#"
            SELECT team1_id, team2_id, team1_score, team2_score, is_bye
            FROM qualifying_games
            WHERE round_id = ?1
            "#,
        )
        .map_err(|e| e.to_string())?;

    let games: Vec<(Option<String>, Option<String>, Option<i32>, Option<i32>, bool)> = stmt
        .query_map(params![round_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get::<_, i32>(4)? != 0,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Update standings for each game
    for (team1_id, team2_id, team1_score, team2_score, is_bye) in games {
        if is_bye {
            // BYE: team gets a win with 13-7 score (FPUSA rules)
            if let Some(t1) = team1_id {
                conn.execute(
                    r#"
                    UPDATE team_standings SET
                        wins = wins + 1,
                        points_for = points_for + 13,
                        points_against = points_against + 7,
                        differential = differential + 6
                    WHERE tournament_id = ?1 AND team_id = ?2
                    "#,
                    params![tournament_id, t1],
                )
                .map_err(|e| e.to_string())?;
            }
        } else if let (Some(t1), Some(t2), Some(s1), Some(s2)) =
            (team1_id, team2_id, team1_score, team2_score)
        {
            // Update team 1
            let (t1_wins, t1_losses) = if s1 > s2 { (1, 0) } else { (0, 1) };
            conn.execute(
                r#"
                UPDATE team_standings SET
                    wins = wins + ?3,
                    losses = losses + ?4,
                    points_for = points_for + ?5,
                    points_against = points_against + ?6,
                    differential = differential + ?7
                WHERE tournament_id = ?1 AND team_id = ?2
                "#,
                params![tournament_id, t1, t1_wins, t1_losses, s1, s2, s1 - s2],
            )
            .map_err(|e| e.to_string())?;

            // Update team 2
            let (t2_wins, t2_losses) = if s2 > s1 { (1, 0) } else { (0, 1) };
            conn.execute(
                r#"
                UPDATE team_standings SET
                    wins = wins + ?3,
                    losses = losses + ?4,
                    points_for = points_for + ?5,
                    points_against = points_against + ?6,
                    differential = differential + ?7
                WHERE tournament_id = ?1 AND team_id = ?2
                "#,
                params![tournament_id, t2, t2_wins, t2_losses, s2, s1, s2 - s1],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    // Calculate Buchholz scores and update ranks
    calculate_buchholz_and_ranks(&conn, &tournament_id)?;

    // Mark round as complete
    conn.execute(
        "UPDATE qualifying_rounds SET is_complete = 1 WHERE id = ?1",
        params![round_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn calculate_buchholz_and_ranks(conn: &rusqlite::Connection, tournament_id: &str) -> Result<(), String> {
    // Get all standings
    let mut stmt = conn
        .prepare(
            r#"
            SELECT team_id, wins, differential, points_for
            FROM team_standings
            WHERE tournament_id = ?1
            "#,
        )
        .map_err(|e| e.to_string())?;

    let standings: Vec<(String, i32, i32, i32)> = stmt
        .query_map(params![tournament_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Get opponent wins for Buchholz calculation
    let mut buchholz_scores: HashMap<String, f64> = HashMap::new();

    for (team_id, _, _, _) in &standings {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT ts.wins
                FROM pairing_history ph
                JOIN team_standings ts ON (
                    (ph.team1_id = ?2 AND ts.team_id = ph.team2_id) OR
                    (ph.team2_id = ?2 AND ts.team_id = ph.team1_id)
                )
                WHERE ph.tournament_id = ?1 AND ts.tournament_id = ?1
                "#,
            )
            .map_err(|e| e.to_string())?;

        let opponent_wins: Vec<i32> = stmt
            .query_map(params![tournament_id, team_id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let buchholz: f64 = opponent_wins.iter().map(|&w| w as f64).sum();
        buchholz_scores.insert(team_id.clone(), buchholz);
    }

    // Update Buchholz scores
    for (team_id, score) in &buchholz_scores {
        conn.execute(
            "UPDATE team_standings SET buchholz_score = ?3 WHERE tournament_id = ?1 AND team_id = ?2",
            params![tournament_id, team_id, score],
        )
        .map_err(|e| e.to_string())?;
    }

    // Calculate ranks
    let mut ranked: Vec<(String, i32, i32, i32, f64)> = standings
        .iter()
        .map(|(id, wins, diff, pf)| {
            let buchholz = buchholz_scores.get(id).copied().unwrap_or(0.0);
            (id.clone(), *wins, *diff, *pf, buchholz)
        })
        .collect();

    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1) // wins
            .then(b.2.cmp(&a.2)) // differential
            .then(b.4.partial_cmp(&a.4).unwrap_or(std::cmp::Ordering::Equal)) // buchholz
            .then(b.3.cmp(&a.3)) // points for
    });

    for (rank, (team_id, _, _, _, _)) in ranked.iter().enumerate() {
        conn.execute(
            "UPDATE team_standings SET rank = ?3 WHERE tournament_id = ?1 AND team_id = ?2",
            params![tournament_id, team_id, (rank + 1) as i32],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}
