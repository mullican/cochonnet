use crate::db::Database;
use crate::models::{GameWithTeams, PanacheSide, QualifyingGame, QualifyingRound, Team, TeamStanding};
use crate::commands::teams::get_team_by_id;
use chrono::Utc;
use super::courts;
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
                   t.region, t.club, t.is_champion, t.is_withdrawn, t.created_at
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
                    is_withdrawn: row.get::<_, i32>(11)? != 0,
                    created_at: row.get(12)?,
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
    // Whether the round before has to be scored first is a property of the
    // pairing method, not of how the operator asked: see
    // `next_round_depends_on_results`.
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

    // Swiss and Pool Play cannot be drawn ahead: each round is built from the
    // last one's results. The rest can be, and also offer 'Generate Next Round'.
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
                "SELECT COUNT(*) FROM teams WHERE tournament_id = ?1 AND is_withdrawn = 0",
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

/// Whether a format's next round is built out of the last one's results.
///
/// Swiss pairs on win records and Pool Play on who won which game, so for those
/// two the round before has to be scored before the next can be drawn. The rest
/// reshuffle without consulting the scoreboard, and write their pairing and
/// court history as they draw rather than as they score, so a further round can
/// be drawn whenever the operator wants one - a director with a settled roster
/// and a sheet to print should not have to wait for the last court to report.
fn next_round_depends_on_results(pairing_method: &str) -> bool {
    pairing_method == "swiss" || pairing_method == "poolPlay"
}

