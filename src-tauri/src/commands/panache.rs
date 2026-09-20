use crate::db::Database;
use crate::models::{QualifyingRound, Team};
use chrono::Utc;
use super::courts;
use rand::seq::SliceRandom;
use rand::Rng;
use rusqlite::params;
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Cost weights
//
// MELEE.md phrases every constraint as "unless unavoidable", so none of these are
// hard rules. They are costs, ordered so that the things the spec states most
// firmly outrank the things it merely implies.
// ---------------------------------------------------------------------------

/// "Those designees should never be on the same team together."
const W_CHAMPION_COLLISION: u64 = 1000;
/// "attempting to avoid having the same members play together more than once"
const W_TEAMMATE_REPEAT: u64 = 100;
/// "every non-champion should be able to play either with or against a champion at least once"
const W_NO_CHAMPION_EXPOSURE: u64 = 50;
/// Implied by "re-shuffling them for each new round".
const W_OPPONENT_REPEAT: u64 = 10;
/// Keeps the sit-out rotation even.
const W_SITOUT_SPREAD: u64 = 5;

/// Steers a champion team toward opponents who have not met one yet. Not a cost
/// term in the final score, only a tie-breaker while pairing teams into games.
const EXPOSURE_PAIRING_BONUS: i64 = 40;

const RESTARTS: usize = 300;
const LOCAL_SEARCH_PASSES: usize = 3;

// ---------------------------------------------------------------------------
// Schedule types
// ---------------------------------------------------------------------------

/// One round of a panaché draw. `teams` holds the temporary teams as player
/// indices; `games` pairs them; `sitouts` are the surplus players who rest.
#[derive(Debug, Clone)]
pub struct PanacheRound {
    pub teams: Vec<Vec<usize>>,
    pub games: Vec<(usize, usize)>,
    pub sitouts: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct PanacheSchedule {
    pub rounds: Vec<PanacheRound>,
}

/// Running tallies the greedy builder consults and the scorer replays.
struct History {
    teammate: Vec<Vec<u32>>,
    opponent: Vec<Vec<u32>>,
    sitouts: Vec<u32>,
    met_champion: Vec<bool>,
}

impl History {
    fn new(n: usize) -> Self {
        History {
            teammate: vec![vec![0; n]; n],
            opponent: vec![vec![0; n]; n],
            sitouts: vec![0; n],
            met_champion: vec![false; n],
        }
    }

    /// Folds a round in, returning the cost incurred by that round alone.
    fn apply(&mut self, round: &PanacheRound, is_champion: &[bool]) -> u64 {
        let mut cost = 0u64;

        for team in &round.teams {
            let champions_here = team.iter().filter(|&&p| is_champion[p]).count();
            if champions_here > 1 {
                cost += W_CHAMPION_COLLISION * (champions_here as u64 - 1);
            }

            for i in 0..team.len() {
                for j in (i + 1)..team.len() {
                    let (a, b) = (team[i], team[j]);
                    cost += W_TEAMMATE_REPEAT * self.teammate[a][b] as u64;
                    self.teammate[a][b] += 1;
                    self.teammate[b][a] += 1;
                }
            }

            // Everyone on a team with a champion has met one.
            if champions_here > 0 {
                for &p in team {
                    self.met_champion[p] = true;
                }
            }
        }

        for &(t1, t2) in &round.games {
            let side1 = &round.teams[t1];
            let side2 = &round.teams[t2];
            let champ1 = side1.iter().any(|&p| is_champion[p]);
            let champ2 = side2.iter().any(|&p| is_champion[p]);

            for &a in side1 {
                for &b in side2 {
                    cost += W_OPPONENT_REPEAT * self.opponent[a][b] as u64;
                    self.opponent[a][b] += 1;
                    self.opponent[b][a] += 1;
                }
            }

            if champ1 {
                for &p in side2 {
                    self.met_champion[p] = true;
                }
            }
            if champ2 {
                for &p in side1 {
                    self.met_champion[p] = true;
                }
            }
        }

        for &p in &round.sitouts {
            self.sitouts[p] += 1;
        }

        cost
    }

