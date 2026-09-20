//! Spreading games across the courts so nobody keeps landing on the same one.
//!
//! MELEE.md phrases the rule the way a director would: a team should not play
//! the same court twice, and where the venue is too small for that they should
//! at least not play it twice running. Both are "unless unavoidable", so they
//! are costs rather than filters - a round has to be drawn even when every
//! choice breaks something.
//!
//! Weighting the consecutive repeat above the plain one is what encodes the
//! fallback: with courts to spare the solver reaches zero and nobody repeats,
//! and when it cannot it spends the cheap repeats first.
//!
//! This is a pure function over a history it is handed, in the same shape as
//! the panache scheduler, so it can be tested without a database.

use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::{HashMap, HashSet};

/// A team drawing the same court in back-to-back rounds - the rule that holds
/// even when the venue is too small to avoid repeats altogether.
const SAME_COURT_IN_A_ROW: i64 = 1000;
/// A team drawing a court it has played on at some earlier point.
const SAME_COURT_AGAIN: i64 = 100;

/// How many greedy draws to try before keeping the best.
const RESTARTS: usize = 12;
/// Cap on improvement sweeps, so a pathological field cannot spin.
const MAX_SWEEPS: usize = 8;

/// Where the competitors of a round have been put so far.
///
/// Keys are competitor ids - a team in the team formats, an individual player
/// in panache, where the temporary teams change every round and only the people
/// carry any history.
#[derive(Debug, Default, Clone)]
pub struct CourtHistory {
    /// Every court a competitor has been assigned.
    pub ever: HashMap<String, HashSet<i32>>,
    /// The court a competitor was on in the round immediately before this one.
    pub previous: HashMap<String, i32>,
}

impl CourtHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records one competitor's court. `is_previous_round` marks the most
    /// recent round, which is the one the back-to-back rule looks at.
    pub fn record(&mut self, competitor_id: &str, court: i32, is_previous_round: bool) {
        self.ever
            .entry(competitor_id.to_string())
            .or_default()
            .insert(court);
        if is_previous_round {
            self.previous.insert(competitor_id.to_string(), court);
        }
    }

    fn cost_for(&self, competitors: &[String], court: i32) -> i64 {
        competitors
            .iter()
            .map(|id| {
                let mut cost = 0;
                if self.previous.get(id) == Some(&court) {
                    cost += SAME_COURT_IN_A_ROW;
                }
                if self.ever.get(id).is_some_and(|seen| seen.contains(&court)) {
                    cost += SAME_COURT_AGAIN;
                }
                cost
            })
            .sum()
    }
}

/// The court numbers one set of simultaneous games has to share.
///
/// Normally one court each. A bracket wave with more games than the venue has
/// courts has to double up - the extra games wait for a court to free up - so
/// the numbers wrap, exactly as the sequential numbering this replaces did.
pub fn court_slots(game_count: usize, number_of_courts: i32) -> Vec<i32> {
    (0..game_count)
        .map(|i| (i as i32 % number_of_courts) + 1)
        .collect()
}

/// Puts each game on a court, keeping teams off courts they have already had.
///
/// `games` lists the competitors on both sides of each game; a game with nobody
/// in it (a bye placeholder) simply has no preferences. The returned courts line
/// up with `games` by index.
pub fn assign_courts(
    games: &[Vec<String>],
    history: &CourtHistory,
    number_of_courts: i32,
) -> Vec<i32> {
    if number_of_courts < 1 {
        return vec![1; games.len()];
    }
    assign_courts_from_slots(games, history, &court_slots(games.len(), number_of_courts))
}

