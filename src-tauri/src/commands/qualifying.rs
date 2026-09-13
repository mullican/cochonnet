use crate::db::Database;
use crate::models::{GameWithTeams, PanacheSide, QualifyingGame, QualifyingRound, Team, TeamStanding};
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
            SELECT id, tournament_id, round_number, is_complete, is_final, created_at
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
                is_final: row.get::<_, i32>(4)? != 0,
                created_at: row.get(5)?,
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
            SELECT id, round_id, court_number, team1_id, team2_id, team1_score, team2_score, is_bye,
                   side1_id, side2_id
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
                side1_id: row.get(8)?,
                side2_id: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    // Panache games reference temporary teams rather than registered teams. Load
    // every side for the round in one pass instead of extending the per-game
    // lookups below.
    let sides = load_panache_sides(&conn, &round_id)?;

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

        let side1 = game
            .side1_id
            .as_ref()
            .and_then(|id| sides.get(id).cloned());
        let side2 = game
            .side2_id
            .as_ref()
            .and_then(|id| sides.get(id).cloned());

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
            side1,
            side2,
        });
    }

    Ok(games_with_teams)
}

/// Loads every panache temporary team in a round, keyed by side id.
///
/// Returns an empty map for the team formats, whose games carry no side ids.
fn load_panache_sides(
    conn: &rusqlite::Connection,
    round_id: &str,
) -> Result<HashMap<String, PanacheSide>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT pt.id, pt.team_index,
                   t.id, t.tournament_id, t.team_number, t.captain, t.player2, t.player3,
                   t.region, t.club, t.is_champion, t.created_at
            FROM panache_teams pt
            JOIN panache_team_members ptm ON ptm.panache_team_id = pt.id
            JOIN teams t ON t.id = ptm.team_id
            WHERE pt.round_id = ?1
            ORDER BY pt.team_index ASC, ptm.position ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows = stmt
        .query_map(params![round_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i32>(1)?,
                Team {
                    id: row.get(2)?,
                    tournament_id: row.get(3)?,
                    team_number: row.get(4)?,
                    captain: row.get(5)?,
                    player2: row.get(6)?,
                    player3: row.get(7)?,
                    region: row.get(8)?,
                    club: row.get(9)?,
                    is_champion: row.get::<_, i32>(10)? != 0,
                    created_at: row.get(11)?,
                },
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut sides: HashMap<String, PanacheSide> = HashMap::new();
    for (side_id, team_index, member) in rows {
        sides
            .entry(side_id.clone())
            .or_insert_with(|| PanacheSide {
                id: side_id,
                team_index,
                members: Vec::new(),
            })
            .members
            .push(member);
    }

    Ok(sides)
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
    if pairing_method == "panache" {
        return Err("Panaché draws its whole schedule at once. Use 'Generate All Rounds' for Panaché instead.".to_string());
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
    let mut max_rounds = if pairing_method == "poolPlay" {
        3
    } else {
        number_of_qualifying_rounds
    };

    // A round robin is finished once everyone has met everyone: that is
    // n - 1 rounds for an even field, n for an odd one (the extra round is
    // where the last team takes its bye). Asking for more can only repeat
    // pairings, so cap it and report how many rounds actually exist.
    if pairing_method == "roundRobin" {
        let team_count: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM teams WHERE tournament_id = ?1",
                params![tournament_id],
                |row| row.get(0),
            )
            .map_err(|e| e.to_string())?;

        if team_count >= 2 {
            let full_cycle = if team_count % 2 == 0 { team_count - 1 } else { team_count };
            max_rounds = max_rounds.min(full_cycle);
        }
    }

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
    let (pairing_method, region_avoidance): (String, bool) = conn
        .query_row(
            "SELECT pairing_method, region_avoidance FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| Ok((row.get(0)?, row.get::<_, i32>(1)? != 0)),
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
            SELECT id, tournament_id, team_number, captain, player2, player3, region, club, is_champion, created_at
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
                team_number: row.get(2)?,
                captain: row.get(3)?,
                player2: row.get(4)?,
                player3: row.get(5)?,
                region: row.get(6)?,
                club: row.get(7)?,
                is_champion: row.get::<_, i32>(8)? != 0,
                created_at: row.get(9)?,
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

    // Which teams have already sat out a round. A bye is scored as a win, so
    // it has to rotate; without this the same team can draw several.
    let mut bye_stmt = conn
        .prepare(
            r#"
            SELECT g.team1_id
            FROM qualifying_games g
            JOIN qualifying_rounds r ON g.round_id = r.id
            WHERE r.tournament_id = ?1 AND g.is_bye = 1 AND g.team1_id IS NOT NULL
            "#,
        )
        .map_err(|e| e.to_string())?;

    let bye_history: HashSet<String> = bye_stmt
        .query_map(params![tournament_id], |row| row.get::<_, String>(0))
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

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
        "swiss" => generate_swiss_pairings(&teams, &standings, &pairing_history, &bye_history, region_avoidance)?,
        "swissHotel" => generate_swiss_hotel_pairings(&teams, &pairing_history, &bye_history, region_avoidance, new_round_number)?,
        "roundRobin" => generate_round_robin_pairings(&teams, new_round_number)?,
        "poolPlay" => generate_pool_play_round(&teams, &standings, &pairing_history, &bye_history, region_avoidance, new_round_number)?,
        "panache" => {
            return Err(
                "Panaché rounds are drawn by the Panaché scheduler, not round-by-round pairing."
                    .to_string(),
            )
        }
        _ => return Err(format!("Unknown pairing method: {}", pairing_method)),
    };

    // Assign courts with rotation
    let games = assign_courts(pairings)?;

    // Create the round
    let round_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO qualifying_rounds (id, tournament_id, round_number, is_complete, is_final, created_at)
        VALUES (?1, ?2, ?3, 0, 0, ?4)
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
        is_final: false,
        created_at: now,
    })
}