/// Draws one round.
fn generate_single_round(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<QualifyingRound, String> {
    // Get tournament info
    let (pairing_method, region_avoidance, number_of_courts, configured_rounds): (String, bool, i32, i32) = conn
        .query_row(
            "SELECT pairing_method, region_avoidance, number_of_courts, number_of_qualifying_rounds \
             FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| Ok((row.get(0)?, row.get::<_, i32>(1)? != 0, row.get(2)?, row.get(3)?)),
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

    // Only the formats that read the scoreboard have to wait for it.
    if next_round_depends_on_results(&pairing_method) && current_round > 0 {
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

    // Every other format stops at the configured count. The cap used to live
    // only in the generate-all loop, which was enough while the formats drawn
    // up front had no way to be advanced one round at a time.
    if pairing_method != "poolPlay" && new_round_number > configured_rounds {
        return Err(format!(
            "This tournament is configured for {} qualifying rounds.",
            configured_rounds
        ));
    }

    // Get all teams
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, tournament_id, team_number, captain, player2, player3, region, club,
                   is_champion, is_withdrawn, created_at
            FROM teams
            WHERE tournament_id = ?1 AND is_withdrawn = 0
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
        return Err("No active teams registered for this tournament".to_string());
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

    let games: Vec<(Option<String>, Option<String>)> = pairings
        .into_iter()
        .map(|(t1, t2)| (Some(t1), t2))
        .collect();

    // Courts used to be the game's position in this list, which put the same
    // team on the same court round after round - worst of all in round robin,
    // whose order never changes. Draw them against what each team has already
    // had instead. A bye still takes a court number, so nothing downstream
    // shifts, but with no opponent it has no preference of its own.
    let sides: Vec<Vec<String>> = games
        .iter()
        .map(|(t1, t2)| t1.iter().chain(t2.iter()).cloned().collect())
        .collect();
    let court_history = courts::load_qualifying_history(conn, tournament_id)?;
    let assigned = courts::assign_courts(&sides, &court_history, number_of_courts);

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
    for (index, (team1_id, team2_id)) in games.iter().enumerate() {
        let game_id = Uuid::new_v4().to_string();
        let court = assigned[index];
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

/// Moves one qualifying game to another court.
///
/// A clash is deliberately not refused: an operator shuffling games around will
/// pass through states where two share a court, and being stopped mid-shuffle
/// is worse than the clash. The UI shows it instead.
#[tauri::command]
pub fn update_game_court(
    db: State<Database>,
    game_id: String,
    court_number: i32,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    move_game_to_court(&conn, &game_id, court_number)
}

/// The body of `update_game_court`, separated from the Tauri state handle so it
/// can be exercised against a plain connection.
///
/// A game that has been played is not moved: its court is a record of where it
/// happened, not a plan for where it will. The UI stops offering the field at
/// that point; this is what makes it true rather than merely unoffered.
fn move_game_to_court(
    conn: &rusqlite::Connection,
    game_id: &str,
    court_number: i32,
) -> Result<(), String> {
    if court_number < 1 {
        return Err("A court number starts at 1.".to_string());
    }

    let played: bool = conn
        .query_row(
            "SELECT team1_score IS NOT NULL OR team2_score IS NOT NULL \
             FROM qualifying_games WHERE id = ?1",
            params![game_id],
            |row| Ok(row.get::<_, i32>(0)? != 0),
        )
        .map_err(|e| e.to_string())?;

    if played {
        return Err("This game has been played; its court can no longer be changed.".to_string());
    }

    conn.execute(
        "UPDATE qualifying_games SET court_number = ?2 WHERE id = ?1",
        params![game_id, court_number],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
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
    complete_round_inner(&conn, &round_id)
}

/// The body of `complete_round`, separated from the Tauri state handle so it can
/// be exercised against a plain connection - the same split `draw_final` uses.
pub(crate) fn complete_round_inner(
    conn: &rusqlite::Connection,
    round_id: &str,
) -> Result<(), String> {
    // Get tournament ID and pairing method
    let (tournament_id, pairing_method, is_final, already_complete): (String, String, bool, bool) =
        conn.query_row(
            r#"
            SELECT qr.tournament_id, t.pairing_method, qr.is_final, qr.is_complete
            FROM qualifying_rounds qr
            JOIN tournaments t ON qr.tournament_id = t.id
            WHERE qr.id = ?1
            "#,
            params![round_id],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get::<_, i32>(2)? != 0,
                    row.get::<_, i32>(3)? != 0,
                ))
            },
        )
        .map_err(|e| e.to_string())?;

    // Completing a round *adds* each result to a running total, so doing it
    // twice silently doubles that round: every winner in it gains a second win
    // and the whole field's points inflate, which reads as an impossible
    // standings table (six wins from five rounds) with nothing in the games to
    // explain it. The round being complete already is the one state that must
    // never apply again, so this is the guard, not the caller's care: a
    // double-clicked button, a retried command or a replayed backup all arrive
    // here. "Complete this round" is satisfied by it already being complete, so
    // this succeeds rather than erroring - there is nothing for an operator to
    // act on.
    if already_complete {
        return Ok(());
    }

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

    // The regulations give the head-to-head tiebreak to Round Robin and Swiss
    // Hotel ("Rounds"). Not to Swiss System, which ranks on Buchholz, and not to
    // Pool Play or Panache, which they do not mention - those pass None and rank
    // exactly as they did before.
    let head_to_head = match pairing_method.as_str() {
        "swissHotel" | "roundRobin" => Some(HeadToHead::load(&conn, &tournament_id, round_id)?),
        _ => None,
    };

    // Calculate rankings based on pairing method
    match pairing_method.as_str() {
        "swiss" => {
            // Swiss uses Buchholz tiebreaker
            calculate_buchholz_and_ranks(&conn, &tournament_id)?;
        }
        "swissHotel" | "roundRobin" | "poolPlay" | "panache" => {
            // These use point quotient tiebreaker
            calculate_point_quotient_ranks(&conn, &tournament_id, head_to_head.as_ref())?;
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

/// The head-to-head tiebreak, as the tournament regulations phrase it: it
/// applies "when only 2 teams are involved".
///
/// So the group is a win count, and the rule fires only when exactly two teams
/// share it and they met during the qualifiers - then the winner of that game
/// ranks ahead, before any computed tiebreaker, because they settled it on the
/// court. Three or more teams on the same number of wins is left entirely to the
/// existing rules, even when two of them did play.
///
/// That "only 2" clause is what keeps this an ordering rather than a mess. With
/// three teams it would have to answer A beat B, B beat C, C beat A - an
/// ordinary weekend, and a comparator that contradicts itself leaves the
/// published table depending on the order rows came out of SQLite. Restricted to
/// a pair there is no third team to form a cycle, and because the group is a win
/// count - already the first sort key - a team's key is only ever compared
/// against the one other team it can be compared against.
///
/// Applies to Round Robin and Swiss Hotel ("Rounds"). Not Swiss System, which
/// ranks on Buchholz alone, and not Pool Play or Panache, which the regulations
/// do not mention: those three pass `None` and are untouched.
pub(crate) struct HeadToHead {
    /// 1 for the winner of a two-way tie that was settled on court, 0 for
    /// everyone else - including both halves of a pair that never met.
    ahead: HashMap<String, i32>,
}

impl HeadToHead {
    /// Reads the games whose results are already in the standings: every round
    /// flagged complete, plus `current_round_id`, which is being completed right
    /// now and is not flagged until after the ranking has run. Rounds drawn
    /// ahead but not yet played are excluded - their scores are not in the
    /// standings, so they must not sway a tiebreak either.
    pub(crate) fn load(
        conn: &rusqlite::Connection,
        tournament_id: &str,
        current_round_id: &str,
    ) -> Result<Self, String> {
        let mut stmt = conn
            .prepare("SELECT team_id, wins FROM team_standings WHERE tournament_id = ?1")
            .map_err(|e| e.to_string())?;

        let mut by_wins: HashMap<i32, Vec<String>> = HashMap::new();
        let rows = stmt
            .query_map(params![tournament_id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i32>(1)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok());
        for (team_id, wins) in rows {
            by_wins.entry(wins).or_default().push(team_id);
        }

        let mut stmt = conn
            .prepare(
                r#"
                SELECT g.team1_id, g.team2_id, g.team1_score, g.team2_score
                FROM qualifying_games g
                JOIN qualifying_rounds qr ON qr.id = g.round_id
                WHERE qr.tournament_id = ?1
                  AND (qr.is_complete = 1 OR qr.id = ?2)
                  AND qr.is_final = 0
                  AND g.is_bye = 0
                  AND g.team1_id IS NOT NULL
                  AND g.team2_id IS NOT NULL
                  AND g.team1_score IS NOT NULL
                  AND g.team2_score IS NOT NULL
                "#,
            )
            .map_err(|e| e.to_string())?;

        let games: Vec<(String, String, i32, i32)> = stmt
            .query_map(params![tournament_id, current_round_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();

        let mut ahead: HashMap<String, i32> = HashMap::new();

        for (_, tied) in by_wins.iter().filter(|(_, tied)| tied.len() == 2) {
            let (one, other) = (&tied[0], &tied[1]);
            // Swiss Hotel avoids a rematch but does not forbid one, so count the
            // meetings rather than assuming a single game. An even split settles
            // nothing and leaves the existing rules to it.
            let mut wins_for_one = 0;
            let mut wins_for_other = 0;
            for (team1, team2, score1, score2) in &games {
                let is_this_pair = (team1 == one && team2 == other)
                    || (team1 == other && team2 == one);
                // A drawn game is not a petanque result; if one is ever stored it
                // settles nothing and must not hand either side the tiebreak.
                if !is_this_pair || score1 == score2 {
                    continue;
                }
                let winner = if score1 > score2 { team1 } else { team2 };
                if winner == one {
                    wins_for_one += 1;
                } else {
                    wins_for_other += 1;
                }
            }

            if wins_for_one > wins_for_other {
                ahead.insert(one.clone(), 1);
            } else if wins_for_other > wins_for_one {
                ahead.insert(other.clone(), 1);
            }
        }

        Ok(Self { ahead })
    }

    fn is_ahead(&self, team_id: &str) -> i32 {
        self.ahead.get(team_id).copied().unwrap_or(0)
    }
}

/// Orders two teams by the head-to-head key when one is in play.
///
/// No guard on the records is needed: the key is non-zero only inside a win
/// count holding exactly two teams, and `wins` has already been compared by the
/// time this runs, so the only pair whose keys can differ is that one.
fn head_to_head_order(
    head_to_head: Option<&HeadToHead>,
    a: &RankRow,
    b: &RankRow,
) -> std::cmp::Ordering {
    match head_to_head {
        Some(h2h) => h2h.is_ahead(&b.team_id).cmp(&h2h.is_ahead(&a.team_id)),
        None => std::cmp::Ordering::Equal,
    }
}

/// One team's ranking keys. Named rather than a positional tuple because the
/// comparators below read every field and a mis-indexed `.4` reorders a field
/// of 174 teams without failing anything.
struct RankRow {
    team_id: String,
    wins: i32,
    differential: i32,
    buchholz: f64,
    fine_buchholz: f64,
    point_quotient: f64,
    random_tiebreaker: u64,
}

/// Swiss System ranks on Buchholz and takes no head-to-head tiebreak: the
/// regulations apply that one to Round Robin and Swiss Hotel only.
fn calculate_buchholz_and_ranks(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<(), String> {
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

    let mut ranked: Vec<RankRow> = standings
        .iter()
        .map(|(id, wins, diff, _)| RankRow {
            team_id: id.clone(),
            wins: *wins,
            differential: *diff,
            buchholz: buchholz_scores.get(id).copied().unwrap_or(0.0),
            fine_buchholz: fine_buchholz_scores.get(id).copied().unwrap_or(0.0),
            point_quotient: 0.0,
            random_tiebreaker: random_tiebreakers.get(id).copied().unwrap_or(0),
        })
        .collect();

    // wins DESC → buchholz DESC → fine_buchholz DESC → differential DESC → random
    ranked.sort_by(|a, b| {
        b.wins
            .cmp(&a.wins)
            .then(b.buchholz.partial_cmp(&a.buchholz).unwrap_or(std::cmp::Ordering::Equal))
            .then(
                b.fine_buchholz
                    .partial_cmp(&a.fine_buchholz)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(b.differential.cmp(&a.differential))
            .then(b.random_tiebreaker.cmp(&a.random_tiebreaker))
    });

    for (rank, RankRow { team_id, .. }) in ranked.iter().enumerate() {
        conn.execute(
            "UPDATE team_standings SET rank = ?3 WHERE tournament_id = ?1 AND team_id = ?2",
            params![tournament_id, team_id, (rank + 1) as i32],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

/// Calculate ranks using point quotient tiebreaker (for Swiss Hotel, Round Robin, Pool Play)
/// Tiebreaker order: wins → head-to-head → differential → point_quotient → random
///
/// `head_to_head` is `Some` for Round Robin and Swiss Hotel ("Rounds"), the two
/// formats the regulations give it to. Pool Play and Panache pass `None`.
pub(crate) fn calculate_point_quotient_ranks(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    head_to_head: Option<&HeadToHead>,
) -> Result<(), String> {
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
    let mut ranked: Vec<RankRow> = standings
        .iter()
        .map(|(id, wins, diff, points_for, points_against)| {
            let point_quotient = if *points_against > 0 {
                *points_for as f64 / *points_against as f64
            } else if *points_for > 0 {
                f64::MAX
            } else {
                1.0
            };
            RankRow {
                team_id: id.clone(),
                wins: *wins,
                differential: *diff,
                buchholz: 0.0,
                fine_buchholz: 0.0,
                point_quotient,
                random_tiebreaker: random_tiebreakers.get(id).copied().unwrap_or(0),
            }
        })
        .collect();

    // wins DESC → head-to-head → differential DESC → point_quotient DESC → random
    ranked.sort_by(|a, b| {
        b.wins
            .cmp(&a.wins)
            .then_with(|| head_to_head_order(head_to_head, a, b))
            .then(b.differential.cmp(&a.differential))
            .then(
                b.point_quotient
                    .partial_cmp(&a.point_quotient)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
            .then(b.random_tiebreaker.cmp(&a.random_tiebreaker))
    });

    for (rank, RankRow { team_id, .. }) in ranked.iter().enumerate() {
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
            is_withdrawn: false,
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

    // -----------------------------------------------------------------------
    // Round generation against a real database
    // -----------------------------------------------------------------------

    use rusqlite::Connection;

    const TID: &str = "tour";

    /// A swissHotel tournament with `n` teams, all active, and no rounds yet.
    fn seed_round_robin(teams: usize, courts: i32, rounds: i32) -> Connection {
        seed_with_method(teams, courts, rounds, "swissHotel")
    }

    fn seed_with_method(teams: usize, courts: i32, rounds: i32, method: &str) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::create_tables(&conn).unwrap();

        conn.execute(
            r#"
            INSERT INTO tournaments (
                id, name, team_composition, tournament_type, start_date, end_date,
                director, head_umpire, format, day_type, number_of_courts,
                number_of_qualifying_rounds, has_consolante, advance_all, advance_count,
                bracket_size, pairing_method, region_avoidance, created_at, updated_at
            ) VALUES (
                ?1, 'Open', 'mixed', 'club', '2026-01-01', '2026-01-02',
                'D', 'U', 'double', 'single', ?2, ?3, 0, 1, NULL, 16,
                ?4, 0, 'now', 'now'
            )
            "#,
            params![TID, courts, rounds, method],
        )
        .unwrap();

        for i in 1..=teams {
            let id = format!("T{}", i);
            conn.execute(
                "INSERT INTO teams (id, tournament_id, team_number, captain, player2, is_champion, is_withdrawn, created_at)
                 VALUES (?1, ?2, ?3, ?4, 'p2', 0, 0, 'now')",
                params![id, TID, i as i32, format!("Captain {}", i)],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO team_standings (id, tournament_id, team_id) VALUES (?1, ?2, ?3)",
                params![format!("S{}", i), TID, id],
            )
            .unwrap();
        }

        conn
    }

    fn teams_in_round(conn: &Connection, round_id: &str) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT team1_id, team2_id FROM qualifying_games WHERE round_id = ?1")
            .unwrap();
        stmt.query_map(params![round_id], |row| {
            Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?))
        })
        .unwrap()
        .flat_map(|r| {
            let (a, b) = r.unwrap();
            a.into_iter().chain(b)
        })
        .collect()
    }

    /// The point of withdrawal: a team that has pulled out cannot be deleted -
    /// its played games and its opponents' Buchholz depend on it - so the draw
    /// has to be the thing that leaves it out.
    #[test]
    fn a_withdrawn_team_is_left_out_of_later_rounds() {
        let conn = seed_round_robin(6, 3, 5);

        let first = generate_single_round(&conn, TID).unwrap();
        assert!(teams_in_round(&conn, &first.id).contains(&"T3".to_string()));

        conn.execute("UPDATE qualifying_rounds SET is_complete = 1", []).unwrap();
        conn.execute("UPDATE teams SET is_withdrawn = 1 WHERE id = 'T3'", []).unwrap();

        let second = generate_single_round(&conn, TID).unwrap();
        let playing = teams_in_round(&conn, &second.id);
        assert!(
            !playing.contains(&"T3".to_string()),
            "a withdrawn team was drawn into round 2: {:?}",
            playing
        );
        assert_eq!(playing.len(), 5, "the rest of the field should still be playing");
    }

    /// Whether a round needs a bye follows from how many teams are actually
    /// still in, not from how many registered.
    #[test]
    fn an_odd_active_count_draws_a_bye() {
        let conn = seed_round_robin(6, 3, 5);

        let first = generate_single_round(&conn, TID).unwrap();
        let byes: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM qualifying_games WHERE round_id = ?1 AND is_bye = 1",
                params![first.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(byes, 0, "six teams pair up evenly");

        conn.execute("UPDATE qualifying_rounds SET is_complete = 1", []).unwrap();
        conn.execute("UPDATE teams SET is_withdrawn = 1 WHERE id = 'T6'", []).unwrap();

        let second = generate_single_round(&conn, TID).unwrap();
        let byes: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM qualifying_games WHERE round_id = ?1 AND is_bye = 1",
                params![second.id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(byes, 1, "five active teams need a bye");
    }

    /// Swiss pairs on win records, so its next round genuinely cannot be drawn
    /// until the current one is scored.
    #[test]
    fn a_results_driven_draw_waits_for_the_previous_round() {
        let conn = seed_with_method(6, 3, 5, "swiss");
        generate_single_round(&conn, TID).unwrap();

        let err = generate_single_round(&conn, TID).unwrap_err();
        assert!(err.contains("Previous round"), "unexpected refusal: {}", err);
    }

    /// The other formats reshuffle without reading the scoreboard, so making
    /// them wait bought nothing and cost the director the ability to print the
    /// next sheet while the current round is still on the ground.
    #[test]
    fn a_draw_that_ignores_results_does_not_wait() {
        let conn = seed_round_robin(6, 3, 5);
        let first = generate_single_round(&conn, TID).unwrap();
        assert_eq!(first.round_number, 1);

        let second = generate_single_round(&conn, TID)
            .expect("swissHotel should draw ahead of the scoreboard");
        assert_eq!(second.round_number, 2);
    }

    /// Completing a round adds its results to a running total, so the one thing
    /// that must never happen twice is the thing a double-clicked button does.
    /// The symptom is a standings table that cannot be read as a tournament -
    /// more wins than there were rounds - with nothing in the games to explain
    /// it, so the guard belongs here where every caller passes.
    #[test]
    fn completing_a_round_twice_does_not_double_its_results() {
        let conn = seed_round_robin(6, 3, 5);
        let round = generate_single_round(&conn, TID).unwrap();

        conn.execute(
            "UPDATE qualifying_games SET team1_score = 13, team2_score = 7 WHERE round_id = ?1",
            params![round.id],
        )
        .unwrap();

        complete_round_inner(&conn, &round.id).unwrap();

        let after_first: (i32, i32, i32) = conn
            .query_row(
                "SELECT SUM(wins), SUM(losses), SUM(points_for) FROM team_standings WHERE tournament_id = ?1",
                params![TID],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(after_first, (3, 3, 60), "three games, 13-7 each");

        // The second call has to be a no-op, not a second helping.
        complete_round_inner(&conn, &round.id).unwrap();

        let after_second: (i32, i32, i32) = conn
            .query_row(
                "SELECT SUM(wins), SUM(losses), SUM(points_for) FROM team_standings WHERE tournament_id = ?1",
                params![TID],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            after_second, after_first,
            "completing an already-complete round changed the standings"
        );
    }

    /// The configured round count used to be enforced only by the loop that
    /// drew every round at once, so a round-by-round draw could run past it.
    #[test]
    fn a_round_by_round_draw_stops_at_the_configured_count() {
        let conn = seed_round_robin(6, 3, 2);

        for _ in 0..2 {
            generate_single_round(&conn, TID).unwrap();
        }

        let err = generate_single_round(&conn, TID).unwrap_err();
        assert!(err.contains("configured for 2"), "unexpected refusal: {}", err);
    }

    /// Plays a fixed schedule and closes each round through the same path the
    /// Complete Round button uses.
    ///
    /// T1 beats T2 13-12, then loses 0-13 to T3 while T2 wins 13-0. Both finish
    /// on one win - a win count holding exactly two teams - they met, and T1 won
    /// it. But T1's differential is -12 against T2's +12, so differential and
    /// head-to-head point opposite ways, and whichever rule is in force is
    /// visible in the order.
    fn seed_head_to_head_clash(method: &str) -> Connection {
        let schedule = [
            // (round, team1, score1, team2, score2)
            (1, "T1", 13, "T2", 12),
            (1, "T3", 13, "T4", 0),
            (2, "T1", 0, "T3", 13),
            (2, "T2", 13, "T4", 0),
        ];
        seed_played_schedule(method, 4, &schedule)
    }

    fn seed_played_schedule(
        method: &str,
        team_count: usize,
        schedule: &[(i32, &str, i32, &str, i32)],
    ) -> Connection {
        let rounds = schedule.iter().map(|(rd, ..)| *rd).max().unwrap_or(1);
        let conn = seed_with_method(team_count, team_count as i32 / 2, rounds, method);

        for round_number in 1..=rounds {
            let round_id = format!("r{}", round_number);
            conn.execute(
                "INSERT INTO qualifying_rounds (id, tournament_id, round_number, is_complete, is_final, created_at)
                 VALUES (?1, ?2, ?3, 0, 0, 'now')",
                params![round_id, TID, round_number],
            )
            .unwrap();

            for (i, (rd, t1, s1, t2, s2)) in schedule.iter().enumerate() {
                if *rd != round_number {
                    continue;
                }
                conn.execute(
                    "INSERT INTO qualifying_games (id, round_id, court_number, team1_id, team2_id,
                        team1_score, team2_score, is_bye) VALUES (?1, ?2, 1, ?3, ?4, ?5, ?6, 0)",
                    params![format!("g{}-{}", round_number, i), round_id, t1, t2, s1, s2],
                )
                .unwrap();
            }

            complete_round_inner(&conn, &round_id).unwrap();
        }

        conn
    }

    fn rank_of(conn: &Connection, team_id: &str) -> i32 {
        conn.query_row(
            "SELECT rank FROM team_standings WHERE tournament_id = ?1 AND team_id = ?2",
            params![TID, team_id],
            |row| row.get(0),
        )
        .unwrap()
    }

    /// Two teams alone on a win count who met settled it on the court, so that
    /// game outranks differential - which here points the other way.
    #[test]
    fn head_to_head_decides_a_two_way_tie_in_rounds() {
        let conn = seed_head_to_head_clash("swissHotel");

        assert!(
            rank_of(&conn, "T1") < rank_of(&conn, "T2"),
            "T1 beat T2 and the two of them are alone on one win, so T1 ranks first despite the \
             worse differential (T1 rank {}, T2 rank {})",
            rank_of(&conn, "T1"),
            rank_of(&conn, "T2")
        );
    }

    /// The regulations give the rule to Round Robin as well as Rounds.
    #[test]
    fn head_to_head_decides_a_two_way_tie_in_round_robin() {
        let conn = seed_head_to_head_clash("roundRobin");

        assert!(
            rank_of(&conn, "T1") < rank_of(&conn, "T2"),
            "Round Robin takes the head-to-head tiebreak too (T1 rank {}, T2 rank {})",
            rank_of(&conn, "T1"),
            rank_of(&conn, "T2")
        );
    }

    /// Swiss System is explicitly excluded, and this is the test that holds it
    /// out: the same games, the same two-way tie, the same disagreement, and it
    /// still ranks on its own chain.
    #[test]
    fn swiss_system_ignores_head_to_head() {
        let conn = seed_head_to_head_clash("swiss");

        assert!(
            rank_of(&conn, "T2") < rank_of(&conn, "T1"),
            "Swiss takes no head-to-head tiebreak, so T1 stays behind T2 on the Buchholz chain \
             (T1 rank {}, T2 rank {})",
            rank_of(&conn, "T1"),
            rank_of(&conn, "T2")
        );
    }

    /// Two teams alone on a win count who never met fall through to the existing
    /// rules, which is the other half of what the regulations say.
    #[test]
    fn head_to_head_is_skipped_when_the_two_never_met() {
        // T2 and T3 both finish on one win without ever being drawn together.
        let conn = seed_played_schedule(
            "swissHotel",
            4,
            &[
                (1, "T1", 13, "T2", 0),
                (1, "T3", 13, "T4", 0),
                (2, "T1", 13, "T3", 12),
                (2, "T2", 13, "T4", 0),
            ],
        );

        // T3 took +12 off its two games and T2 took 0, so differential decides
        // exactly as it did before this rule existed.
        assert!(
            rank_of(&conn, "T3") < rank_of(&conn, "T2"),
            "with no meeting between them, differential should still order T3 ahead of T2 \
             (T2 rank {}, T3 rank {})",
            rank_of(&conn, "T2"),
            rank_of(&conn, "T3")
        );
    }

    /// "When only 2 teams are involved" is a real limit, not a figure of speech.
    /// Three teams on one win: T1 beat T2, so the pairwise reading would lift T1
    /// over it, but the group is not a pair and the existing rules take the whole
    /// group - which here means differential, putting T2 first.
    ///
    /// This is also what keeps the comparator honest. Head-to-head among three
    /// teams has to be able to answer A over B, B over C and C over A, and no
    /// ordering can; restricting it to a pair means the question never arises.
    #[test]
    fn head_to_head_is_skipped_when_more_than_two_teams_are_tied() {
        // Six teams, two rounds. T1, T2 and T5 all finish on one win; T1 beat T2
        // in round 1, and T2 carries the best differential of the three.
        let conn = seed_played_schedule(
            "swissHotel",
            6,
            &[
                (1, "T1", 13, "T2", 12),
                (1, "T3", 13, "T4", 0),
                (1, "T5", 13, "T6", 1),
                (2, "T1", 0, "T3", 13),
                (2, "T2", 13, "T6", 0),
                (2, "T5", 2, "T4", 13),
            ],
        );

        for team in ["T1", "T2", "T5"] {
            let wins: i32 = conn
                .query_row(
                    "SELECT wins FROM team_standings WHERE tournament_id = ?1 AND team_id = ?2",
                    params![TID, team],
                    |row| row.get(0),
                )
                .unwrap();
            assert_eq!(wins, 1, "{} should be on one win for this test to mean anything", team);
        }

        assert!(
            rank_of(&conn, "T2") < rank_of(&conn, "T1"),
            "three teams share the win count, so head-to-head does not apply and differential \
             orders T2 (+12) ahead of T1 (-12), even though T1 beat T2 \
             (T1 rank {}, T2 rank {})",
            rank_of(&conn, "T1"),
            rank_of(&conn, "T2")
        );
    }

    /// A result from a round drawn ahead but not yet scored into the standings
    /// must not sway a tiebreak - the standings and the tiebreak have to be
    /// reading the same tournament. Drawing ahead became possible when the
    /// round-by-round guard was narrowed to the formats that consume results.
    #[test]
    fn a_round_that_is_not_yet_complete_does_not_count_for_head_to_head() {
        let conn = seed_with_method(2, 1, 2, "swissHotel");

        conn.execute(
            "UPDATE team_standings SET wins = 1, losses = 1 WHERE tournament_id = ?1",
            params![TID],
        )
        .unwrap();
        // Round 2 is drawn and even scored, but never completed.
        conn.execute(
            "INSERT INTO qualifying_rounds (id, tournament_id, round_number, is_complete, is_final, created_at)
             VALUES ('r2', ?1, 2, 0, 0, 'now')",
            params![TID],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO qualifying_games (id, round_id, court_number, team1_id, team2_id,
                team1_score, team2_score, is_bye) VALUES ('g2', 'r2', 1, 'T1', 'T2', 13, 0, 0)",
            [],
        )
        .unwrap();

        // Loading for a different round must not pick r2 up.
        let h2h = HeadToHead::load(&conn, TID, "r1").unwrap();
        assert_eq!(
            h2h.is_ahead("T1"),
            0,
            "an unplayed round's score is not in the standings and must not be in the tiebreak"
        );

        // Loading while r2 is the round being closed must pick it up.
        let h2h = HeadToHead::load(&conn, TID, "r2").unwrap();
        assert_eq!(
            h2h.is_ahead("T1"),
            1,
            "the round being completed is not flagged yet, so it has to be included by id"
        );
    }

    /// Replays the 2024 Amelia Island Open through the real scoring and ranking
    /// code and checks the standings against the order the organizers published.
    ///
    /// The fixture is the tournament's own results workbook: 174 teams, five
    /// rounds, 435 games, every pairing and score exactly as played. It is the
    /// one test here built from a real event rather than a constructed case, so
    /// it is what catches a scoring change that looks reasonable in isolation
    /// and silently reorders a real field.
    ///
    /// The team names in the fixture are placeholders - the real entrants are
    /// not in this repository. Nothing here reads them: teams are keyed by
    /// number and every assertion is on the score line, so the names are
    /// decoration and can be regenerated freely.
    ///
    /// Nineteen teams sit in nine groups that tie on all three ranking keys, and
    /// the app breaks those with a random tiebreaker. So the comparison is
    /// position-by-position on the KEYS, not on team identity: within a tie group
    /// the keys are identical, so any permutation passes, while a team landing in
    /// the wrong place on wins, differential or quotient still fails.
    #[test]
    fn the_2024_amelia_island_open_reproduces_its_published_standings() {
        use serde_json::Value;

        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/aio_2024.json"
        ))
        .expect("fixture missing");
        let fx: Value = serde_json::from_str(&raw).unwrap();

        let conn = Connection::open_in_memory().unwrap();
        crate::db::schema::create_tables(&conn).unwrap();

        let courts = fx["courts"].as_i64().unwrap() as i32;
        conn.execute(
            r#"
            INSERT INTO tournaments (
                id, name, team_composition, tournament_type, start_date, end_date,
                director, head_umpire, format, day_type, number_of_courts,
                number_of_qualifying_rounds, has_consolante, advance_all, advance_count,
                bracket_size, pairing_method, region_avoidance, created_at, updated_at
            ) VALUES (
                ?1, 'AIO 2024 Replay', 'select', 'open', '2024-11-01', '2024-11-03',
                'D', 'U', 'double', 'single', ?2, 5, 1, 1, NULL, 32,
                'swissHotel', 0, 'now', 'now'
            )
            "#,
            params![TID, courts],
        )
        .unwrap();

        let teams = fx["teams"].as_array().unwrap();
        for t in teams {
            let n = t["n"].as_i64().unwrap() as i32;
            conn.execute(
                "INSERT INTO teams (id, tournament_id, team_number, captain, player2, is_champion, is_withdrawn, created_at)
                 VALUES (?1, ?2, ?3, ?4, '', 0, 0, 'now')",
                params![format!("t{}", n), TID, n, t["name"].as_str().unwrap()],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO team_standings (id, tournament_id, team_id) VALUES (?1, ?2, ?3)",
                params![format!("s{}", n), TID, format!("t{}", n)],
            )
            .unwrap();
        }

        // Each round is seeded with the pairings and scores as they were played,
        // then closed through the same path the Complete Round button uses.
        for rd in 1..=5 {
            let round_id = format!("r{}", rd);
            conn.execute(
                "INSERT INTO qualifying_rounds (id, tournament_id, round_number, is_complete, is_final, created_at)
                 VALUES (?1, ?2, ?3, 0, 0, 'now')",
                params![round_id, TID, rd],
            )
            .unwrap();

            for (i, g) in fx["games"][rd.to_string()].as_array().unwrap().iter().enumerate() {
                let g = g.as_array().unwrap();
                let (court, t1, s1, t2, s2) = (
                    g[0].as_i64().unwrap() as i32,
                    g[1].as_i64().unwrap() as i32,
                    g[2].as_i64().unwrap() as i32,
                    g[3].as_i64().unwrap() as i32,
                    g[4].as_i64().unwrap() as i32,
                );
                conn.execute(
                    "INSERT INTO qualifying_games (id, round_id, court_number, team1_id, team2_id,
                        team1_score, team2_score, is_bye) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
                    params![format!("g{}-{}", rd, i), round_id, court,
                            format!("t{}", t1), format!("t{}", t2), s1, s2],
                )
                .unwrap();
            }

            complete_round_inner(&conn, &round_id).unwrap();
        }

        // Every team's own score line must match the workbook exactly.
        let mut expected: HashMap<i32, (i32, i32, i32, i32, i32)> = HashMap::new();
        for t in teams {
            expected.insert(
                t["n"].as_i64().unwrap() as i32,
                (
                    t["rank"].as_i64().unwrap() as i32,
                    t["w"].as_i64().unwrap() as i32,
                    t["diff"].as_i64().unwrap() as i32,
                    t["pf"].as_i64().unwrap() as i32,
                    t["pa"].as_i64().unwrap() as i32,
                ),
            );
        }

        let mut stmt = conn
            .prepare(
                "SELECT t.team_number, s.rank, s.wins, s.differential, s.points_for, s.points_against
                 FROM team_standings s JOIN teams t ON t.id = s.team_id
                 WHERE s.tournament_id = ?1 ORDER BY s.rank ASC",
            )
            .unwrap();
        let actual: Vec<(i32, i32, i32, i32, i32, i32)> = stmt
            .query_map(params![TID], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(actual.len(), teams.len(), "every team must be ranked");

        for (num, _rank, w, diff, pf, pa) in &actual {
            let (_, ew, ed, epf, epa) = expected[num];
            assert_eq!(
                (*w, *diff, *pf, *pa),
                (ew, ed, epf, epa),
                "team {} score line differs from the workbook",
                num
            );
        }

        // Ranks must be 1..n with no gaps or repeats.
        let ranks: Vec<i32> = actual.iter().map(|a| a.1).collect();
        assert_eq!(ranks, (1..=teams.len() as i32).collect::<Vec<_>>(), "ranks are not 1..n");

        // Position-by-position on the ranking keys.
        let key = |w: i32, diff: i32, pf: i32, pa: i32| -> (i32, i32, f64) {
            (w, diff, if pa > 0 { pf as f64 / pa as f64 } else { f64::MAX })
        };
        let mut order: Vec<usize> = (0..teams.len()).collect();
        order.sort_by_key(|&i| expected[&(teams[i]["n"].as_i64().unwrap() as i32)].0);
        let published: Vec<(i32, i32, f64)> = order
            .iter()
            .map(|&i| {
                let (_, w, d, pf, pa) = expected[&(teams[i]["n"].as_i64().unwrap() as i32)];
                (w, d, if pa > 0 { pf as f64 / pa as f64 } else { f64::MAX })
            })
            .collect();

        for (pos, ((_, _, w, diff, pf, pa), want)) in actual.iter().zip(&published).enumerate() {
            let got = key(*w, *diff, *pf, *pa);
            assert_eq!(got.0, want.0, "position {}: wins differ from the published order", pos + 1);
            assert_eq!(got.1, want.1, "position {}: differential differs", pos + 1);
            assert!(
                (got.2 - want.2).abs() < 1e-9,
                "position {}: point quotient differs ({} vs {})",
                pos + 1, got.2, want.2
            );
        }
    }

    /// A court is a plan until the game is played and a record afterwards.
    /// The UI stops offering the field once a score is in; this is what makes
    /// that true rather than merely unoffered.
    #[test]
    fn a_played_game_cannot_be_moved_to_another_court() {
        let conn = seed_round_robin(6, 3, 5);
        let round = generate_single_round(&conn, TID).unwrap();

        let game_id: String = conn
            .query_row(
                "SELECT id FROM qualifying_games WHERE round_id = ?1 LIMIT 1",
                params![round.id],
                |row| row.get(0),
            )
            .unwrap();

        // Unplayed: the court moves.
        move_game_to_court(&conn, &game_id, 2).unwrap();
        let court: i32 = conn
            .query_row(
                "SELECT court_number FROM qualifying_games WHERE id = ?1",
                params![game_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(court, 2);

        // A court number below 1 is never a court.
        assert!(move_game_to_court(&conn, &game_id, 0).is_err());

        conn.execute(
            "UPDATE qualifying_games SET team1_score = 13, team2_score = 9 WHERE id = ?1",
            params![game_id],
        )
        .unwrap();

        let err = move_game_to_court(&conn, &game_id, 3).unwrap_err();
        assert!(err.contains("played"), "unexpected refusal: {}", err);

        let court: i32 = conn
            .query_row(
                "SELECT court_number FROM qualifying_games WHERE id = ?1",
                params![game_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(court, 2, "the played game was moved anyway");
    }

    /// Courts used to be the game's index in the pairing list. Round robin
    /// never shuffles that list, so the team at the top of the circle played
    /// court 1 in every single round of the tournament.
    #[test]
    fn courts_rotate_across_rounds() {
        let conn = seed_round_robin(6, 3, 4);

        let mut seen: HashMap<String, Vec<i32>> = HashMap::new();
        for _ in 0..4 {
            let round = generate_single_round(&conn, TID).unwrap();
            let mut stmt = conn
                .prepare(
                    "SELECT team1_id, team2_id, court_number FROM qualifying_games WHERE round_id = ?1",
                )
                .unwrap();
            let rows: Vec<(Option<String>, Option<String>, i32)> = stmt
                .query_map(params![round.id], |row| {
                    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let mut this_round: Vec<i32> = rows.iter().map(|(_, _, court)| *court).collect();
            let games = this_round.len();
            this_round.sort_unstable();
            this_round.dedup();
            assert_eq!(this_round.len(), games, "two games in one round shared a court");
            assert!(
                this_round.iter().all(|court| (1..=3).contains(court)),
                "a game was sent to a court the venue does not have: {:?}",
                this_round
            );

            for (a, b, court) in rows {
                for team in a.into_iter().chain(b) {
                    seen.entry(team).or_default().push(court);
                }
            }
        }

        // The bug this guards: a team pinned to one court for the whole
        // tournament.
        //
        // The stronger "never twice running" property is asserted in the
        // solver's own tests, where the pairings are fixed and a clean answer
        // always exists. It cannot be asserted here: these pairings reshuffle
        // every round, so a game's two teams can arrive with two different
        // courts to avoid, and with only as many courts as games there may be
        // no assignment that satisfies everyone. The solver minimises the
        // damage; it cannot conjure a solution that is not there.
        for (team, courts) in &seen {
            assert_eq!(courts.len(), 4, "{} did not play every round", team);

            let mut distinct = courts.clone();
            distinct.sort_unstable();
            distinct.dedup();
            assert!(
                distinct.len() > 1,
                "{} drew court {} in every round: {:?}",
                team,
                courts[0],
                courts
            );
        }
    }
}