/// The same draw over a court list the caller has already worked out.
///
/// Brackets need this: a wave can hold games whose court is already settled -
/// played, or set by hand - and those courts are gone from what the rest of the
/// wave has to share.
pub fn assign_courts_from_slots(
    games: &[Vec<String>],
    history: &CourtHistory,
    slots: &[i32],
) -> Vec<i32> {
    if games.is_empty() {
        return Vec::new();
    }
    debug_assert_eq!(games.len(), slots.len(), "one slot per game");

    let mut rng = thread_rng();

    let mut best: Option<(i64, Vec<i32>)> = None;

    for _ in 0..RESTARTS {
        let mut order: Vec<usize> = (0..games.len()).collect();
        order.shuffle(&mut rng);

        let mut pool = slots.to_vec();
        pool.shuffle(&mut rng);

        let mut courts = vec![0; games.len()];
        for &game in &order {
            // Cheapest remaining court for this game. The pool was shuffled, so
            // equally good courts are taken in no particular order rather than
            // always the lowest-numbered one.
            let pick = pool
                .iter()
                .enumerate()
                .min_by_key(|(_, &court)| history.cost_for(&games[game], court))
                .map(|(idx, _)| idx)
                .expect("pool holds one slot per game");
            courts[game] = pool.swap_remove(pick);
        }

        improve(games, history, &mut courts);

        let cost = total_cost(games, history, &courts);
        if best.as_ref().is_none_or(|(b, _)| cost < *b) {
            best = Some((cost, courts));
        }
        if cost == 0 {
            break;
        }
    }

    best.map(|(_, courts)| courts).unwrap_or_default()
}

fn total_cost(games: &[Vec<String>], history: &CourtHistory, courts: &[i32]) -> i64 {
    games
        .iter()
        .zip(courts)
        .map(|(competitors, &court)| history.cost_for(competitors, court))
        .sum()
}

/// Swaps pairs of games between their courts for as long as that helps.
///
/// The greedy pass commits to a court before it has seen the games that come
/// after, so it regularly leaves a swap on the table that costs nothing to take.
fn improve(games: &[Vec<String>], history: &CourtHistory, courts: &mut [i32]) {
    for _ in 0..MAX_SWEEPS {
        let mut improved = false;

        for a in 0..games.len() {
            for b in (a + 1)..games.len() {
                if courts[a] == courts[b] {
                    continue;
                }
                let before = history.cost_for(&games[a], courts[a])
                    + history.cost_for(&games[b], courts[b]);
                let after = history.cost_for(&games[a], courts[b])
                    + history.cost_for(&games[b], courts[a]);
                if after < before {
                    courts.swap(a, b);
                    improved = true;
                }
            }
        }

        if !improved {
            break;
        }
    }
}

// ---------------------------------------------------------------------------
// Reading the history out of the database
// ---------------------------------------------------------------------------