/// Finds a pairing of `order` in which no pair is `forbidden`, preferring
/// opponents that appear early in `order`.
///
/// The callers used to take the first feasible opponent for each team in turn
/// and commit to it. That is fast but incomplete: a greedy choice can consume
/// the only legal partner some later team had, forcing a repeat matchup even
/// when a repeat-free pairing existed. With four level teams who have only
/// ever played C vs D, greedy pairs A-B and then has nothing left for C but D,
/// while A-C plus B-D would have been clean.
///
/// So this searches instead of guessing: depth-first over the first unpaired
/// team's candidates, in `order`, backtracking when a branch strands someone.
/// Taking candidates in order means the first complete pairing found is also
/// the one that keeps teams closest to their own score group, which is what
/// Swiss wants.
///
/// `steps` bounds the work. Real fields settle in well under the budget; a
/// pathological one gives up and lets the caller fall back rather than hang
/// the app mid-tournament.
fn find_pairing_avoiding<F>(
    order: &[&Team],
    forbidden: F,
    budget: u32,
) -> Option<Vec<(String, String)>>
where
    F: Fn(&Team, &Team) -> bool,
{
    // An odd field cannot pair everyone; the caller draws a bye first and
    // passes the rest, so anything odd here is a caller bug, not a draw.
    if order.len() % 2 != 0 {
        return None;
    }

    let n = order.len();
    let mut taken = vec![false; n];
    let mut acc: Vec<(String, String)> = Vec::with_capacity(n / 2);
    let mut steps: u32 = 0;

    fn search<F>(
        order: &[&Team],
        taken: &mut Vec<bool>,
        acc: &mut Vec<(String, String)>,
        steps: &mut u32,
        budget: u32,
        forbidden: &F,
    ) -> bool
    where
        F: Fn(&Team, &Team) -> bool,
    {
        let Some(i) = (0..order.len()).find(|&i| !taken[i]) else {
            return true; // everyone is paired
        };

        for j in (i + 1)..order.len() {
            if taken[j] || forbidden(order[i], order[j]) {
                continue;
            }

            *steps += 1;
            if *steps > budget {
                return false;
            }

            taken[i] = true;
            taken[j] = true;
            acc.push((order[i].id.clone(), order[j].id.clone()));

            if search(order, taken, acc, steps, budget, forbidden) {
                return true;
            }

            acc.pop();
            taken[i] = false;
            taken[j] = false;
        }

        false
    }

    if search(order, &mut taken, &mut acc, &mut steps, budget, &forbidden) {
        Some(acc)
    } else {
        None
    }
}

