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
    generate_single_round(&conn, &tournament_id)
}

#[tauri::command]
pub fn generate_all_qualifying_rounds(
    db: State<Database>,
    tournament_id: String,
) -> Result<Vec<QualifyingRound>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Get tournament info including pairing method
    let (pairing_method, number_of_qualifying_rounds): (String, i32) = conn
        .query_row(
            "SELECT pairing_method, number_of_qualifying_rounds FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;

    // Swiss and Pool Play require round-by-round generation
    if pairing_method == "swiss" {
        return Err("Swiss system requires round-by-round generation. Use 'Generate Next Round' instead.".to_string());
    }
    if pairing_method == "poolPlay" {
        return Err("Pool Play requires round-by-round generation. Use 'Generate Next Round' instead.".to_string());
    }

    // Get current round number
    let current_round: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(round_number), 0) FROM qualifying_rounds WHERE tournament_id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Pool Play is fixed at 3 rounds max
    let max_rounds = if pairing_method == "poolPlay" {
        3
    } else {
        number_of_qualifying_rounds
    };

    // Generate all remaining rounds
    let mut rounds = Vec::new();
    for _ in current_round..max_rounds {
        let round = generate_single_round(&conn, &tournament_id)?;
        rounds.push(round);
    }

    if rounds.is_empty() {
        return Err("All qualifying rounds have already been generated".to_string());
    }

    Ok(rounds)
}