/// What the qualifying rounds have already put each competitor on.
///
/// `court_history` has been written since the table was added and never once
/// read; this is what it was for. Rows are per competitor, which for panache
/// means per player - the temporary team they were drawn into is gone by the
/// next round, the person is not.
pub fn load_qualifying_history(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<CourtHistory, String> {
    let mut stmt = conn
        .prepare(
            r#"
            SELECT ch.team_id, ch.court_number, r.round_number
            FROM court_history ch
            JOIN qualifying_rounds r ON r.id = ch.round_id
            WHERE ch.tournament_id = ?1
            "#,
        )
        .map_err(|e| e.to_string())?;

    let rows: Vec<(String, i32, i32)> = stmt
        .query_map(rusqlite::params![tournament_id], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    let latest = rows.iter().map(|(_, _, round)| *round).max().unwrap_or(0);

    let mut history = CourtHistory::new();
    for (team_id, court, round_number) in rows {
        history.record(&team_id, court, round_number == latest);
    }

    Ok(history)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Plays `rounds` rounds of `pairs` fixed pairings and returns, per team,
    /// the courts it was given in order.
    fn play(pairs: &[(&str, &str)], courts: i32, rounds: usize) -> HashMap<String, Vec<i32>> {
        let games: Vec<Vec<String>> = pairs
            .iter()
            .map(|(a, b)| vec![a.to_string(), b.to_string()])
            .collect();

        let mut history = CourtHistory::new();
        let mut seen: HashMap<String, Vec<i32>> = HashMap::new();

        for _ in 0..rounds {
            let assigned = assign_courts(&games, &history, courts);

            // Last round's courts stop being "the previous round"; only this
            // round's are, which is what the back-to-back rule reads.
            history.previous.clear();
            for (game, court) in games.iter().zip(&assigned) {
                for id in game {
                    history.record(id, *court, true);
                    seen.entry(id.clone()).or_default().push(*court);
                }
            }
        }

        seen
    }

    fn pairs_for(n: usize) -> Vec<(String, String)> {
        (0..n)
            .map(|i| (format!("A{}", i), format!("B{}", i)))
            .collect()
    }

    /// The whole point of the feature. Court numbers used to be the game's
    /// index in the pairing list, so a format that does not shuffle - round
    /// robin - put the same team on court 1 every single round.
    #[test]
    fn nobody_repeats_a_court_while_there_is_room() {
        for _ in 0..20 {
            let owned = pairs_for(4);
            let pairs: Vec<(&str, &str)> = owned
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect();

            // Four games, eight courts, four rounds: every team can have a
            // fresh court every time.
            for (team, courts) in play(&pairs, 8, 4) {
                let unique: HashSet<i32> = courts.iter().copied().collect();
                assert_eq!(
                    unique.len(),
                    courts.len(),
                    "{} repeated a court: {:?}",
                    team,
                    courts
                );
            }
        }
    }

    /// The fallback. With fewer courts than rounds a repeat is arithmetically
    /// unavoidable, so the rule that has to survive is the weaker one.
    #[test]
    fn a_starved_venue_still_never_repeats_a_court_in_a_row() {
        for _ in 0..20 {
            let owned = pairs_for(3);
            let pairs: Vec<(&str, &str)> = owned
                .iter()
                .map(|(a, b)| (a.as_str(), b.as_str()))
                .collect();

            // Three games on three courts over six rounds: every team must
            // repeat, but never twice running.
            for (team, courts) in play(&pairs, 3, 6) {
                for window in courts.windows(2) {
                    assert_ne!(
                        window[0], window[1],
                        "{} drew court {} twice running: {:?}",
                        team, window[0], courts
                    );
                }
            }
        }
    }

    /// Two games must never be sent to the same court while a free one exists.
    #[test]
    fn simultaneous_games_do_not_share_a_court() {
        let owned = pairs_for(5);
        let games: Vec<Vec<String>> = owned
            .iter()
            .map(|(a, b)| vec![a.clone(), b.clone()])
            .collect();

        let courts = assign_courts(&games, &CourtHistory::new(), 6);
        let unique: HashSet<i32> = courts.iter().copied().collect();
        assert_eq!(unique.len(), games.len(), "courts double-booked: {:?}", courts);
    }

    /// A bracket wave can be bigger than the venue. The extra games wait for a
    /// court, which is what wrapping the numbers means - but no court may be
    /// asked to hold more games than it has to.
    #[test]
    fn a_wave_larger_than_the_venue_spreads_as_evenly_as_it_can() {
        let owned = pairs_for(7);
        let games: Vec<Vec<String>> = owned
            .iter()
            .map(|(a, b)| vec![a.clone(), b.clone()])
            .collect();

        let courts = assign_courts(&games, &CourtHistory::new(), 3);

        let mut used: HashMap<i32, usize> = HashMap::new();
        for court in &courts {
            assert!((1..=3).contains(court), "court {} is not in the venue", court);
            *used.entry(*court).or_default() += 1;
        }
        // Seven games over three courts: 3, 2, 2 - never 4 on one.
        assert_eq!(used.values().copied().max(), Some(3));
    }

    /// Panache identity is the player, not the temporary team they were drawn
    /// into: the team is gone next round, the person is not.
    #[test]
    fn history_is_tracked_per_competitor_not_per_game() {
        let mut history = CourtHistory::new();
        history.record("p1", 2, true);

        let games = vec![
            vec!["p1".to_string(), "p2".to_string()],
            vec!["p3".to_string(), "p4".to_string()],
        ];
        let courts = assign_courts(&games, &history, 2);

        assert_ne!(courts[0], 2, "p1 was sent straight back to court 2");
    }
}