/// How many steps `find_pairing_avoiding` may take before giving up.
const PAIRING_BUDGET: u32 = 50_000;

/// Pairs `order` under the graduated constraints every format here shares:
/// avoid rematches and (optionally) same-region meetings, relaxing region
/// first and rematches only as a last resort. Each pass is a complete search,
/// so a constraint is only relaxed when no pairing satisfying it exists.
fn pair_with_graduated_constraints(
    order: &[&Team],
    pairing_history: &HashSet<(String, String)>,
    region_avoidance: bool,
) -> Vec<(String, Option<String>)> {
    let played = |a: &Team, b: &Team| pairing_history.contains(&(a.id.clone(), b.id.clone()));
    let same_region = |a: &Team, b: &Team| match (&a.region, &b.region) {
        (Some(r1), Some(r2)) => !r1.is_empty() && !r2.is_empty() && r1 == r2,
        _ => false,
    };

    let found = if region_avoidance {
        find_pairing_avoiding(order, |a, b| played(a, b) || same_region(a, b), PAIRING_BUDGET)
            .or_else(|| find_pairing_avoiding(order, |a, b| played(a, b), PAIRING_BUDGET))
    } else {
        find_pairing_avoiding(order, |a, b| played(a, b), PAIRING_BUDGET)
    };

    // Last resort: a repeat is unavoidable (or the search ran out of budget),
    // so pair in order and accept it.
    let pairs = found.unwrap_or_else(|| {
        let mut taken = vec![false; order.len()];
        let mut out = Vec::new();
        for i in 0..order.len() {
            if taken[i] {
                continue;
            }
            if let Some(j) = ((i + 1)..order.len()).find(|&j| !taken[j]) {
                taken[i] = true;
                taken[j] = true;
                out.push((order[i].id.clone(), order[j].id.clone()));
            }
        }
        out
    });

    pairs.into_iter().map(|(a, b)| (a, Some(b))).collect()
}

/// Picks the team to sit out an odd round: the lowest placed who has not had
/// one yet, so byes rotate instead of landing on the same team every round.
///
/// Without this the bye fell to whoever the pairing loop happened to leave
/// over, which in a 7-team field was the same team three rounds running - and
/// a bye is scored as a 13-7 win, so that was three free wins.
fn select_bye_team<'a>(order: &[&'a Team], bye_history: &HashSet<String>) -> Option<&'a Team> {
    order
        .iter()
        .rev()
        .find(|t| !bye_history.contains(&t.id))
        .or_else(|| order.last())
        .copied()
}