    /// Costs that can only be judged once the whole schedule is known.
    fn terminal_cost(&self, is_champion: &[bool]) -> u64 {
        let mut cost = 0u64;

        let unexposed = (0..is_champion.len())
            .filter(|&p| !is_champion[p] && !self.met_champion[p])
            .count();
        cost += W_NO_CHAMPION_EXPOSURE * unexposed as u64;

        if let (Some(&min), Some(&max)) = (self.sitouts.iter().min(), self.sitouts.iter().max()) {
            cost += W_SITOUT_SPREAD * (max - min) as u64;
        }

        cost
    }
}

/// Total cost of `fixed` (already played, immovable) followed by `candidate`.
fn score(fixed: &[PanacheRound], candidate: &[PanacheRound], n: usize, is_champion: &[bool]) -> u64 {
    let mut history = History::new(n);
    let mut cost = 0u64;
    for round in fixed.iter().chain(candidate.iter()) {
        cost += history.apply(round, is_champion);
    }
    cost + history.terminal_cost(is_champion)
}

/// Draws `rounds` rounds for `n` players, keeping `fixed` rounds untouched.
///
/// Randomized greedy with restarts, then a local-search polish. Every constraint
/// is a cost rather than a filter, so an impossible request (more champions than
/// teams, say) still produces the least-bad draw instead of failing.
pub fn solve_panache_schedule(
    n: usize,
    is_champion: &[bool],
    team_size: usize,
    rounds: usize,
    fixed: &[PanacheRound],
) -> Result<PanacheSchedule, String> {
    let players_per_game = team_size * 2;
    if n < players_per_game {
        return Err(format!(
            "Panaché needs at least {} players for a {}-a-side game; {} registered.",
            players_per_game, team_size, n
        ));
    }
    if rounds == 0 {
        return Ok(PanacheSchedule { rounds: Vec::new() });
    }

    let mut rng = rand::thread_rng();
    let mut best: Option<(u64, Vec<PanacheRound>)> = None;

    for _ in 0..RESTARTS {
        let mut history = History::new(n);
        for round in fixed {
            history.apply(round, is_champion);
        }

        let mut candidate = Vec::with_capacity(rounds);
        for _ in 0..rounds {
            let round = draw_round(n, is_champion, team_size, &history, &mut rng);
            history.apply(&round, is_champion);
            candidate.push(round);
        }

        let cost = score(fixed, &candidate, n, is_champion);
        if best.as_ref().map_or(true, |(b, _)| cost < *b) {
            best = Some((cost, candidate));
        }
        if cost == 0 {
            break;
        }
    }

    let (mut best_cost, mut best_rounds) = best.expect("at least one restart runs");
    local_search(&mut best_rounds, &mut best_cost, fixed, n, is_champion);

    Ok(PanacheSchedule {
        rounds: best_rounds,
    })
}

/// Builds one round: pick sit-outs, form teams, then pair teams into games.
fn draw_round(
    n: usize,
    is_champion: &[bool],
    team_size: usize,
    history: &History,
    rng: &mut impl Rng,
) -> PanacheRound {
    let players_per_game = team_size * 2;
    let surplus = n % players_per_game;

    // Sit-outs go to whoever has sat out least, ties broken randomly, so nobody
    // sits twice before everyone has sat once.
    let mut by_sitouts: Vec<usize> = (0..n).collect();
    by_sitouts.shuffle(rng);
    by_sitouts.sort_by_key(|&p| history.sitouts[p]);
    let sitouts: Vec<usize> = by_sitouts.iter().take(surplus).copied().collect();

    let mut pool: Vec<usize> = by_sitouts.into_iter().skip(surplus).collect();
    pool.shuffle(rng);

    // Form teams: seed from the first unplaced player, then add whoever costs least.
    let mut teams: Vec<Vec<usize>> = Vec::new();
    while !pool.is_empty() {
        let seed = pool.remove(0);
        let mut team = vec![seed];

        while team.len() < team_size && !pool.is_empty() {
            let mut best_at = 0usize;
            let mut best_cost = u64::MAX;
            for (idx, &cand) in pool.iter().enumerate() {
                let mut cost = 0u64;
                for &m in &team {
                    cost += W_TEAMMATE_REPEAT * history.teammate[m][cand] as u64;
                }
                if is_champion[cand] && team.iter().any(|&m| is_champion[m]) {
                    cost += W_CHAMPION_COLLISION;
                }
                // Random jitter keeps equal-cost choices from always picking the
                // same player, which is what makes restarts explore.
                let jittered = cost.saturating_mul(4) + rng.gen_range(0..4);
                if jittered < best_cost {
                    best_cost = jittered;
                    best_at = idx;
                }
            }
            team.push(pool.remove(best_at));
        }

        teams.push(team);
    }

    // Pair teams into games, favouring fresh opponents and champion exposure.
    let mut order: Vec<usize> = (0..teams.len()).collect();
    order.shuffle(rng);

    let mut games = Vec::new();
    let mut used = vec![false; teams.len()];
    for &t1 in &order {
        if used[t1] {
            continue;
        }
        let mut best_at: Option<usize> = None;
        let mut best_cost = i64::MAX;
        for &t2 in &order {
            if t2 == t1 || used[t2] {
                continue;
            }
            let mut cost = 0i64;
            for &a in &teams[t1] {
                for &b in &teams[t2] {
                    cost += (W_OPPONENT_REPEAT * history.opponent[a][b] as u64) as i64;
                }
            }
            let champ1 = teams[t1].iter().any(|&p| is_champion[p]);
            let champ2 = teams[t2].iter().any(|&p| is_champion[p]);
            if champ1 != champ2 {
                let exposed_side = if champ1 { &teams[t2] } else { &teams[t1] };
                let newly = exposed_side
                    .iter()
                    .filter(|&&p| !is_champion[p] && !history.met_champion[p])
                    .count() as i64;
                cost -= EXPOSURE_PAIRING_BONUS * newly;
            }
            let jittered = cost * 4 + rng.gen_range(0..4);
            if jittered < best_cost {
                best_cost = jittered;
                best_at = Some(t2);
            }
        }

        if let Some(t2) = best_at {
            used[t1] = true;
            used[t2] = true;
            games.push((t1, t2));
        }
    }

    PanacheRound {
        teams,
        games,
        sitouts,
    }
}

/// Swaps pairs of players between teams while it lowers the total cost.
///
/// The greedy pass commits to early choices it cannot revisit; this recovers the
/// cases where a late round is forced into a repeat that an earlier swap avoids.
fn local_search(
    rounds: &mut Vec<PanacheRound>,
    best_cost: &mut u64,
    fixed: &[PanacheRound],
    n: usize,
    is_champion: &[bool],
) {
    for _ in 0..LOCAL_SEARCH_PASSES {
        let mut improved = false;

        for r in 0..rounds.len() {
            let team_count = rounds[r].teams.len();
            for t1 in 0..team_count {
                for t2 in (t1 + 1)..team_count {
                    for i in 0..rounds[r].teams[t1].len() {
                        for j in 0..rounds[r].teams[t2].len() {
                            let a = rounds[r].teams[t1][i];
                            let b = rounds[r].teams[t2][j];
                            rounds[r].teams[t1][i] = b;
                            rounds[r].teams[t2][j] = a;

                            let cost = score(fixed, rounds, n, is_champion);
                            if cost < *best_cost {
                                *best_cost = cost;
                                improved = true;
                            } else {
                                rounds[r].teams[t1][i] = a;
                                rounds[r].teams[t2][j] = b;
                            }
                        }
                    }
                }
            }
        }

        if !improved {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Database plumbing
// ---------------------------------------------------------------------------

/// Panaché games are doubles or triples; the roster is always individuals.
fn team_size_for_format(format: &str) -> Result<usize, String> {
    match format {
        "double" => Ok(2),
        "triple" => Ok(3),
        "single" => Err(
            "Panaché games must be doubles or triples. Set the tournament format to Double or Triple."
                .to_string(),
        ),
        other => Err(format!("Unknown tournament format: {}", other)),
    }
}

struct PanacheConfig {
    team_size: usize,
    number_of_rounds: i32,
}

fn load_config(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<PanacheConfig, String> {
    let (pairing_method, format, number_of_rounds): (String, String, i32) = conn
        .query_row(
            "SELECT pairing_method, format, number_of_qualifying_rounds FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;

    if pairing_method != "panache" {
        return Err("This tournament does not use the Panaché format.".to_string());
    }

    Ok(PanacheConfig {
        team_size: team_size_for_format(&format)?,
        number_of_rounds,
    })
}

fn load_players(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<Vec<Team>, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, tournament_id, team_number, captain, player2, player3, region, club,
                   is_champion, is_withdrawn, created_at
            FROM teams
            WHERE tournament_id = ?1 AND is_withdrawn = 0
            ORDER BY team_number ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let players = stmt
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

    Ok(players)
}

/// Rebuilds already-drawn rounds as solver input so a redraw respects them.
fn load_existing_rounds(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    below_round_number: i32,
    index_of: &HashMap<String, usize>,
) -> Result<Vec<PanacheRound>, String> {
    let mut round_stmt = conn
        .prepare(
            "SELECT id FROM qualifying_rounds WHERE tournament_id = ?1 AND round_number < ?2 ORDER BY round_number ASC",
        )
        .map_err(|e| e.to_string())?;
    let round_ids: Vec<String> = round_stmt
        .query_map(params![tournament_id, below_round_number], |row| row.get(0))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(round_stmt);

    let mut rounds = Vec::new();
    for round_id in round_ids {
        let (sides, side_order) = load_sides_for_round(conn, &round_id, index_of)?;

        let mut teams = Vec::new();
        let mut position_of: HashMap<String, usize> = HashMap::new();
        for side_id in &side_order {
            position_of.insert(side_id.clone(), teams.len());
            teams.push(sides.get(side_id).cloned().unwrap_or_default());
        }

        let mut game_stmt = conn
            .prepare("SELECT side1_id, side2_id FROM qualifying_games WHERE round_id = ?1")
            .map_err(|e| e.to_string())?;
        let games: Vec<(usize, usize)> = game_stmt
            .query_map(params![round_id], |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .filter_map(|(s1, s2)| match (s1, s2) {
                (Some(a), Some(b)) => match (position_of.get(&a), position_of.get(&b)) {
                    (Some(&i), Some(&j)) => Some((i, j)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        drop(game_stmt);

        let mut sitout_stmt = conn
            .prepare("SELECT team_id FROM panache_sitouts WHERE round_id = ?1")
            .map_err(|e| e.to_string())?;
        let sitouts: Vec<usize> = sitout_stmt
            .query_map(params![round_id], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .filter_map(|id| index_of.get(&id).copied())
            .collect();
        drop(sitout_stmt);

        rounds.push(PanacheRound {
            teams,
            games,
            sitouts,
        });
    }

    Ok(rounds)
}

/// Returns each side's members (as player indices) plus a stable side ordering.
fn load_sides_for_round(
    conn: &rusqlite::Connection,
    round_id: &str,
    index_of: &HashMap<String, usize>,
) -> Result<(HashMap<String, Vec<usize>>, Vec<String>), String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT pt.id, ptm.team_id
            FROM panache_teams pt
            LEFT JOIN panache_team_members ptm ON ptm.panache_team_id = pt.id
            WHERE pt.round_id = ?1
            ORDER BY pt.team_index ASC, ptm.position ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, Option<String>)> = stmt
        .query_map(params![round_id], |row| Ok((row.get(0)?, row.get(1)?)))
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let mut sides: HashMap<String, Vec<usize>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for (side_id, member_id) in rows {
        if !sides.contains_key(&side_id) {
            sides.insert(side_id.clone(), Vec::new());
            order.push(side_id.clone());
        }
        if let Some(member_id) = member_id {
            if let Some(&idx) = index_of.get(&member_id) {
                sides.get_mut(&side_id).unwrap().push(idx);
            }
        }
    }

    Ok((sides, order))
}

/// Writes one drawn round: the round, its temporary teams, games, sit-outs and history.
fn persist_round(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    round_number: i32,
    is_final: bool,
    round: &PanacheRound,
    players: &[Team],
) -> Result<QualifyingRound, String> {
    let round_id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO qualifying_rounds (id, tournament_id, round_number, is_complete, is_final, created_at)
        VALUES (?1, ?2, ?3, 0, ?4, ?5)
        "#,
        params![
            round_id,
            tournament_id,
            round_number,
            if is_final { 1 } else { 0 },
            now
        ],
    )
    .map_err(|e| e.to_string())?;

    // Temporary teams and their members.
    let mut side_ids: Vec<String> = Vec::with_capacity(round.teams.len());
    for (idx, team) in round.teams.iter().enumerate() {
        let side_id = Uuid::new_v4().to_string();
        conn.execute(
            "INSERT INTO panache_teams (id, tournament_id, round_id, team_index) VALUES (?1, ?2, ?3, ?4)",
            params![side_id, tournament_id, round_id, (idx as i32) + 1],
        )
        .map_err(|e| e.to_string())?;

        for (position, &player_idx) in team.iter().enumerate() {
            conn.execute(
                "INSERT INTO panache_team_members (id, panache_team_id, team_id, position) VALUES (?1, ?2, ?3, ?4)",
                params![
                    Uuid::new_v4().to_string(),
                    side_id,
                    players[player_idx].id,
                    position as i32
                ],
            )
            .map_err(|e| e.to_string())?;
        }

        side_ids.push(side_id);
    }

    // Courts are drawn against what each player has already had, the same way
    // the team formats do it. Identity here is the person: the team they were
    // shuffled into is gone by the next round, their court history is not.
    let sides: Vec<Vec<String>> = round
        .games
        .iter()
        .map(|&(t1, t2)| {
            round.teams[t1]
                .iter()
                .chain(round.teams[t2].iter())
                .map(|&p| players[p].id.clone())
                .collect()
        })
        .collect();
    let number_of_courts: i32 = conn
        .query_row(
            "SELECT number_of_courts FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let court_history = courts::load_qualifying_history(conn, tournament_id)?;
    let assigned = courts::assign_courts(&sides, &court_history, number_of_courts);

    for (game_idx, &(t1, t2)) in round.games.iter().enumerate() {
        let game_id = Uuid::new_v4().to_string();
        let court = assigned[game_idx];

        conn.execute(
            r#"
            INSERT INTO qualifying_games (id, round_id, court_number, team1_id, team2_id, is_bye, side1_id, side2_id)
            VALUES (?1, ?2, ?3, NULL, NULL, 0, ?4, ?5)
            "#,
            params![game_id, round_id, court, side_ids[t1], side_ids[t2]],
        )
        .map_err(|e| e.to_string())?;

        // Opponent history, one row per cross-team player pair, so "who has played
        // whom" stays queryable the same way it is for the team formats.
        for &a in &round.teams[t1] {
            for &b in &round.teams[t2] {
                conn.execute(
                    "INSERT INTO pairing_history (id, tournament_id, team1_id, team2_id, round_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![
                        Uuid::new_v4().to_string(),
                        tournament_id,
                        players[a].id,
                        players[b].id,
                        round_id
                    ],
                )
                .map_err(|e| e.to_string())?;
            }
        }

        for &p in round.teams[t1].iter().chain(round.teams[t2].iter()) {
            conn.execute(
                "INSERT INTO court_history (id, tournament_id, team_id, court_number, round_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    Uuid::new_v4().to_string(),
                    tournament_id,
                    players[p].id,
                    court,
                    round_id
                ],
            )
            .map_err(|e| e.to_string())?;
        }
    }

    for &p in &round.sitouts {
        conn.execute(
            "INSERT INTO panache_sitouts (id, tournament_id, round_id, team_id) VALUES (?1, ?2, ?3, ?4)",
            params![
                Uuid::new_v4().to_string(),
                tournament_id,
                round_id,
                players[p].id
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(QualifyingRound {
        id: round_id,
        tournament_id: tournament_id.to_string(),
        round_number,
        is_complete: false,
        is_final,
        created_at: now,
    })
}

/// Deletes rounds at or after `from_round_number`, refusing if any has a score.
fn delete_rounds_from(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    from_round_number: i32,
) -> Result<(), String> {
    let scored: i32 = conn
        .query_row(
            r#"
            SELECT COUNT(*)
            FROM qualifying_games qg
            JOIN qualifying_rounds qr ON qg.round_id = qr.id
            WHERE qr.tournament_id = ?1 AND qr.round_number >= ?2
              AND (qg.team1_score IS NOT NULL OR qg.team2_score IS NOT NULL)
            "#,
            params![tournament_id, from_round_number],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if scored > 0 {
        return Err(
            "Cannot redraw a round that already has scores. Clear the scores first.".to_string(),
        );
    }

    // pairing_history and court_history cascade on round_id, but delete them
    // explicitly to match how delete_all_qualifying_rounds is written.
    for table in ["pairing_history", "court_history", "panache_sitouts"] {
        conn.execute(
            &format!(
                "DELETE FROM {} WHERE round_id IN (SELECT id FROM qualifying_rounds WHERE tournament_id = ?1 AND round_number >= ?2)",
                table
            ),
            params![tournament_id, from_round_number],
        )
        .map_err(|e| e.to_string())?;
    }

    conn.execute(
        "DELETE FROM qualifying_rounds WHERE tournament_id = ?1 AND round_number >= ?2",
        params![tournament_id, from_round_number],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

fn champion_flags(players: &[Team]) -> Vec<bool> {
    players.iter().map(|p| p.is_champion).collect()
}

fn index_map(players: &[Team]) -> HashMap<String, usize> {
    players
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id.clone(), i))
        .collect()
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Draws the whole qualifying schedule at once.
#[tauri::command]
pub fn generate_panache_rounds(
    db: State<Database>,
    tournament_id: String,
) -> Result<Vec<QualifyingRound>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let config = load_config(&conn, &tournament_id)?;

    let existing: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM qualifying_rounds WHERE tournament_id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if existing > 0 {
        return Err(
            "Rounds have already been drawn. Use Redraw to re-shuffle a round that has no scores."
                .to_string(),
        );
    }

    let players = load_players(&conn, &tournament_id)?;
    if players.is_empty() {
        return Err("No players registered for this tournament".to_string());
    }

    let flags = champion_flags(&players);
    let schedule = solve_panache_schedule(
        players.len(),
        &flags,
        config.team_size,
        config.number_of_rounds as usize,
        &[],
    )?;

    let mut rounds = Vec::new();
    for (idx, round) in schedule.rounds.iter().enumerate() {
        rounds.push(persist_round(
            &conn,
            &tournament_id,
            (idx as i32) + 1,
            false,
            round,
            &players,
        )?);
    }

    Ok(rounds)
}

/// Draws the next round only, holding everything already drawn fixed.
///
/// The counterpart to drawing the whole schedule up front: a director who
/// expects the roster to move - a withdrawal, a late arrival - draws one round
/// at a time so each draw sees the players who are actually still in.
#[tauri::command]
pub fn generate_panache_round(
    db: State<Database>,
    tournament_id: String,
) -> Result<QualifyingRound, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let config = load_config(&conn, &tournament_id)?;

    let finals: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM qualifying_rounds WHERE tournament_id = ?1 AND is_final = 1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if finals > 0 {
        return Err("The final has been drawn; no further qualifying rounds can be added.".to_string());
    }

    let highest: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(round_number), 0) FROM qualifying_rounds WHERE tournament_id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if highest >= config.number_of_rounds {
        return Err(format!(
            "This tournament is configured for {} qualifying rounds.",
            config.number_of_rounds
        ));
    }

    // The round before this one has to be scored: drawing one at a time is only
    // worth doing if each draw sees the state the previous round left behind.
    if highest > 0 {
        let prior_complete: bool = conn
            .query_row(
                "SELECT is_complete FROM qualifying_rounds WHERE tournament_id = ?1 AND round_number = ?2",
                params![tournament_id, highest],
                |row| Ok(row.get::<_, i32>(0)? != 0),
            )
            .map_err(|e| e.to_string())?;
        if !prior_complete {
            return Err("Previous round must be completed before generating the next round.".to_string());
        }
    }

    let players = load_players(&conn, &tournament_id)?;
    if players.is_empty() {
        return Err("No players registered for this tournament".to_string());
    }
    let index_of = index_map(&players);

    // Rounds already drawn are handed to the solver as fixed, so teammate and
    // opponent repeats are still counted against the new round. A player who has
    // withdrawn since is simply absent from `index_of`, and the rounds they
    // appeared in load without them.
    let fixed = load_existing_rounds(&conn, &tournament_id, highest + 1, &index_of)?;

    let flags = champion_flags(&players);
    let schedule = solve_panache_schedule(players.len(), &flags, config.team_size, 1, &fixed)?;

    let round = schedule
        .rounds
        .first()
        .ok_or_else(|| "The draw produced no round.".to_string())?;

    persist_round(&conn, &tournament_id, highest + 1, false, round, &players)
}

/// Re-shuffles every round from `from_round_number` on, holding earlier rounds fixed.
#[tauri::command]
pub fn redraw_panache_rounds(
    db: State<Database>,
    tournament_id: String,
    from_round_number: i32,
) -> Result<Vec<QualifyingRound>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let config = load_config(&conn, &tournament_id)?;

    let players = load_players(&conn, &tournament_id)?;
    if players.is_empty() {
        return Err("No players registered for this tournament".to_string());
    }
    let index_of = index_map(&players);

    // Read the kept rounds before deleting anything.
    let fixed = load_existing_rounds(&conn, &tournament_id, from_round_number, &index_of)?;

    // A redraw only reshuffles qualifying rounds. If the final has been drawn it
    // would be deleted here and reissued as an ordinary round, so refuse instead.
    let finals: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM qualifying_rounds WHERE tournament_id = ?1 AND is_final = 1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if finals > 0 {
        return Err("The final has been drawn; qualifying rounds can no longer be redrawn.".to_string());
    }

    let highest: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(round_number), 0) FROM qualifying_rounds WHERE tournament_id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let remaining = (highest - from_round_number + 1).max(0) as usize;
    if remaining == 0 {
        return Err("There are no rounds to redraw from that point.".to_string());
    }

    delete_rounds_from(&conn, &tournament_id, from_round_number)?;

    let flags = champion_flags(&players);
    let schedule =
        solve_panache_schedule(players.len(), &flags, config.team_size, remaining, &fixed)?;

    let mut rounds = Vec::new();
    for (idx, round) in schedule.rounds.iter().enumerate() {
        rounds.push(persist_round(
            &conn,
            &tournament_id,
            from_round_number + idx as i32,
            false,
            round,
            &players,
        )?);
    }

    Ok(rounds)
}

/// Draws the single final game from the top-ranked players.
#[tauri::command]
pub fn generate_panache_final(
    db: State<Database>,
    tournament_id: String,
) -> Result<QualifyingRound, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    draw_final(&conn, &tournament_id)
}

/// The body of `generate_panache_final`, separated from the Tauri state handle so
/// it can be exercised against a plain connection.
fn draw_final(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<QualifyingRound, String> {
    let config = load_config(conn, tournament_id)?;

    let finals: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM qualifying_rounds WHERE tournament_id = ?1 AND is_final = 1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    if finals > 0 {
        return Err("The final has already been drawn.".to_string());
    }

    // Ranks are only meaningful once a round has been scored; with no completed
    // round every player still sits at rank 0 and the draw would be arbitrary.
    let (total, incomplete): (i32, i32) = conn
        .query_row(
            r#"
            SELECT COUNT(*), COALESCE(SUM(CASE WHEN is_complete = 0 THEN 1 ELSE 0 END), 0)
            FROM qualifying_rounds WHERE tournament_id = ?1
            "#,
            params![tournament_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|e| e.to_string())?;
    if total == 0 {
        return Err("Draw and complete the qualifying rounds before the final.".to_string());
    }
    if incomplete > 0 {
        return Err("Complete every qualifying round before drawing the final.".to_string());
    }

    let players = load_players(conn, tournament_id)?;
    let index_of = index_map(&players);

    let finalist_count = config.team_size * 2;
    let mut stmt = conn
        .prepare(
            r#"
            SELECT ts.team_id
            FROM team_standings ts
            WHERE ts.tournament_id = ?1
            ORDER BY ts.rank ASC
            LIMIT ?2
            "#,
        )
        .map_err(|e| e.to_string())?;
    let finalist_ids: Vec<String> = stmt
        .query_map(params![tournament_id, finalist_count as i64], |row| {
            row.get(0)
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    drop(stmt);

    if finalist_ids.len() < finalist_count {
        return Err(format!(
            "The final needs {} ranked players; only {} available.",
            finalist_count,
            finalist_ids.len()
        ));
    }

    let mut finalists: Vec<usize> = finalist_ids
        .iter()
        .filter_map(|id| index_of.get(id).copied())
        .collect();
    if finalists.len() < finalist_count {
        return Err("Could not resolve every finalist to a registered player.".to_string());
    }

    // The finalists are drawn into two teams at random.
    finalists.shuffle(&mut rand::thread_rng());
    let team2 = finalists.split_off(config.team_size);
    let round = PanacheRound {
        teams: vec![finalists, team2],
        games: vec![(0, 1)],
        sitouts: Vec::new(),
    };

    let round_number: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(round_number), 0) + 1 FROM qualifying_rounds WHERE tournament_id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    persist_round(
        conn,
        tournament_id,
        round_number,
        true,
        &round,
        &players,
    )
}

/// The players resting this round. A sit-out is not a bye: no record is awarded.
#[tauri::command]
pub fn get_sitouts_for_round(
    db: State<Database>,
    round_id: String,
) -> Result<Vec<Team>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT t.id, t.tournament_id, t.team_number, t.captain, t.player2, t.player3,
                   t.region, t.club, t.is_champion, t.is_withdrawn, t.created_at
            FROM panache_sitouts ps
            JOIN teams t ON t.id = ps.team_id
            WHERE ps.round_id = ?1
            ORDER BY t.team_number ASC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let sitouts = stmt
        .query_map(params![round_id], |row| {
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

    Ok(sitouts)
}

/// Marks an entrant as having pulled out, or puts them back in.
///
/// Withdrawing is not deleting: the games they have already played stand, and
/// their opponents' Buchholz still counts them. They are simply left out of
/// every draw from here on.
#[tauri::command]
pub fn set_team_withdrawn(
    db: State<Database>,
    team_id: String,
    is_withdrawn: bool,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE teams SET is_withdrawn = ?2 WHERE id = ?1",
        params![team_id, if is_withdrawn { 1 } else { 0 }],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_team_champion(
    db: State<Database>,
    team_id: String,
    is_champion: bool,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    conn.execute(
        "UPDATE teams SET is_champion = ?2 WHERE id = ?1",
        params![team_id, if is_champion { 1 } else { 0 }],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    fn solve(n: usize, champions: &[usize], team_size: usize, rounds: usize) -> PanacheSchedule {
        let mut flags = vec![false; n];
        for &c in champions {
            flags[c] = true;
        }
        solve_panache_schedule(n, &flags, team_size, rounds, &[]).expect("solver should succeed")
    }

    #[test]
    fn exact_multiple_has_no_sitouts() {
        let schedule = solve(24, &[], 2, 5);
        assert_eq!(schedule.rounds.len(), 5);
        for round in &schedule.rounds {
            assert!(round.sitouts.is_empty());
            assert_eq!(round.teams.len(), 12);
            assert_eq!(round.games.len(), 6);
            for team in &round.teams {
                assert_eq!(team.len(), 2);
            }
        }
    }

    #[test]
    fn triples_split_evenly() {
        let schedule = solve(24, &[], 3, 4);
        for round in &schedule.rounds {
            assert!(round.sitouts.is_empty());
            assert_eq!(round.teams.len(), 8);
            assert_eq!(round.games.len(), 4);
            for team in &round.teams {
                assert_eq!(team.len(), 3);
            }
        }
    }

    #[test]
    fn surplus_players_sit_out_and_rotate() {
        let rounds = 5;
        let schedule = solve(13, &[], 2, rounds);

        let mut sitout_counts = vec![0u32; 13];
        for round in &schedule.rounds {
            assert_eq!(round.sitouts.len(), 1);
            assert_eq!(round.teams.len(), 6);
            assert_eq!(round.games.len(), 3);
            for &p in &round.sitouts {
                sitout_counts[p] += 1;
            }
        }

        // Five sit-outs across thirteen players: nobody should sit twice while
        // someone has not sat at all.
        let max = *sitout_counts.iter().max().unwrap();
        assert_eq!(max, 1, "sit-outs should be spread, got {:?}", sitout_counts);
        assert_eq!(sitout_counts.iter().sum::<u32>(), rounds as u32);
    }

    #[test]
    fn every_player_plays_when_not_sitting_out() {
        let schedule = solve(13, &[], 2, 3);
        for round in &schedule.rounds {
            let mut seen: HashSet<usize> = HashSet::new();
            for team in &round.teams {
                for &p in team {
                    assert!(seen.insert(p), "player {} appears twice in a round", p);
                }
            }
            for &p in &round.sitouts {
                assert!(!seen.contains(&p), "sitting-out player {} also played", p);
            }
            assert_eq!(seen.len() + round.sitouts.len(), 13);
        }
    }

    #[test]
    fn teammates_do_not_repeat_when_avoidable() {
        // 16 players, doubles, 4 rounds: each player has 4 partners out of 15
        // available, so a repeat-free draw exists.
        let schedule = solve(16, &[], 2, 4);

        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        for round in &schedule.rounds {
            for team in &round.teams {
                for i in 0..team.len() {
                    for j in (i + 1)..team.len() {
                        let pair = (team[i].min(team[j]), team[i].max(team[j]));
                        assert!(seen.insert(pair), "repeat teammates {:?}", pair);
                    }
                }
            }
        }
    }

    #[test]
    fn champions_are_never_teamed_together() {
        // 24 players in doubles is 12 teams, so 4 champions fit apart easily.
        let champions = [0usize, 1, 2, 3];
        let schedule = solve(24, &champions, 2, 5);

        for round in &schedule.rounds {
            for team in &round.teams {
                let count = team.iter().filter(|p| champions.contains(p)).count();
                assert!(count <= 1, "two champions on one team: {:?}", team);
            }
        }
    }

    #[test]
    fn every_non_champion_meets_a_champion() {
        let champions = [0usize, 1, 2];
        let schedule = solve(24, &champions, 2, 5);

        let mut exposed: HashSet<usize> = HashSet::new();
        for round in &schedule.rounds {
            for team in &round.teams {
                if team.iter().any(|p| champions.contains(p)) {
                    exposed.extend(team.iter().copied());
                }
            }
            for &(t1, t2) in &round.games {
                let c1 = round.teams[t1].iter().any(|p| champions.contains(p));
                let c2 = round.teams[t2].iter().any(|p| champions.contains(p));
                if c1 {
                    exposed.extend(round.teams[t2].iter().copied());
                }
                if c2 {
                    exposed.extend(round.teams[t1].iter().copied());
                }
            }
        }

        let missed: Vec<usize> = (0..24)
            .filter(|p| !champions.contains(p) && !exposed.contains(p))
            .collect();
        assert!(missed.is_empty(), "players never met a champion: {:?}", missed);
    }

    // -----------------------------------------------------------------------
    // Database round-trip
    //
    // Exercises the real persistence path: solve a schedule, write it, read the
    // sides back the way get_games_for_round does, score every game, and confirm a
    // shared result lands on each member of the temporary team individually.
    // -----------------------------------------------------------------------

    use crate::commands::qualifying::{apply_game_result, load_side_member_ids};
    use crate::db::schema::create_tables;
    use rusqlite::Connection;

    fn seed_tournament(player_count: usize, format: &str, champions: &[usize]) -> (Connection, Vec<Team>) {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
        create_tables(&conn).unwrap();

        conn.execute(
            r#"
            INSERT INTO tournaments (
                id, name, team_composition, tournament_type, start_date, end_date,
                director, head_umpire, format, day_type, number_of_courts,
                number_of_qualifying_rounds, has_consolante, advance_all, advance_count,
                bracket_size, pairing_method, region_avoidance, created_at, updated_at
            ) VALUES (
                'T', 'Panache Open', 'mixed', 'club', '2026-01-01', '2026-01-02',
                'D', 'U', ?1, 'single', 12, 3, 0, 1, NULL, 16, 'panache', 0, 'now', 'now'
            )
            "#,
            params![format],
        )
        .unwrap();

        for i in 0..player_count {
            let id = format!("p{}", i);
            conn.execute(
                r#"
                INSERT INTO teams (id, tournament_id, team_number, captain, player2, player3,
                                   region, club, is_champion, created_at)
                VALUES (?1, 'T', ?2, ?3, '', NULL, NULL, NULL, ?4, 'now')
                "#,
                params![
                    id,
                    (i as i32) + 1,
                    format!("Player {}", i),
                    if champions.contains(&i) { 1 } else { 0 }
                ],
            )
            .unwrap();
            conn.execute(
                r#"
                INSERT INTO team_standings (id, tournament_id, team_id, wins, losses,
                                            points_for, points_against, differential,
                                            buchholz_score, rank)
                VALUES (?1, 'T', ?2, 0, 0, 0, 0, 0, 0, 0)
                "#,
                params![format!("s{}", i), id],
            )
            .unwrap();
        }

        let players = load_players(&conn, "T").unwrap();
        assert_eq!(players.len(), player_count);
        (conn, players)
    }

    #[test]
    fn persisted_rounds_read_back_with_their_sides() {
        let (conn, players) = seed_tournament(13, "double", &[]);
        let flags = champion_flags(&players);
        let schedule = solve_panache_schedule(players.len(), &flags, 2, 3, &[]).unwrap();

        for (idx, round) in schedule.rounds.iter().enumerate() {
            persist_round(&conn, "T", (idx as i32) + 1, false, round, &players).unwrap();
        }

        let rounds: i32 = conn
            .query_row("SELECT COUNT(*) FROM qualifying_rounds", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rounds, 3);

        // Three rounds of three games; 13 players means one sits out each round.
        let games: i32 = conn
            .query_row("SELECT COUNT(*) FROM qualifying_games", [], |r| r.get(0))
            .unwrap();
        assert_eq!(games, 9);

        let sitouts: i32 = conn
            .query_row("SELECT COUNT(*) FROM panache_sitouts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sitouts, 3);

        // Every game references two temporary teams and no registered team.
        let mut stmt = conn
            .prepare("SELECT team1_id, team2_id, side1_id, side2_id FROM qualifying_games")
            .unwrap();
        let rows: Vec<(Option<String>, Option<String>, Option<String>, Option<String>)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        drop(stmt);

        for (t1, t2, s1, s2) in &rows {
            assert!(t1.is_none() && t2.is_none(), "panache games hold no registered team");
            let members1 = load_side_member_ids(&conn, s1.as_ref().unwrap()).unwrap();
            let members2 = load_side_member_ids(&conn, s2.as_ref().unwrap()).unwrap();
            assert_eq!(members1.len(), 2);
            assert_eq!(members2.len(), 2);
        }
    }

    #[test]
    fn a_shared_score_lands_on_each_member_individually() {
        let (conn, players) = seed_tournament(8, "double", &[]);
        let flags = champion_flags(&players);
        let schedule = solve_panache_schedule(players.len(), &flags, 2, 1, &[]).unwrap();
        persist_round(&conn, "T", 1, false, &schedule.rounds[0], &players).unwrap();

        let mut stmt = conn
            .prepare("SELECT side1_id, side2_id FROM qualifying_games ORDER BY court_number")
            .unwrap();
        let sides: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        drop(stmt);
        assert_eq!(sides.len(), 2);

        // Side one wins 13-7 in both games.
        for (s1, s2) in &sides {
            let winners = load_side_member_ids(&conn, s1).unwrap();
            let losers = load_side_member_ids(&conn, s2).unwrap();
            apply_game_result(&conn, "T", &winners, 13, 7).unwrap();
            apply_game_result(&conn, "T", &losers, 7, 13).unwrap();
        }

        let mut stmt = conn
            .prepare("SELECT wins, losses, points_for, points_against, differential FROM team_standings")
            .unwrap();
        let standings: Vec<(i32, i32, i32, i32, i32)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        drop(stmt);

        assert_eq!(standings.len(), 8);
        // Four winners and four losers: each member of a temporary team carries the
        // whole team's result, not a share of it.
        let winners: Vec<_> = standings.iter().filter(|s| s.0 == 1).collect();
        let losers: Vec<_> = standings.iter().filter(|s| s.1 == 1).collect();
        assert_eq!(winners.len(), 4);
        assert_eq!(losers.len(), 4);
        for w in winners {
            assert_eq!(*w, (1, 0, 13, 7, 6));
        }
        for l in losers {
            assert_eq!(*l, (0, 1, 7, 13, -6));
        }
    }

    #[test]
    fn sitting_out_costs_a_player_nothing() {
        let (conn, players) = seed_tournament(9, "double", &[]);
        let flags = champion_flags(&players);
        let schedule = solve_panache_schedule(players.len(), &flags, 2, 1, &[]).unwrap();
        persist_round(&conn, "T", 1, false, &schedule.rounds[0], &players).unwrap();

        let rested: String = conn
            .query_row("SELECT team_id FROM panache_sitouts", [], |r| r.get(0))
            .unwrap();

        let mut stmt = conn
            .prepare("SELECT side1_id, side2_id FROM qualifying_games")
            .unwrap();
        let sides: Vec<(String, String)> = stmt
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        drop(stmt);

        for (s1, s2) in &sides {
            apply_game_result(&conn, "T", &load_side_member_ids(&conn, s1).unwrap(), 13, 7).unwrap();
            apply_game_result(&conn, "T", &load_side_member_ids(&conn, s2).unwrap(), 7, 13).unwrap();
        }

        // A sit-out is not a bye: no win, no loss, no points either way.
        let (wins, losses, pf, pa): (i32, i32, i32, i32) = conn
            .query_row(
                "SELECT wins, losses, points_for, points_against FROM team_standings WHERE team_id = ?1",
                params![rested],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .unwrap();
        assert_eq!((wins, losses, pf, pa), (0, 0, 0, 0));
    }

    #[test]
    fn deleting_a_round_clears_its_temporary_teams() {
        let (conn, players) = seed_tournament(8, "double", &[]);
        let flags = champion_flags(&players);
        let schedule = solve_panache_schedule(players.len(), &flags, 2, 2, &[]).unwrap();
        for (idx, round) in schedule.rounds.iter().enumerate() {
            persist_round(&conn, "T", (idx as i32) + 1, false, round, &players).unwrap();
        }

        delete_rounds_from(&conn, "T", 2).unwrap();

        let rounds: i32 = conn
            .query_row("SELECT COUNT(*) FROM qualifying_rounds", [], |r| r.get(0))
            .unwrap();
        assert_eq!(rounds, 1);

        // The round-two sides and their membership rows must cascade away, leaving
        // only round one's four teams of two.
        let sides: i32 = conn
            .query_row("SELECT COUNT(*) FROM panache_teams", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sides, 4);
        let members: i32 = conn
            .query_row("SELECT COUNT(*) FROM panache_team_members", [], |r| r.get(0))
            .unwrap();
        assert_eq!(members, 8);
    }

    #[test]
    fn a_scored_round_cannot_be_redrawn() {
        let (conn, players) = seed_tournament(8, "double", &[]);
        let flags = champion_flags(&players);
        let schedule = solve_panache_schedule(players.len(), &flags, 2, 2, &[]).unwrap();
        for (idx, round) in schedule.rounds.iter().enumerate() {
            persist_round(&conn, "T", (idx as i32) + 1, false, round, &players).unwrap();
        }

        conn.execute(
            r#"
            UPDATE qualifying_games SET team1_score = 13, team2_score = 7
            WHERE round_id IN (SELECT id FROM qualifying_rounds WHERE round_number = 2)
            "#,
            [],
        )
        .unwrap();

        assert!(delete_rounds_from(&conn, "T", 2).is_err());
    }

    /// Scores every game in a round the way complete_round does, then ranks.
    fn score_and_complete(conn: &Connection, tid: &str, round_number: i32) {
        let rid: String = conn
            .query_row(
                "SELECT id FROM qualifying_rounds WHERE tournament_id = ?1 AND round_number = ?2",
                params![tid, round_number],
                |r| r.get(0),
            )
            .unwrap();
        let games: Vec<(String, String)> = conn
            .prepare("SELECT side1_id, side2_id FROM qualifying_games WHERE round_id = ?1 ORDER BY court_number")
            .unwrap()
            .query_map(params![rid], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        for (s1, s2) in games {
            apply_game_result(conn, tid, &load_side_member_ids(conn, &s1).unwrap(), 13, 7).unwrap();
            apply_game_result(conn, tid, &load_side_member_ids(conn, &s2).unwrap(), 7, 13).unwrap();
        }
        conn.execute(
            "UPDATE qualifying_rounds SET is_complete = 1 WHERE id = ?1",
            params![rid],
        )
        .unwrap();
        crate::commands::qualifying::calculate_point_quotient_ranks(conn, tid).unwrap();
    }

    #[test]
    fn the_final_draws_the_top_players_into_two_teams() {
        let (conn, players) = seed_tournament(12, "double", &[]);
        let flags = champion_flags(&players);
        let schedule = solve_panache_schedule(players.len(), &flags, 2, 2, &[]).unwrap();
        for (idx, round) in schedule.rounds.iter().enumerate() {
            persist_round(&conn, "T", (idx as i32) + 1, false, round, &players).unwrap();
        }
        for rn in 1..=2 {
            score_and_complete(&conn, "T", rn);
        }

        // The top four by rank are the finalists.
        let expected: Vec<String> = conn
            .prepare("SELECT team_id FROM team_standings WHERE tournament_id = 'T' ORDER BY rank LIMIT 4")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        let standings_before: Vec<(String, i32, i32, i32)> = conn
            .prepare("SELECT team_id, wins, losses, rank FROM team_standings WHERE tournament_id='T' ORDER BY team_id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();

        let final_round = draw_final(&conn, "T").unwrap();
        assert!(final_round.is_final);
        assert_eq!(final_round.round_number, 3);

        // Exactly one game, between two teams of two.
        let games: Vec<(String, String)> = conn
            .prepare("SELECT side1_id, side2_id FROM qualifying_games WHERE round_id = ?1")
            .unwrap()
            .query_map(params![final_round.id], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(games.len(), 1);

        let mut drawn = load_side_member_ids(&conn, &games[0].0).unwrap();
        let other = load_side_member_ids(&conn, &games[0].1).unwrap();
        assert_eq!(drawn.len(), 2);
        assert_eq!(other.len(), 2);
        drawn.extend(other);
        drawn.sort();

        let mut want = expected.clone();
        want.sort();
        assert_eq!(drawn, want, "the final must draw exactly the top four");

        // Nobody sits out a final, and the qualifying standings are untouched.
        let sitouts: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM panache_sitouts WHERE round_id = ?1",
                params![final_round.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(sitouts, 0);

        let standings_after: Vec<(String, i32, i32, i32)> = conn
            .prepare("SELECT team_id, wins, losses, rank FROM team_standings WHERE tournament_id='T' ORDER BY team_id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(standings_before, standings_after);
    }

    #[test]
    fn the_final_is_drawn_only_once_and_only_when_ready() {
        let (conn, players) = seed_tournament(12, "double", &[]);

        // No rounds drawn yet: every rank is still 0, so there is nothing to seed from.
        assert!(draw_final(&conn, "T").is_err());

        let flags = champion_flags(&players);
        let schedule = solve_panache_schedule(players.len(), &flags, 2, 2, &[]).unwrap();
        for (idx, round) in schedule.rounds.iter().enumerate() {
            persist_round(&conn, "T", (idx as i32) + 1, false, round, &players).unwrap();
        }

        // Rounds drawn but not scored.
        assert!(draw_final(&conn, "T").is_err());

        for rn in 1..=2 {
            score_and_complete(&conn, "T", rn);
        }
        assert!(draw_final(&conn, "T").is_ok());

        // A second call must not draw another final.
        assert!(draw_final(&conn, "T").is_err());

        // And with a final on the board, qualifying rounds can no longer be redrawn.
        let finals: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM qualifying_rounds WHERE tournament_id='T' AND is_final=1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(finals, 1);
    }

    #[test]
    fn triples_finals_take_the_top_six() {
        let (conn, players) = seed_tournament(18, "triple", &[]);
        let flags = champion_flags(&players);
        let schedule = solve_panache_schedule(players.len(), &flags, 3, 2, &[]).unwrap();
        for (idx, round) in schedule.rounds.iter().enumerate() {
            persist_round(&conn, "T", (idx as i32) + 1, false, round, &players).unwrap();
        }
        for rn in 1..=2 {
            score_and_complete(&conn, "T", rn);
        }

        let final_round = draw_final(&conn, "T").unwrap();
        let (s1, s2): (String, String) = conn
            .query_row(
                "SELECT side1_id, side2_id FROM qualifying_games WHERE round_id = ?1",
                params![final_round.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(load_side_member_ids(&conn, &s1).unwrap().len(), 3);
        assert_eq!(load_side_member_ids(&conn, &s2).unwrap().len(), 3);
    }

    #[test]
    fn too_few_players_is_rejected() {
        let flags = vec![false; 3];
        assert!(solve_panache_schedule(3, &flags, 2, 3, &[]).is_err());
    }

    #[test]
    fn fixed_rounds_are_respected_on_redraw() {
        let flags = vec![false; 16];
        let first = solve_panache_schedule(16, &flags, 2, 2, &[]).unwrap();

        // Redrawing rounds 3-4 must not reuse partners from rounds 1-2.
        let rest = solve_panache_schedule(16, &flags, 2, 2, &first.rounds).unwrap();

        let mut seen: HashSet<(usize, usize)> = HashSet::new();
        for round in first.rounds.iter().chain(rest.rounds.iter()) {
            for team in &round.teams {
                let pair = (team[0].min(team[1]), team[0].max(team[1]));
                assert!(seen.insert(pair), "redraw reused partners {:?}", pair);
            }
        }
    }
}