fn generate_single_round(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<QualifyingRound, String> {
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

    // Swiss and Pool Play: verify prior round is complete before generating next
    if (pairing_method == "swiss" || pairing_method == "poolPlay") && current_round > 0 {
        let prior_round_complete: bool = conn
            .query_row(
                "SELECT is_complete FROM qualifying_rounds WHERE tournament_id = ?1 AND round_number = ?2",
                params![tournament_id, current_round],
                |row| Ok(row.get::<_, i32>(0)? != 0),
            )
            .map_err(|e| e.to_string())?;

        if !prior_round_complete {
            return Err("Previous round must be completed before generating the next round.".to_string());
        }
    }

    // Pool Play: max 3 rounds
    if pairing_method == "poolPlay" && new_round_number > 3 {
        return Err("Pool Play format only has 3 rounds.".to_string());
    }

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
    let tournament_id_owned = tournament_id.to_string();
    let mut standings_stmt = conn
        .prepare(
            r#"
            SELECT team_id, wins, losses, points_for, points_against, differential, buchholz_score, fine_buchholz_score, point_quotient, is_eliminated
            FROM team_standings
            WHERE tournament_id = ?1
            ORDER BY wins DESC, buchholz_score DESC, fine_buchholz_score DESC, differential DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let standings: HashMap<String, TeamStanding> = standings_stmt
        .query_map(params![tournament_id], |row| {
            Ok(TeamStanding {
                id: String::new(),
                tournament_id: tournament_id_owned.clone(),
                team_id: row.get(0)?,
                wins: row.get(1)?,
                losses: row.get(2)?,
                points_for: row.get(3)?,
                points_against: row.get(4)?,
                differential: row.get(5)?,
                buchholz_score: row.get(6)?,
                fine_buchholz_score: row.get(7)?,
                point_quotient: row.get(8)?,
                is_eliminated: row.get::<_, i32>(9)? != 0,
                rank: 0,
            })
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .map(|s| (s.team_id.clone(), s))
        .collect();

    // Generate pairings based on method
    let pairings = match pairing_method.as_str() {
        "swiss" => generate_swiss_pairings(&teams, &standings, &pairing_history, region_avoidance)?,
        "swissHotel" => generate_swiss_hotel_pairings(&teams, &pairing_history, region_avoidance, new_round_number)?,
        "roundRobin" => generate_round_robin_pairings(&teams, new_round_number)?,
        "poolPlay" => generate_pool_play_round(&teams, &standings, &pairing_history, region_avoidance, new_round_number)?,
        _ => return Err(format!("Unknown pairing method: {}", pairing_method)),
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
        tournament_id: tournament_id.to_string(),
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

    // Helper to check if two teams are from the same region
    let same_region = |t1: &Team, t2: &Team| -> bool {
        if let (Some(r1), Some(r2)) = (&t1.region, &t2.region) {
            !r1.is_empty() && !r2.is_empty() && r1 == r2
        } else {
            false
        }
    };

    // Try to pair teams from similar score groups
    for i in 0..sorted_teams.len() {
        let team = sorted_teams[i];
        if paired.contains(&team.id) {
            continue;
        }

        // Find best opponent with graduated fallback:
        // Pass 1: Respect both pairing history AND region avoidance
        // Pass 2: Respect pairing history only (relax region avoidance)
        // Pass 3: Any unpaired opponent (relax all constraints)
        let mut best_opponent: Option<&Team> = None;

        // Pass 1: Full constraints (no repeat matchups, avoid same region)
        if region_avoidance {
            for j in (i + 1)..sorted_teams.len() {
                let opponent = sorted_teams[j];
                if paired.contains(&opponent.id) {
                    continue;
                }
                if pairing_history.contains(&(team.id.clone(), opponent.id.clone())) {
                    continue;
                }
                if same_region(team, opponent) {
                    continue;
                }
                best_opponent = Some(opponent);
                break;
            }
        }

        // Pass 2: Relax region avoidance, but still avoid repeat matchups
        if best_opponent.is_none() {
            for j in (i + 1)..sorted_teams.len() {
                let opponent = sorted_teams[j];
                if paired.contains(&opponent.id) {
                    continue;
                }
                if pairing_history.contains(&(team.id.clone(), opponent.id.clone())) {
                    continue;
                }
                best_opponent = Some(opponent);
                break;
            }
        }

        // Pass 3: Relax all constraints - just find any unpaired opponent
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

/// Swiss Hotel pairing: random pairing with graduated constraints (avoid repeats, region avoidance)
/// All rounds are pre-generated upfront.
fn generate_swiss_hotel_pairings(
    teams: &[Team],
    pairing_history: &HashSet<(String, String)>,
    region_avoidance: bool,
    _round_number: i32,
) -> Result<Vec<(String, Option<String>)>, String> {
    let mut rng = thread_rng();

    // Shuffle teams randomly
    let mut shuffled_teams: Vec<&Team> = teams.iter().collect();
    shuffled_teams.shuffle(&mut rng);

    let mut pairings: Vec<(String, Option<String>)> = Vec::new();
    let mut paired: HashSet<String> = HashSet::new();

    let needs_bye = shuffled_teams.len() % 2 == 1;

    // Helper to check if two teams are from the same region
    let same_region = |t1: &Team, t2: &Team| -> bool {
        if let (Some(r1), Some(r2)) = (&t1.region, &t2.region) {
            !r1.is_empty() && !r2.is_empty() && r1 == r2
        } else {
            false
        }
    };

    // Try to pair teams with graduated fallback
    for i in 0..shuffled_teams.len() {
        let team = shuffled_teams[i];
        if paired.contains(&team.id) {
            continue;
        }

        let mut best_opponent: Option<&Team> = None;

        // Pass 1: Full constraints (no repeat matchups, avoid same region)
        if region_avoidance {
            for j in (i + 1)..shuffled_teams.len() {
                let opponent = shuffled_teams[j];
                if paired.contains(&opponent.id) {
                    continue;
                }
                if pairing_history.contains(&(team.id.clone(), opponent.id.clone())) {
                    continue;
                }
                if same_region(team, opponent) {
                    continue;
                }
                best_opponent = Some(opponent);
                break;
            }
        }

        // Pass 2: Relax region avoidance, but still avoid repeat matchups
        if best_opponent.is_none() {
            for j in (i + 1)..shuffled_teams.len() {
                let opponent = shuffled_teams[j];
                if paired.contains(&opponent.id) {
                    continue;
                }
                if pairing_history.contains(&(team.id.clone(), opponent.id.clone())) {
                    continue;
                }
                best_opponent = Some(opponent);
                break;
            }
        }

        // Pass 3: Relax all constraints - just find any unpaired opponent
        if best_opponent.is_none() {
            for j in (i + 1)..shuffled_teams.len() {
                let opponent = shuffled_teams[j];
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
        for team in &shuffled_teams {
            if !paired.contains(&team.id) {
                pairings.push((team.id.clone(), None));
                break;
            }
        }
    }

    Ok(pairings)
}

/// Pool Play pairing: fixed 3-round format
/// Round 1: Random pairings
/// Round 2: Winners (1-0) play winners, losers (0-1) play losers
/// Round 3: Teams with 2 losses are eliminated; teams with 2 wins sit out; teams with 1 win (1-1) play each other
fn generate_pool_play_round(
    teams: &[Team],
    standings: &HashMap<String, TeamStanding>,
    pairing_history: &HashSet<(String, String)>,
    region_avoidance: bool,
    round_number: i32,
) -> Result<Vec<(String, Option<String>)>, String> {
    let mut rng = thread_rng();

    match round_number {
        1 => {
            // Round 1: Random pairings (same as Swiss Hotel round 1)
            generate_swiss_hotel_pairings(teams, pairing_history, region_avoidance, round_number)
        }
        2 => {
            // Round 2: Winners play winners, losers play losers
            let mut winners: Vec<&Team> = Vec::new();
            let mut losers: Vec<&Team> = Vec::new();

            for team in teams {
                if let Some(standing) = standings.get(&team.id) {
                    if standing.wins > standing.losses {
                        winners.push(team);
                    } else {
                        losers.push(team);
                    }
                } else {
                    // No standing yet, treat as 0-0 (shouldn't happen in round 2)
                    losers.push(team);
                }
            }

            winners.shuffle(&mut rng);
            losers.shuffle(&mut rng);

            let mut pairings: Vec<(String, Option<String>)> = Vec::new();

            // Pair winners
            pair_teams_with_constraints(&mut pairings, &winners, pairing_history, region_avoidance);

            // Pair losers
            pair_teams_with_constraints(&mut pairings, &losers, pairing_history, region_avoidance);

            // Handle any odd team out (give them a bye)
            let paired: HashSet<String> = pairings
                .iter()
                .flat_map(|(t1, t2)| {
                    let mut ids = vec![t1.clone()];
                    if let Some(t2_id) = t2 {
                        ids.push(t2_id.clone());
                    }
                    ids
                })
                .collect();

            for team in teams {
                if !paired.contains(&team.id) {
                    pairings.push((team.id.clone(), None));
                }
            }

            Ok(pairings)
        }
        3 => {
            // Round 3: Teams with 2 losses are eliminated
            // Teams with 2 wins sit out (already qualified)
            // Teams with 1-1 play each other
            let mut one_win_teams: Vec<&Team> = Vec::new();

            for team in teams {
                if let Some(standing) = standings.get(&team.id) {
                    // Only 1-1 teams play in round 3
                    if standing.wins == 1 && standing.losses == 1 {
                        one_win_teams.push(team);
                    }
                }
            }

            one_win_teams.shuffle(&mut rng);

            let mut pairings: Vec<(String, Option<String>)> = Vec::new();

            // Pair 1-1 teams
            pair_teams_with_constraints(&mut pairings, &one_win_teams, pairing_history, region_avoidance);

            // Handle odd team out
            let paired: HashSet<String> = pairings
                .iter()
                .flat_map(|(t1, t2)| {
                    let mut ids = vec![t1.clone()];
                    if let Some(t2_id) = t2 {
                        ids.push(t2_id.clone());
                    }
                    ids
                })
                .collect();

            for team in &one_win_teams {
                if !paired.contains(&team.id) {
                    pairings.push((team.id.clone(), None));
                }
            }

            Ok(pairings)
        }
        _ => Err("Pool Play format only has 3 rounds.".to_string()),
    }
}

/// Helper function to pair teams with graduated constraint relaxation
fn pair_teams_with_constraints(
    pairings: &mut Vec<(String, Option<String>)>,
    teams: &[&Team],
    pairing_history: &HashSet<(String, String)>,
    region_avoidance: bool,
) {
    let mut paired: HashSet<String> = HashSet::new();

    let same_region = |t1: &Team, t2: &Team| -> bool {
        if let (Some(r1), Some(r2)) = (&t1.region, &t2.region) {
            !r1.is_empty() && !r2.is_empty() && r1 == r2
        } else {
            false
        }
    };

    for i in 0..teams.len() {
        let team = teams[i];
        if paired.contains(&team.id) {
            continue;
        }

        let mut best_opponent: Option<&Team> = None;

        // Pass 1: Full constraints
        if region_avoidance {
            for j in (i + 1)..teams.len() {
                let opponent = teams[j];
                if paired.contains(&opponent.id) {
                    continue;
                }
                if pairing_history.contains(&(team.id.clone(), opponent.id.clone())) {
                    continue;
                }
                if same_region(team, opponent) {
                    continue;
                }
                best_opponent = Some(opponent);
                break;
            }
        }

        // Pass 2: Relax region avoidance
        if best_opponent.is_none() {
            for j in (i + 1)..teams.len() {
                let opponent = teams[j];
                if paired.contains(&opponent.id) {
                    continue;
                }
                if pairing_history.contains(&(team.id.clone(), opponent.id.clone())) {
                    continue;
                }
                best_opponent = Some(opponent);
                break;
            }
        }

        // Pass 3: Any unpaired opponent
        if best_opponent.is_none() {
            for j in (i + 1)..teams.len() {
                let opponent = teams[j];
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

    // Get tournament ID and pairing method
    let (tournament_id, pairing_method): (String, String) = conn
        .query_row(
            r#"
            SELECT qr.tournament_id, t.pairing_method
            FROM qualifying_rounds qr
            JOIN tournaments t ON qr.tournament_id = t.id
            WHERE qr.id = ?1
            "#,
            params![round_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
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

    // Get round number to check for Pool Play elimination
    let round_number: i32 = conn
        .query_row(
            "SELECT round_number FROM qualifying_rounds WHERE id = ?1",
            params![round_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Calculate rankings based on pairing method
    match pairing_method.as_str() {
        "swiss" => {
            // Swiss uses Buchholz tiebreaker
            calculate_buchholz_and_ranks(&conn, &tournament_id)?;
        }
        "swissHotel" | "roundRobin" | "poolPlay" => {
            // These use point quotient tiebreaker
            calculate_point_quotient_ranks(&conn, &tournament_id)?;
        }
        _ => {
            // Default to Buchholz
            calculate_buchholz_and_ranks(&conn, &tournament_id)?;
        }
    }

    // For Pool Play, mark teams with 2 losses as eliminated after round 3
    if pairing_method == "poolPlay" && round_number == 3 {
        conn.execute(
            r#"
            UPDATE team_standings
            SET is_eliminated = 1
            WHERE tournament_id = ?1 AND losses >= 2
            "#,
            params![tournament_id],
        )
        .map_err(|e| e.to_string())?;
    }

    // Mark round as complete
    conn.execute(
        "UPDATE qualifying_rounds SET is_complete = 1 WHERE id = ?1",
        params![round_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_all_qualifying_rounds(
    db: State<Database>,
    tournament_id: String,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Check if any rounds have scores entered
    let scored_games: i32 = conn
        .query_row(
            r#"
            SELECT COUNT(*) FROM qualifying_games g
            JOIN qualifying_rounds r ON g.round_id = r.id
            WHERE r.tournament_id = ?1 AND (g.team1_score IS NOT NULL OR g.team2_score IS NOT NULL)
            "#,
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if scored_games > 0 {
        return Err("Cannot delete qualifying rounds after scores have been entered.".to_string());
    }

    // Delete court history
    conn.execute(
        "DELETE FROM court_history WHERE tournament_id = ?1",
        params![tournament_id],
    )
    .map_err(|e| e.to_string())?;

    // Delete pairing history
    conn.execute(
        "DELETE FROM pairing_history WHERE tournament_id = ?1",
        params![tournament_id],
    )
    .map_err(|e| e.to_string())?;

    // Delete games (via cascade or explicit)
    conn.execute(
        r#"
        DELETE FROM qualifying_games WHERE round_id IN (
            SELECT id FROM qualifying_rounds WHERE tournament_id = ?1
        )
        "#,
        params![tournament_id],
    )
    .map_err(|e| e.to_string())?;

    // Delete rounds
    conn.execute(
        "DELETE FROM qualifying_rounds WHERE tournament_id = ?1",
        params![tournament_id],
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

    // Get opponents for each team (for Buchholz calculations)
    let mut team_opponents: HashMap<String, Vec<String>> = HashMap::new();

    for (team_id, _, _, _) in &standings {
        let mut stmt = conn
            .prepare(
                r#"
                SELECT CASE
                    WHEN ph.team1_id = ?2 THEN ph.team2_id
                    ELSE ph.team1_id
                END as opponent_id
                FROM pairing_history ph
                WHERE ph.tournament_id = ?1
                AND (ph.team1_id = ?2 OR ph.team2_id = ?2)
                "#,
            )
            .map_err(|e| e.to_string())?;

        let opponents: Vec<String> = stmt
            .query_map(params![tournament_id, team_id], |row| row.get(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        team_opponents.insert(team_id.clone(), opponents);
    }

    // Build a map of team_id -> wins for quick lookup
    let team_wins: HashMap<String, i32> = standings
        .iter()
        .map(|(id, wins, _, _)| (id.clone(), *wins))
        .collect();

    // Calculate Buchholz scores (sum of opponent wins)
    let mut buchholz_scores: HashMap<String, f64> = HashMap::new();

    for (team_id, _, _, _) in &standings {
        let opponents = team_opponents.get(team_id).cloned().unwrap_or_default();
        let buchholz: f64 = opponents
            .iter()
            .map(|opp_id| team_wins.get(opp_id).copied().unwrap_or(0) as f64)
            .sum();
        buchholz_scores.insert(team_id.clone(), buchholz);
    }

    // Calculate Fine Buchholz scores (sum of opponents' Buchholz scores)
    let mut fine_buchholz_scores: HashMap<String, f64> = HashMap::new();

    for (team_id, _, _, _) in &standings {
        let opponents = team_opponents.get(team_id).cloned().unwrap_or_default();
        let fine_buchholz: f64 = opponents
            .iter()
            .map(|opp_id| buchholz_scores.get(opp_id).copied().unwrap_or(0.0))
            .sum();
        fine_buchholz_scores.insert(team_id.clone(), fine_buchholz);
    }

    // Update Buchholz and Fine Buchholz scores in database
    for (team_id, buchholz) in &buchholz_scores {
        let fine_buchholz = fine_buchholz_scores.get(team_id).copied().unwrap_or(0.0);
        conn.execute(
            "UPDATE team_standings SET buchholz_score = ?3, fine_buchholz_score = ?4 WHERE tournament_id = ?1 AND team_id = ?2",
            params![tournament_id, team_id, buchholz, fine_buchholz],
        )
        .map_err(|e| e.to_string())?;
    }

    // Calculate ranks with tiebreaker order: wins → buchholz → fine_buchholz → differential → random
    // Generate random tiebreaker values for each team
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_tiebreakers: HashMap<String, u64> = standings
        .iter()
        .map(|(id, _, _, _)| (id.clone(), rng.gen()))
        .collect();

    let mut ranked: Vec<(String, i32, f64, f64, i32, u64)> = standings
        .iter()
        .map(|(id, wins, diff, _)| {
            let buchholz = buchholz_scores.get(id).copied().unwrap_or(0.0);
            let fine_buchholz = fine_buchholz_scores.get(id).copied().unwrap_or(0.0);
            let random_tb = random_tiebreakers.get(id).copied().unwrap_or(0);
            (id.clone(), *wins, buchholz, fine_buchholz, *diff, random_tb)
        })
        .collect();

    // Sort by: wins DESC → buchholz DESC → fine_buchholz DESC → differential DESC → random
    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1) // wins (descending)
            .then(b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal)) // buchholz (descending)
            .then(b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal)) // fine_buchholz (descending)
            .then(b.4.cmp(&a.4)) // differential (descending)
            .then(b.5.cmp(&a.5)) // random tiebreaker (descending)
    });

    for (rank, (team_id, _, _, _, _, _)) in ranked.iter().enumerate() {
        conn.execute(
            "UPDATE team_standings SET rank = ?3 WHERE tournament_id = ?1 AND team_id = ?2",
            params![tournament_id, team_id, (rank + 1) as i32],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Calculate ranks using point quotient tiebreaker (for Swiss Hotel, Round Robin, Pool Play)
/// Tiebreaker order: wins → differential → point_quotient → random
fn calculate_point_quotient_ranks(conn: &rusqlite::Connection, tournament_id: &str) -> Result<(), String> {
    // Get all standings
    let mut stmt = conn
        .prepare(
            r#"
            SELECT team_id, wins, differential, points_for, points_against
            FROM team_standings
            WHERE tournament_id = ?1
            "#,
        )
        .map_err(|e| e.to_string())?;

    let standings: Vec<(String, i32, i32, i32, i32)> = stmt
        .query_map(params![tournament_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Calculate and update point quotient for each team
    for (team_id, _, _, points_for, points_against) in &standings {
        let point_quotient = if *points_against > 0 {
            *points_for as f64 / *points_against as f64
        } else if *points_for > 0 {
            f64::MAX // Infinite quotient if no points against but some points for
        } else {
            1.0 // Default to 1.0 if no games played
        };

        conn.execute(
            "UPDATE team_standings SET point_quotient = ?3 WHERE tournament_id = ?1 AND team_id = ?2",
            params![tournament_id, team_id, point_quotient],
        )
        .map_err(|e| e.to_string())?;
    }

    // Generate random tiebreaker values for each team
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let random_tiebreakers: HashMap<String, u64> = standings
        .iter()
        .map(|(id, _, _, _, _)| (id.clone(), rng.gen()))
        .collect();

    // Build ranked list with point quotient
    let mut ranked: Vec<(String, i32, i32, f64, u64)> = standings
        .iter()
        .map(|(id, wins, diff, points_for, points_against)| {
            let point_quotient = if *points_against > 0 {
                *points_for as f64 / *points_against as f64
            } else if *points_for > 0 {
                f64::MAX
            } else {
                1.0
            };
            let random_tb = random_tiebreakers.get(id).copied().unwrap_or(0);
            (id.clone(), *wins, *diff, point_quotient, random_tb)
        })
        .collect();

    // Sort by: wins DESC → differential DESC → point_quotient DESC → random
    ranked.sort_by(|a, b| {
        b.1.cmp(&a.1) // wins (descending)
            .then(b.2.cmp(&a.2)) // differential (descending)
            .then(b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal)) // point_quotient (descending)
            .then(b.4.cmp(&a.4)) // random tiebreaker (descending)
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