fn generate_swiss_pairings(
    teams: &[Team],
    standings: &HashMap<String, TeamStanding>,
    pairing_history: &HashSet<(String, String)>,
    bye_history: &HashSet<String>,
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

    // Draw the bye first so the rest is an even field to pair. Picking it up
    // front also means the bye follows the rotation rule rather than falling
    // to whichever team the pairing happened to strand.
    if sorted_teams.len() % 2 == 1 {
        if let Some(bye_team) = select_bye_team(&sorted_teams, bye_history) {
            pairings.push((bye_team.id.clone(), None));
            sorted_teams.retain(|t| t.id != bye_team.id);
        }
    }

    pairings.extend(pair_with_graduated_constraints(
        &sorted_teams,
        pairing_history,
        region_avoidance,
    ));

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
    // The circle method fixes one team and rotates the rest by one seat per
    // round, so after total - 1 rounds everyone has met everyone exactly once.
    //
    // The offset here is the round index itself. It used to be
    // `round_index * (total - 1)`, which is always a multiple of the modulus
    // and so always rotated by zero - every round produced the identical set
    // of games.
    let round_index = ((round_number - 1) as usize) % (total - 1);

    // Rotate teams (keep first fixed for circle method)
    let mut rotated = vec![team_ids[0].clone()];
    for i in 1..total {
        let idx = 1 + (i - 1 + round_index) % (total - 1);
        rotated.push(team_ids[idx].clone());
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
    bye_history: &HashSet<String>,
    region_avoidance: bool,
    _round_number: i32,
) -> Result<Vec<(String, Option<String>)>, String> {
    let mut rng = thread_rng();

    // Shuffle teams randomly
    let mut shuffled_teams: Vec<&Team> = teams.iter().collect();
    shuffled_teams.shuffle(&mut rng);

    let mut pairings: Vec<(String, Option<String>)> = Vec::new();

    if shuffled_teams.len() % 2 == 1 {
        if let Some(bye_team) = select_bye_team(&shuffled_teams, bye_history) {
            pairings.push((bye_team.id.clone(), None));
            shuffled_teams.retain(|t| t.id != bye_team.id);
        }
    }

    pairings.extend(pair_with_graduated_constraints(
        &shuffled_teams,
        pairing_history,
        region_avoidance,
    ));

    pairings.shuffle(&mut rng);

    Ok(pairings)
}

fn generate_pool_play_round(
    teams: &[Team],
    standings: &HashMap<String, TeamStanding>,
    pairing_history: &HashSet<(String, String)>,
    bye_history: &HashSet<String>,
    region_avoidance: bool,
    round_number: i32,
) -> Result<Vec<(String, Option<String>)>, String> {
    let mut rng = thread_rng();

    match round_number {
        1 => {
            // Round 1: Random pairings (same as Swiss Hotel round 1)
            generate_swiss_hotel_pairings(teams, pairing_history, bye_history, region_avoidance, round_number)
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
    // Pool play hands this an already-grouped slice (all the winners, or all
    // the losers). An odd group leaves one team over; it plays nobody this
    // round rather than being forced into a repeat, which is what the caller
    // expects when a pool does not divide evenly.
    let mut group: Vec<&Team> = teams.to_vec();
    if group.len() % 2 == 1 {
        group.pop();
    }

    pairings.extend(pair_with_graduated_constraints(
        &group,
        pairing_history,
        region_avoidance,
    ));
}

fn assign_courts(
    pairings: Vec<(String, Option<String>)>,
) -> Result<Vec<(Option<String>, Option<String>)>, String> {
    // Court numbers are assigned sequentially by the caller, based on each
    // game's position in this list.
    let games = pairings
        .into_iter()
        .map(|(t1, t2)| (Some(t1), t2))
        .collect();

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
    let (tournament_id, pairing_method, is_final): (String, String, bool) = conn
        .query_row(
            r#"
            SELECT qr.tournament_id, t.pairing_method, qr.is_final
            FROM qualifying_rounds qr
            JOIN tournaments t ON qr.tournament_id = t.id
            WHERE qr.id = ?1
            "#,
            params![round_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get::<_, i32>(2)? != 0)),
        )
        .map_err(|e| e.to_string())?;

    // The panache final names the champions; it does not reopen the qualifying
    // standings, the same way bracket results don't feed back into them.
    if is_final {
        conn.execute(
            "UPDATE qualifying_rounds SET is_complete = 1 WHERE id = ?1",
            params![round_id],
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Get all games for this round
    let mut stmt = conn
        .prepare(
            r#"
            SELECT team1_id, team2_id, team1_score, team2_score, is_bye, side1_id, side2_id
            FROM qualifying_games
            WHERE round_id = ?1
            "#,
        )
        .map_err(|e| e.to_string())?;

    let games: Vec<(
        Option<String>,
        Option<String>,
        Option<i32>,
        Option<i32>,
        bool,
        Option<String>,
        Option<String>,
    )> = stmt
        .query_map(params![round_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get::<_, i32>(4)? != 0,
                row.get(5)?,
                row.get(6)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .filter_map(|r| r.ok())
        .collect();

    // Update standings for each game
    for (team1_id, team2_id, team1_score, team2_score, is_bye, side1_id, side2_id) in games {
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
        } else if let (Some(s1), Some(s2)) = (team1_score, team2_score) {
            // Panache: the temporary team's result accrues to each member
            // individually. Everything else scores a single team per side.
            if let (Some(side1), Some(side2)) = (&side1_id, &side2_id) {
                let members1 = load_side_member_ids(&conn, side1)?;
                let members2 = load_side_member_ids(&conn, side2)?;
                apply_game_result(&conn, &tournament_id, &members1, s1, s2)?;
                apply_game_result(&conn, &tournament_id, &members2, s2, s1)?;
            } else if let (Some(t1), Some(t2)) = (&team1_id, &team2_id) {
                apply_game_result(&conn, &tournament_id, std::slice::from_ref(t1), s1, s2)?;
                apply_game_result(&conn, &tournament_id, std::slice::from_ref(t2), s2, s1)?;
            }
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
        "swissHotel" | "roundRobin" | "poolPlay" | "panache" => {
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

    // Delete panache sit-outs and temporary teams. These cascade from
    // qualifying_rounds, but are removed explicitly like the histories above.
    conn.execute(
        "DELETE FROM panache_sitouts WHERE tournament_id = ?1",
        params![tournament_id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        r#"
        DELETE FROM panache_team_members WHERE panache_team_id IN (
            SELECT id FROM panache_teams WHERE tournament_id = ?1
        )
        "#,
        params![tournament_id],
    )
    .map_err(|e| e.to_string())?;

    conn.execute(
        "DELETE FROM panache_teams WHERE tournament_id = ?1",
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

/// Adds one game result to every listed competitor's standing row.
///
/// For the team formats that is a single team; for panache it is each member of
/// the temporary team, which is how a shared score becomes an individual record.
/// A tie counts as a loss for both sides, as it always has here.
pub(crate) fn apply_game_result(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    competitor_ids: &[String],
    own_score: i32,
    opponent_score: i32,
) -> Result<(), String> {
    let (wins, losses) = if own_score > opponent_score {
        (1, 0)
    } else {
        (0, 1)
    };

    for competitor_id in competitor_ids {
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
            params![
                tournament_id,
                competitor_id,
                wins,
                losses,
                own_score,
                opponent_score,
                own_score - opponent_score
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// The individuals making up one panache temporary team.
pub(crate) fn load_side_member_ids(
    conn: &rusqlite::Connection,
    side_id: &str,
) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT team_id FROM panache_team_members WHERE panache_team_id = ?1 ORDER BY position ASC",
        )
        .map_err(|e| e.to_string())?;

    let ids = stmt
        .query_map(params![side_id], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(ids)
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
pub(crate) fn calculate_point_quotient_ranks(conn: &rusqlite::Connection, tournament_id: &str) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn team(id: &str, region: Option<&str>) -> Team {
        Team {
            id: id.to_string(),
            tournament_id: "t".into(),
            team_number: id[1..].parse().unwrap_or(0),
            captain: id.to_string(),
            player2: String::new(),
            player3: None,
            region: region.map(|r| r.to_string()),
            club: None,
            is_champion: false,
            created_at: "now".into(),
        }
    }

    fn teams(n: usize) -> Vec<Team> {
        (1..=n).map(|i| team(&format!("T{}", i), None)).collect()
    }

    fn standing(id: &str, wins: i32) -> TeamStanding {
        TeamStanding {
            id: String::new(),
            tournament_id: "t".into(),
            team_id: id.into(),
            wins,
            losses: 0,
            points_for: 13 * wins,
            points_against: 0,
            differential: wins * 3,
            buchholz_score: 0.0,
            fine_buchholz_score: 0.0,
            point_quotient: 0.0,
            is_eliminated: false,
            rank: 0,
        }
    }

    fn history(pairs: &[(&str, &str)]) -> HashSet<(String, String)> {
        let mut h = HashSet::new();
        for (a, b) in pairs {
            h.insert((a.to_string(), b.to_string()));
            h.insert((b.to_string(), a.to_string()));
        }
        h
    }

    fn played_again(pairs: &[(String, Option<String>)], hist: &HashSet<(String, String)>) -> usize {
        pairs
            .iter()
            .filter_map(|(a, b)| b.as_ref().map(|b| (a, b)))
            .filter(|(a, b)| hist.contains(&((*a).clone(), (*b).clone())))
            .count()
    }

    /// Greedy first-fit pairs A-B and then has only D left for C. Searching
    /// finds A-C plus B-D, which repeats nothing.
    #[test]
    fn avoidable_rematch_is_avoided() {
        let t = teams(4);
        let hist = history(&[("T3", "T4")]);
        let standings: HashMap<String, TeamStanding> = HashMap::new();

        let pairs = generate_swiss_pairings(&t, &standings, &hist, &HashSet::new(), false).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(played_again(&pairs, &hist), 0, "avoidable rematch: {:?}", pairs);
    }

    /// Across a normal round count nobody should meet twice, whatever way the
    /// results fall. Run many result sequences, not one lucky seed.
    ///
    /// Note the ceiling this stops short of. Swiss pairs one round at a time
    /// from the standings so far, so it cannot look ahead: a pairing that is
    /// perfectly legal now can leave a later round with no repeat-free option.
    /// Measured over 300 result sequences, 8 teams stay clean through 5 rounds
    /// but not 6, and 16 teams stay clean through 8. Scheduling every round up
    /// front is what Round Robin is for; past roughly n - 3 rounds, use it.
    #[test]
    fn a_normal_round_count_never_repeats() {
        for (n, rounds) in [(8usize, 5usize), (16, 6), (12, 6), (10, 5), (32, 5)] {
            let t = teams(n);
            for seed in 0..40u64 {
                let mut hist = HashSet::new();
                let mut byes: HashSet<String> = HashSet::new();
                let mut wins: HashMap<String, i32> = t.iter().map(|x| (x.id.clone(), 0)).collect();
                let mut state = seed.wrapping_mul(7919).wrapping_add(13);

                for round in 1..=rounds {
                    let standings: HashMap<String, TeamStanding> =
                        t.iter().map(|x| (x.id.clone(), standing(&x.id, wins[&x.id]))).collect();
                    let pairs =
                        generate_swiss_pairings(&t, &standings, &hist, &byes, false).unwrap();
                    assert_eq!(
                        played_again(&pairs, &hist),
                        0,
                        "{} teams, seed {}, round {} repeated a matchup",
                        n, seed, round
                    );
                    for (a, b) in &pairs {
                        match b {
                            Some(b) => {
                                hist.insert((a.clone(), b.clone()));
                                hist.insert((b.clone(), a.clone()));
                                state = state
                                    .wrapping_mul(6364136223846793005)
                                    .wrapping_add(1442695040888963407);
                                if (state >> 33) % 2 == 0 {
                                    *wins.get_mut(a).unwrap() += 1;
                                } else {
                                    *wins.get_mut(b).unwrap() += 1;
                                }
                            }
                            None => {
                                byes.insert(a.clone());
                            }
                        }
                    }
                }
            }
        }
    }

    /// A bye is scored as a win, so it must rotate. Seven teams over five
    /// rounds should never hand the same team two.
    #[test]
    fn byes_rotate_instead_of_landing_on_one_team() {
        let t = teams(7);
        let mut hist = HashSet::new();
        let mut byes: HashSet<String> = HashSet::new();
        let mut counts: HashMap<String, i32> = HashMap::new();

        for _ in 1..=5 {
            let standings: HashMap<String, TeamStanding> =
                t.iter().map(|x| (x.id.clone(), standing(&x.id, 0))).collect();
            let pairs =
                generate_swiss_pairings(&t, &standings, &hist, &byes, false).unwrap();

            let bye: Vec<&String> = pairs.iter().filter(|(_, b)| b.is_none()).map(|(a, _)| a).collect();
            assert_eq!(bye.len(), 1, "odd field should draw exactly one bye");
            byes.insert(bye[0].clone());
            *counts.entry(bye[0].clone()).or_insert(0) += 1;

            for (a, b) in &pairs {
                if let Some(b) = b {
                    hist.insert((a.clone(), b.clone()));
                    hist.insert((b.clone(), a.clone()));
                }
            }
        }

        assert_eq!(counts.len(), 5, "byes did not rotate: {:?}", counts);
        assert!(counts.values().all(|&c| c == 1), "a team drew two byes: {:?}", counts);
    }

    /// The circle method must actually rotate. The offset was multiplied by
    /// the modulus, so every round produced the identical set of games.
    #[test]
    fn round_robin_gives_every_team_every_opponent() {
        let t = teams(6);
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut rounds_seen: HashSet<String> = HashSet::new();

        for round in 1..=5 {
            let pairs = generate_round_robin_pairings(&t, round).unwrap();
            let mut sig: Vec<String> =
                pairs.iter().map(|(a, b)| format!("{}v{:?}", a, b)).collect();
            sig.sort();
            rounds_seen.insert(sig.join("|"));

            for (a, b) in &pairs {
                if let Some(b) = b {
                    let key = if a < b { (a.clone(), b.clone()) } else { (b.clone(), a.clone()) };
                    assert!(seen.insert(key), "{} played {} twice", a, b);
                }
            }
        }

        assert_eq!(rounds_seen.len(), 5, "rounds were not distinct");
        // 6 teams, 5 rounds, 3 games each = every one of the 15 pairs exactly once.
        assert_eq!(seen.len(), 15, "not a complete round robin");
    }

    /// An odd field rotates the bye through every team.
    #[test]
    fn round_robin_rotates_the_bye_when_odd() {
        let t = teams(5);
        let mut byes = Vec::new();
        for round in 1..=5 {
            let pairs = generate_round_robin_pairings(&t, round).unwrap();
            let bye: Vec<&String> = pairs.iter().filter(|(_, b)| b.is_none()).map(|(a, _)| a).collect();
            assert_eq!(bye.len(), 1, "round {} should have one bye", round);
            byes.push(bye[0].clone());
        }
        let distinct: HashSet<&String> = byes.iter().collect();
        assert_eq!(distinct.len(), 5, "bye did not rotate: {:?}", byes);
    }

    /// Region avoidance may be relaxed, but only when it genuinely cannot be
    /// satisfied - and never at the cost of a rematch that was avoidable.
    #[test]
    fn region_is_relaxed_before_rematches_are() {
        // Four teams, two regions, already played across regions. Keeping
        // regions apart now would force a rematch, so regions give way.
        let t = vec![
            team("T1", Some("N")),
            team("T2", Some("S")),
            team("T3", Some("N")),
            team("T4", Some("S")),
        ];
        let hist = history(&[("T1", "T2"), ("T3", "T4")]);
        let standings: HashMap<String, TeamStanding> = HashMap::new();

        let pairs = generate_swiss_pairings(&t, &standings, &hist, &HashSet::new(), true).unwrap();
        assert_eq!(played_again(&pairs, &hist), 0, "relaxed the wrong constraint: {:?}", pairs);
    }

    /// The searcher must not hang a tournament on a large field.
    #[test]
    fn large_field_pairs_quickly() {
        let t = teams(128);
        let standings: HashMap<String, TeamStanding> = HashMap::new();
        let start = std::time::Instant::now();
        let pairs =
            generate_swiss_pairings(&t, &standings, &HashSet::new(), &HashSet::new(), false).unwrap();
        assert_eq!(pairs.len(), 64);
        assert!(start.elapsed().as_millis() < 500, "took {:?}", start.elapsed());
    }

    /// Everyone having played everyone is the one case where a repeat is
    /// unavoidable. It must still return a full set of games, not give up.
    #[test]
    fn exhausted_field_still_pairs() {
        let t = teams(4);
        let hist = history(&[("T1", "T2"), ("T1", "T3"), ("T1", "T4"), ("T2", "T3"), ("T2", "T4"), ("T3", "T4")]);
        let standings: HashMap<String, TeamStanding> = HashMap::new();
        let pairs = generate_swiss_pairings(&t, &standings, &hist, &HashSet::new(), false).unwrap();
        assert_eq!(pairs.len(), 2, "should still schedule both games");
    }

    /// Diagnostic, not an assertion: prints the repeat rate and worst bye
    /// count per field size. This is what exposed the greedy pairer, and what
    /// located the round-count ceiling documented on
    /// `a_normal_round_count_never_repeats`. Run with:
    ///   cargo test --lib measure_rematch_rate -- --ignored --nocapture
    #[test]
    #[ignore]
    fn measure_rematch_rate() {
        for (n, r) in [(128usize, 5usize), (32, 5), (16, 6), (16, 8), (12, 6), (10, 5), (8, 5), (8, 6), (8, 7), (6, 5)] {
            let t = teams(n);
            let mut total_re = 0;
            let mut total_g = 0;
            let mut worst_byes = 0;
            for seed in 0..200u64 {
                let mut hist = HashSet::new();
                let mut byes: HashSet<String> = HashSet::new();
                let mut bye_counts: HashMap<String, i32> = HashMap::new();
                let mut wins: HashMap<String, i32> = t.iter().map(|x| (x.id.clone(), 0)).collect();
                let mut state = seed * 7919 + 13;
                for _ in 0..r {
                    let st: HashMap<String, TeamStanding> =
                        t.iter().map(|x| (x.id.clone(), standing(&x.id, wins[&x.id]))).collect();
                    let pairs = generate_swiss_pairings(&t, &st, &hist, &byes, false).unwrap();
                    for (a, b) in &pairs {
                        match b {
                            Some(b) => {
                                total_g += 1;
                                if hist.contains(&(a.clone(), b.clone())) { total_re += 1; }
                                hist.insert((a.clone(), b.clone()));
                                hist.insert((b.clone(), a.clone()));
                                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                                if (state >> 33) % 2 == 0 { *wins.get_mut(a).unwrap() += 1; }
                                else { *wins.get_mut(b).unwrap() += 1; }
                            }
                            None => {
                                byes.insert(a.clone());
                                *bye_counts.entry(a.clone()).or_insert(0) += 1;
                            }
                        }
                    }
                }
                worst_byes = worst_byes.max(bye_counts.values().cloned().max().unwrap_or(0));
            }
            let feasible = r <= n - 1;
            println!("  {:>3} teams / {} rounds: {:>3} rematches of {:>5} games (feasible: {:<5}) worst byes to one team: {}",
                n, r, total_re, total_g, feasible, worst_byes);
        }
    }
}
