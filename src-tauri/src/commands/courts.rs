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
//!
//! A **court plan** (`CourtPlan` in models) adds the venue's own rules, set
//! before play: auxiliary courts brought in for the event, which courts each
//! round can use once string lines come out for the closing rounds, and
//! featured courts for particular bracket matches. They are more costs on the
//! same draw - a plan that cannot be met still produces a round - and a
//! tournament without one draws exactly as it did before plans existed.

use super::brackets;
use crate::db::Database;
use crate::models::CourtPlan;
use chrono::Utc;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use tauri::State;

/// A team drawing the same court in back-to-back rounds - the rule that holds
/// even when the venue is too small to avoid repeats altogether.
const SAME_COURT_IN_A_ROW: i64 = 1000;
/// A team drawing a court it has played on at some earlier point.
const SAME_COURT_AGAIN: i64 = 100;

/// A game on a court its round does not offer, or off the courts its pin
/// names. Nothing else is ever worth this, but it is still a cost rather than a
/// filter: when the plan cannot be met the round is drawn anyway.
const OFF_PLAN: i64 = 10_000_000;
/// Any game on an auxiliary court. Charged per game, and far above what the
/// history adds up to for one game, so the main courts always fill first and
/// the auxiliary courts take only the overflow.
const AUXILIARY_COURT: i64 = 100_000;
/// A game with no pin on a court a pin is holding this round.
const RESERVED_COURT: i64 = 50_000;
/// Per competitor, for each auxiliary game they have already had. It sits
/// between the two repeats: a second auxiliary game is worse than an ordinary
/// repeat but better than the same court twice running. Scaling with the count
/// is what spreads the auxiliary games evenly once some must take a second.
const ANOTHER_AUXILIARY_GAME: i64 = 300;

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
    /// How many games each competitor has played on each court. A count rather
    /// than a set, because spreading the auxiliary courts needs to know how
    /// many a competitor has already had.
    pub ever: HashMap<String, HashMap<i32, u32>>,
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
        *self
            .ever
            .entry(competitor_id.to_string())
            .or_default()
            .entry(court)
            .or_default() += 1;
        if is_previous_round {
            self.previous.insert(competitor_id.to_string(), court);
        }
    }

    /// Games a competitor has played on any of `courts`.
    fn games_on(&self, competitor_id: &str, courts: &HashSet<i32>) -> u32 {
        self.ever.get(competitor_id).map_or(0, |seen| {
            seen.iter()
                .filter(|(court, _)| courts.contains(court))
                .map(|(_, games)| games)
                .sum()
        })
    }

    fn cost_for(&self, competitors: &[String], court: i32, rules: &DrawRules) -> i64 {
        let spreading = rules.spread_auxiliary && rules.auxiliary.contains(&court);
        competitors
            .iter()
            .map(|id| {
                let mut cost = 0;
                if self.previous.get(id) == Some(&court) {
                    cost += SAME_COURT_IN_A_ROW;
                }
                if self.ever.get(id).is_some_and(|seen| seen.contains_key(&court)) {
                    cost += SAME_COURT_AGAIN;
                }
                if spreading {
                    cost += ANOTHER_AUXILIARY_GAME * self.games_on(id, &rules.auxiliary) as i64;
                }
                cost
            })
            .sum()
    }
}

/// What a court plan asks of one draw, on top of the history.
///
/// The default asks nothing, which is the draw as it was before plans.
#[derive(Debug, Default, Clone)]
pub struct DrawRules {
    /// The courts this round offers. Empty means any court in the pool.
    pub available: HashSet<i32>,
    /// Courts brought in for the event: used only when the others run out.
    pub auxiliary: HashSet<i32>,
    /// Charge competitors for auxiliary games they have already played.
    pub spread_auxiliary: bool,
    /// Per game, by index: the courts a pin sends it to. A game past the end
    /// of this list, or with `None`, has no pin.
    pub pinned: Vec<Option<HashSet<i32>>>,
    /// Courts the pins are holding this round; other games keep off them.
    pub reserved: HashSet<i32>,
}

impl DrawRules {
    fn game_cost(&self, game: usize, court: i32) -> i64 {
        let mut cost = 0;
        match self.pinned.get(game).and_then(Option::as_ref) {
            // A pin is the more specific instruction, so it decides on its
            // own - including sending a game to a court the round otherwise
            // does not offer.
            Some(pin) => {
                if !pin.contains(&court) {
                    cost += OFF_PLAN;
                }
            }
            None => {
                if !self.available.is_empty() && !self.available.contains(&court) {
                    cost += OFF_PLAN;
                }
                if self.reserved.contains(&court) {
                    cost += RESERVED_COURT;
                }
            }
        }
        if self.auxiliary.contains(&court) {
            cost += AUXILIARY_COURT;
        }
        cost
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
    debug_assert_eq!(games.len(), slots.len(), "one slot per game");
    assign_courts_from_pool(games, history, slots, &DrawRules::default())
}

/// The draw in full, under a court plan's rules.
///
/// `pool` holds at least one court per game and may hold more: each game takes
/// a distinct entry, and the spare ones simply go unused. A court listed twice
/// can be given to two games, which is how a wave bigger than the venue
/// doubles up.
pub fn assign_courts_from_pool(
    games: &[Vec<String>],
    history: &CourtHistory,
    pool: &[i32],
    rules: &DrawRules,
) -> Vec<i32> {
    if games.is_empty() {
        return Vec::new();
    }
    debug_assert!(pool.len() >= games.len(), "at least one slot per game");

    let cost = |game: usize, court: i32| {
        rules.game_cost(game, court) + history.cost_for(&games[game], court, rules)
    };

    let mut rng = thread_rng();

    let mut best: Option<(i64, Vec<i32>)> = None;

    for _ in 0..RESTARTS {
        let mut order: Vec<usize> = (0..games.len()).collect();
        order.shuffle(&mut rng);

        let mut spare = pool.to_vec();
        spare.shuffle(&mut rng);

        let mut courts = vec![0; games.len()];
        for &game in &order {
            // Cheapest remaining court for this game. The pool was shuffled, so
            // equally good courts are taken in no particular order rather than
            // always the lowest-numbered one.
            let pick = spare
                .iter()
                .enumerate()
                .min_by_key(|(_, &court)| cost(game, court))
                .map(|(idx, _)| idx)
                .expect("pool holds a slot for every game");
            courts[game] = spare.swap_remove(pick);
        }

        improve(&mut courts, &mut spare, &cost);

        let total: i64 = courts.iter().enumerate().map(|(game, &court)| cost(game, court)).sum();
        if best.as_ref().is_none_or(|(b, _)| total < *b) {
            best = Some((total, courts));
        }
        if total == 0 {
            break;
        }
    }

    best.map(|(_, courts)| courts).unwrap_or_default()
}

/// Swaps pairs of games between their courts, and moves games onto courts
/// nobody took, for as long as that helps.
///
/// The greedy pass commits to a court before it has seen the games that come
/// after, so it regularly leaves a swap on the table that costs nothing to take.
/// `spare` is empty unless the pool held more courts than games.
fn improve(courts: &mut [i32], spare: &mut [i32], cost: &impl Fn(usize, i32) -> i64) {
    for _ in 0..MAX_SWEEPS {
        let mut improved = false;

        for a in 0..courts.len() {
            for b in (a + 1)..courts.len() {
                if courts[a] == courts[b] {
                    continue;
                }
                let before = cost(a, courts[a]) + cost(b, courts[b]);
                let after = cost(a, courts[b]) + cost(b, courts[a]);
                if after < before {
                    courts.swap(a, b);
                    improved = true;
                }
            }
            for slot in spare.iter_mut() {
                if cost(a, *slot) < cost(a, courts[a]) {
                    std::mem::swap(&mut courts[a], slot);
                    improved = true;
                }
            }
        }

        if !improved {
            break;
        }
    }
}

/// The slots one round draws from, when a plan says which courts it offers.
///
/// Every offered court is a candidate, so the draw is free to spread a small
/// field across a large venue. Only a round with more games than courts wraps,
/// and then the main courts take the extra games before the auxiliary ones.
pub fn plan_slots(game_count: usize, offered: &[i32], auxiliary: &HashSet<i32>) -> Vec<i32> {
    if game_count <= offered.len() {
        return offered.to_vec();
    }
    let mut ordered: Vec<i32> = offered.iter().copied().filter(|c| !auxiliary.contains(c)).collect();
    ordered.extend(offered.iter().copied().filter(|c| auxiliary.contains(c)));
    (0..game_count).map(|i| ordered[i % ordered.len()]).collect()
}

/// Draws one qualifying round's courts, under the plan where there is one.
pub fn assign_round_courts(
    games: &[Vec<String>],
    history: &CourtHistory,
    number_of_courts: i32,
    plan: Option<&ResolvedPlan>,
    round_number: i32,
) -> Vec<i32> {
    let Some(plan) = plan else {
        return assign_courts(games, history, number_of_courts);
    };
    let offered = plan.qualifying_courts(round_number);
    let rules = plan.rules(offered, Vec::new(), HashSet::new());
    assign_courts_from_pool(games, history, &plan_slots(games.len(), offered, &plan.auxiliary), &rules)
}

// ---------------------------------------------------------------------------
// The court plan
// ---------------------------------------------------------------------------

/// Reads a court list the way a print dialog reads a page range:
/// `1,3,5,6-11, 15-20`. Spaces are ignored, duplicates collapse, and the result
/// comes back sorted. A blank list is an empty one.
pub fn parse_court_list(text: &str, number_of_courts: i32) -> Result<Vec<i32>, String> {
    let number = |token: &str| -> Result<i32, String> {
        // Digits only: `parse` alone would take "+5", which the preview in
        // `courtRanges.ts` rejects, and the two have to agree.
        let court: i32 = Some(token)
            .filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()))
            .and_then(|t| t.parse().ok())
            .ok_or_else(|| format!("\"{}\" is not a court number", token))?;
        if court < 1 {
            return Err("Court numbers start at 1".to_string());
        }
        if court > number_of_courts {
            return Err(format!(
                "Court {} is beyond the {} courts this tournament has",
                court, number_of_courts
            ));
        }
        Ok(court)
    };

    let mut courts = Vec::new();
    for token in text.split(',') {
        let token: String = token.chars().filter(|c| !c.is_whitespace()).collect();
        if token.is_empty() {
            continue;
        }
        // An en dash is what a range turns into when it is pasted from a
        // document, so it reads the same as a hyphen.
        match token.replace('\u{2013}', "-").split_once('-') {
            Some((from, to)) => {
                let (from, to) = (number(from)?, number(to)?);
                if from > to {
                    return Err(format!("{}-{} runs backwards", from, to));
                }
                courts.extend(from..=to);
            }
            None => courts.push(number(&token)?),
        }
    }
    courts.sort_unstable();
    courts.dedup();
    Ok(courts)
}

/// Whether a plan asks for anything at all. One that does not is treated as no
/// plan, so saving an empty form leaves the draw exactly as it was.
pub fn plan_is_empty(plan: &CourtPlan) -> bool {
    plan.auxiliary.trim().is_empty()
        && plan.qualifying_rounds.iter().all(|row| row.trim().is_empty())
        && plan.elimination_rounds.iter().all(|row| row.trim().is_empty())
        && plan.pins.is_empty()
}

/// Checks every field of a plan before it is saved, naming the one at fault.
pub fn validate_court_plan(plan: &CourtPlan, number_of_courts: i32) -> Result<(), String> {
    let check = |label: String, text: &str| {
        parse_court_list(text, number_of_courts).map_err(|e| format!("{}: {}", label, e))
    };

    let auxiliary = check("Auxiliary courts".to_string(), &plan.auxiliary)?;
    if !auxiliary.is_empty() && auxiliary.len() as i32 >= number_of_courts {
        return Err("Auxiliary courts: at least one court has to be a main court".to_string());
    }
    for (i, row) in plan.qualifying_rounds.iter().enumerate() {
        check(format!("Qualifying round {}", i + 1), row)?;
    }
    for (i, row) in plan.elimination_rounds.iter().enumerate() {
        check(format!("Elimination round {}", i + 1), row)?;
    }
    let mut pinned = HashSet::new();
    for pin in &plan.pins {
        let bracket = pin.bracket.trim().to_uppercase();
        if bracket.is_empty() {
            return Err("Featured courts: every row needs a bracket".to_string());
        }
        let stage = match pin.stage {
            0 => "final",
            1 => "semifinals",
            2 => "quarterfinals",
            _ => return Err("Featured courts: unknown round".to_string()),
        };
        let label = format!("Featured courts for the {} {}", bracket, stage);
        if !pinned.insert((bracket, pin.stage)) {
            return Err(format!("{}: listed twice", label));
        }
        // One court per match in that round: a court short and one match has
        // nowhere to go, a court over and a court is held for nobody.
        let matches = 1usize << pin.stage;
        let courts = check(label.clone(), &pin.courts)?.len();
        if courts != matches {
            return Err(format!(
                "{}: {} court{} needed, one per match, and {} given",
                label,
                matches,
                if matches == 1 { "" } else { "s" },
                courts
            ));
        }
    }
    Ok(())
}

/// A plan with its court lists parsed and its blank rows filled in.
///
/// Built leniently: anything that no longer parses - a court above a court
/// count that has since been lowered - is dropped rather than failing the draw
/// in the middle of an event. Saving is where a plan is checked strictly.
#[derive(Debug, Clone)]
pub struct ResolvedPlan {
    pub auxiliary: HashSet<i32>,
    pub spread_auxiliary: bool,
    /// Courts offered per qualifying round, by position, blanks carried forward.
    qualifying: Vec<Vec<i32>>,
    /// The same per elimination wave, carrying on from the last qualifying row.
    elimination: Vec<Vec<i32>>,
    /// The last row of all, for a wave past the end of the list.
    last: Vec<i32>,
    /// Every court, for a qualifying round when no qualifying row is listed.
    venue: Vec<i32>,
    pins: Vec<(String, i32, HashSet<i32>)>,
}

impl ResolvedPlan {
    pub fn new(plan: &CourtPlan, number_of_courts: i32) -> Self {
        let lenient = |text: &str| -> Vec<i32> {
            parse_court_list(text, i32::MAX)
                .unwrap_or_default()
                .into_iter()
                .filter(|&court| court <= number_of_courts)
                .collect()
        };

        // One timeline from the first qualifying round to the last wave: a
        // blank row is the row before it, because a string line that has
        // come out stays out. A blank first row is the whole venue.
        let venue: Vec<i32> = (1..=number_of_courts.max(1)).collect();
        let mut current = venue.clone();
        let mut carry = |rows: &[String]| -> Vec<Vec<i32>> {
            rows.iter()
                .map(|row| {
                    let courts = lenient(row);
                    if !courts.is_empty() {
                        current = courts;
                    }
                    current.clone()
                })
                .collect()
        };
        let qualifying = carry(&plan.qualifying_rounds);
        let elimination = carry(&plan.elimination_rounds);

        Self {
            auxiliary: lenient(&plan.auxiliary).into_iter().collect(),
            spread_auxiliary: plan.spread_auxiliary,
            qualifying,
            elimination,
            last: current,
            venue,
            pins: plan
                .pins
                .iter()
                .map(|pin| {
                    let courts: HashSet<i32> = lenient(&pin.courts).into_iter().collect();
                    (pin.bracket.trim().to_uppercase(), pin.stage, courts)
                })
                .filter(|(_, _, courts)| !courts.is_empty())
                .collect(),
        }
    }

    /// The courts qualifying round `round_number` (from 1) can use. A round
    /// past the listed ones keeps the last qualifying row.
    pub fn qualifying_courts(&self, round_number: i32) -> &[i32] {
        let index = (round_number.max(1) - 1) as usize;
        self.qualifying
            .get(index)
            .or(self.qualifying.last())
            .map_or(&self.venue, Vec::as_slice)
    }

    /// The courts elimination wave `wave` (from 1, as `brackets::play_wave`
    /// counts) can use. A wave past the listed ones keeps the last row.
    pub fn wave_courts(&self, wave: i32) -> &[i32] {
        let index = (wave.max(1) - 1) as usize;
        self.elimination.get(index).map_or(&self.last, Vec::as_slice)
    }

    /// The courts pinned for one bracket's match `stage` rounds from its end.
    pub fn pin_for(&self, bracket: &str, stage: i32) -> Option<&HashSet<i32>> {
        let bracket = bracket.trim().to_uppercase();
        self.pins
            .iter()
            .find(|(name, s, _)| *name == bracket && *s == stage)
            .map(|(_, _, courts)| courts)
    }

    /// The draw rules for one round that offers `offered`.
    pub fn rules(
        &self,
        offered: &[i32],
        pinned: Vec<Option<HashSet<i32>>>,
        reserved: HashSet<i32>,
    ) -> DrawRules {
        DrawRules {
            available: offered.iter().copied().collect(),
            auxiliary: self.auxiliary.clone(),
            spread_auxiliary: self.spread_auxiliary,
            pinned,
            reserved,
        }
    }
}

/// The tournament's court plan, ready to draw with. `None` when it has none -
/// or only an empty one - which is the draw as it always was.
pub fn load_court_plan(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<Option<ResolvedPlan>, String> {
    let Some(plan) = read_court_plan(conn, tournament_id)? else {
        return Ok(None);
    };
    if plan_is_empty(&plan) {
        return Ok(None);
    }
    let number_of_courts: i32 = conn
        .query_row(
            "SELECT number_of_courts FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok(Some(ResolvedPlan::new(&plan, number_of_courts)))
}

/// The plan as the operator entered it, if one has been saved.
pub fn read_court_plan(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<Option<CourtPlan>, String> {
    let text: Option<String> = conn
        .query_row(
            "SELECT plan FROM court_plans WHERE tournament_id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    text.map(|text| {
        serde_json::from_str(&text).map_err(|e| format!("The saved court plan could not be read: {}", e))
    })
    .transpose()
}

/// Stores a plan, or removes it when it asks for nothing.
pub fn write_court_plan(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    plan: &CourtPlan,
) -> Result<(), String> {
    if plan_is_empty(plan) {
        conn.execute("DELETE FROM court_plans WHERE tournament_id = ?1", params![tournament_id])
            .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let text = serde_json::to_string(plan).map_err(|e| e.to_string())?;
    conn.execute(
        r#"
        INSERT INTO court_plans (tournament_id, plan, updated_at) VALUES (?1, ?2, ?3)
        ON CONFLICT(tournament_id) DO UPDATE SET plan = excluded.plan, updated_at = excluded.updated_at
        "#,
        params![tournament_id, text, Utc::now().to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn get_court_plan(db: State<Database>, tournament_id: String) -> Result<CourtPlan, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(read_court_plan(&conn, &tournament_id)?.unwrap_or_default())
}

/// The courts the plan offers qualifying round `round_number`, or `None` when
/// there is no plan. The rounds view highlights a game moved off them, the same
/// way it highlights two games on one court.
#[tauri::command]
pub fn get_round_court_offer(
    db: State<Database>,
    tournament_id: String,
    round_number: i32,
) -> Result<Option<Vec<i32>>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    Ok(load_court_plan(&conn, &tournament_id)?
        .map(|plan| plan.qualifying_courts(round_number).to_vec()))
}

/// Saves the plan and re-draws every bracket wave that has not started, so the
/// change reaches the courts straight away. Qualifying rounds already drawn
/// keep their courts: they may be printed and on the ground.
#[tauri::command]
pub fn save_court_plan(
    db: State<Database>,
    tournament_id: String,
    plan: CourtPlan,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    save_court_plan_inner(&conn, &tournament_id, &plan)
}

pub(crate) fn save_court_plan_inner(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    plan: &CourtPlan,
) -> Result<(), String> {
    let number_of_courts: i32 = conn
        .query_row(
            "SELECT number_of_courts FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    validate_court_plan(plan, number_of_courts)?;
    if !plan_is_empty(plan) {
        check_court_capacity(plan, number_of_courts, &court_demand(conn, tournament_id)?)?;
    }
    write_court_plan(conn, tournament_id, plan)?;
    brackets::assign_bracket_courts(conn, tournament_id)
}

// ---------------------------------------------------------------------------
// Whether the plan has room for the field
// ---------------------------------------------------------------------------

/// How many games each round of the tournament will need a court for, as far
/// as can be told today, and the brackets a featured court can name.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourtDemand {
    /// Games per qualifying round, from round 1. A round already drawn has its
    /// courts and asks for none; neither does one whose size depends on
    /// results not yet in.
    pub qualifying: Vec<i32>,
    /// Matches per elimination wave, from wave 1.
    pub elimination: Vec<brackets::WaveDemand>,
    /// How many elimination rows the plan should show.
    pub wave_count: i32,
    pub brackets: Vec<String>,
}

pub(crate) fn court_demand(
    conn: &rusqlite::Connection,
    tournament_id: &str,
) -> Result<CourtDemand, String> {
    let (number_of_courts, pairing_method, format, rounds): (i32, String, String, i32) = conn
        .query_row(
            "SELECT number_of_courts, pairing_method, format, number_of_qualifying_rounds \
             FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )
        .map_err(|e| e.to_string())?;
    let entrants: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM teams WHERE tournament_id = ?1 AND is_withdrawn = 0",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let drawn: HashSet<i32> = conn
        .prepare("SELECT round_number FROM qualifying_rounds WHERE tournament_id = ?1 AND is_final = 0")
        .and_then(|mut stmt| {
            stmt.query_map(params![tournament_id], |row| row.get(0))?
                .collect::<Result<_, _>>()
        })
        .map_err(|e| e.to_string())?;

    let is_panache = pairing_method == "panache";
    // A panache game seats both temporary teams, and the surplus sits out; any
    // other format gives an odd team a bye, which still takes a court.
    let games = if is_panache {
        let team_size = if format == "triple" { 3 } else { 2 };
        entrants / (2 * team_size)
    } else {
        (entrants + 1) / 2
    };
    let qualifying = (1..=rounds)
        .map(|round| {
            // Pool play's third round is only the teams on one win and one
            // loss, which nobody knows until round two is in.
            if drawn.contains(&round) || (pairing_method == "poolPlay" && round >= 3) {
                0
            } else {
                games
            }
        })
        .collect();

    if is_panache {
        return Ok(CourtDemand { qualifying, elimination: Vec::new(), wave_count: 0, brackets: Vec::new() });
    }
    // The roster's ceiling, as `check_roster_capacity` sets it: two teams a court.
    let demand = brackets::bracket_demand(conn, tournament_id, entrants, number_of_courts * 2)?;
    Ok(CourtDemand {
        qualifying,
        elimination: demand.waves,
        wave_count: demand.wave_count,
        brackets: demand.names,
    })
}

/// Refuses a plan that offers a round fewer courts than it has games: the
/// draw would have to put two games on one court.
pub fn check_court_capacity(
    plan: &CourtPlan,
    number_of_courts: i32,
    demand: &CourtDemand,
) -> Result<(), String> {
    let resolved = ResolvedPlan::new(plan, number_of_courts);
    let short = |label: String, games: i32, courts: usize| {
        Err(format!(
            "{}: {} games need a court, and only {} {} offered",
            label,
            games,
            courts,
            if courts == 1 { "is" } else { "are" }
        ))
    };
    for (i, &games) in demand.qualifying.iter().enumerate() {
        let round = i as i32 + 1;
        let courts = resolved.qualifying_courts(round).len();
        if games as usize > courts {
            return short(format!("Qualifying round {}", round), games, courts);
        }
    }
    for (i, wave) in demand.elimination.iter().enumerate() {
        let number = i as i32 + 1;
        // A featured court counts even off the row: the draw offers it too.
        let mut courts: HashSet<i32> = resolved.wave_courts(number).iter().copied().collect();
        for stage in &wave.stages {
            courts.extend(resolved.pin_for(&stage.bracket, stage.stage).into_iter().flatten());
        }
        if wave.matches as usize > courts.len() {
            return short(format!("Elimination round {}", number), wave.matches, courts.len());
        }
    }
    Ok(())
}

#[tauri::command]
pub fn get_court_demand(db: State<Database>, tournament_id: String) -> Result<CourtDemand, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    court_demand(&conn, &tournament_id)
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

    // -----------------------------------------------------------------------
    // Court plans
    // -----------------------------------------------------------------------

    fn games_for(n: usize) -> Vec<Vec<String>> {
        pairs_for(n).into_iter().map(|(a, b)| vec![a, b]).collect()
    }

    fn plan(auxiliary: &str, spread: bool) -> CourtPlan {
        CourtPlan {
            auxiliary: auxiliary.to_string(),
            spread_auxiliary: spread,
            ..CourtPlan::default()
        }
    }

    fn rows(rows: &[&str]) -> Vec<String> {
        rows.iter().map(|r| r.to_string()).collect()
    }

    #[test]
    fn court_lists_read_like_a_print_dialog() {
        assert_eq!(
            parse_court_list("1,3,5,6-11,15-20", 20).unwrap(),
            vec![1, 3, 5, 6, 7, 8, 9, 10, 11, 15, 16, 17, 18, 19, 20]
        );
        assert_eq!(parse_court_list(" 4 , 2-3 ,2, ", 10).unwrap(), vec![2, 3, 4]);
        assert_eq!(parse_court_list("1\u{2013}3", 10).unwrap(), vec![1, 2, 3]);
        assert_eq!(parse_court_list("   ", 10).unwrap(), Vec::<i32>::new());

        assert!(parse_court_list("6-2", 10).unwrap_err().contains("backwards"));
        assert!(parse_court_list("0", 10).is_err());
        assert!(parse_court_list("3,x", 10).unwrap_err().contains("\"x\""));
        assert!(parse_court_list("11", 10).unwrap_err().contains("beyond the 10 courts"));
        assert!(parse_court_list("8-12", 10).is_err());
    }

    #[test]
    fn saving_names_the_field_at_fault() {
        let mut bad = plan("", false);
        bad.qualifying_rounds = rows(&["", "1-60"]);
        let err = validate_court_plan(&bad, 52).unwrap_err();
        assert!(err.starts_with("Qualifying round 2"), "{}", err);

        // Every court auxiliary leaves no main court to prefer.
        assert!(validate_court_plan(&plan("1-10", false), 10).is_err());

        let mut pinless = plan("", false);
        pinless.pins.push(crate::models::CourtPin {
            bracket: "A".to_string(),
            stage: 0,
            courts: String::new(),
        });
        assert!(validate_court_plan(&pinless, 10).is_err());

        let pin = |bracket: &str, stage: i32, courts: &str| crate::models::CourtPin {
            bracket: bracket.to_string(),
            stage,
            courts: courts.to_string(),
        };
        let pinned = |pins| crate::models::CourtPlan {
            pins,
            ..Default::default()
        };
        // One court per match: the final takes one, the semifinals two.
        assert!(validate_court_plan(&pinned(vec![pin("A", 0, "1"), pin("A", 1, "1,3")]), 10).is_ok());
        assert!(validate_court_plan(&pinned(vec![pin("A", 0, "1,3")]), 10).is_err());
        assert!(validate_court_plan(&pinned(vec![pin("A", 1, "1")]), 10).is_err());
        assert!(validate_court_plan(&pinned(vec![pin("A", 1, "3,3")]), 10).is_err());
        assert!(validate_court_plan(&pinned(vec![pin("A", 2, "1-4")]), 10).is_ok());
        // The same match twice, however the bracket is spelled.
        let err = validate_court_plan(&pinned(vec![pin("A", 0, "1"), pin("a", 0, "2")]), 10).unwrap_err();
        assert!(err.contains("listed twice"), "{}", err);
        assert!(validate_court_plan(&pinned(vec![pin("A", 0, "1"), pin("B", 0, "2")]), 10).is_ok());
        assert!(validate_court_plan(&pinned(vec![pin("A", 3, "1-8")]), 10).is_err());

        assert!(validate_court_plan(&plan("41-52", true), 52).is_ok());
    }

    /// Blank rows are the row before, across the whole day: a string line that
    /// came out for the semifinals is still out for the final.
    #[test]
    fn blank_rows_carry_the_round_before_forward() {
        let mut p = plan("", false);
        p.qualifying_rounds = rows(&["", "", "1-8"]);
        p.elimination_rounds = rows(&["", "1-6", "", "1,3"]);
        let resolved = ResolvedPlan::new(&p, 10);

        let all: Vec<i32> = (1..=10).collect();
        assert_eq!(resolved.qualifying_courts(1), all.as_slice());
        assert_eq!(resolved.qualifying_courts(3), &[1, 2, 3, 4, 5, 6, 7, 8]);
        // A round past the listed rows keeps the last one.
        assert_eq!(resolved.qualifying_courts(5), &[1, 2, 3, 4, 5, 6, 7, 8]);
        // The first elimination row is blank, so it carries on from qualifying.
        assert_eq!(resolved.wave_courts(1), &[1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(resolved.wave_courts(3), &[1, 2, 3, 4, 5, 6]);
        assert_eq!(resolved.wave_courts(4), &[1, 3]);
        assert_eq!(resolved.wave_courts(9), &[1, 3]);
    }

    /// A court count lowered after the plan was saved must not stop the draw:
    /// the courts that no longer exist simply drop out.
    #[test]
    fn a_plan_outliving_its_courts_still_draws() {
        let mut p = plan("9-12", true);
        p.qualifying_rounds = rows(&["1-12"]);
        let resolved = ResolvedPlan::new(&p, 10);
        assert_eq!(resolved.qualifying_courts(1), &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        assert_eq!(resolved.auxiliary, HashSet::from([9, 10]));
    }

    #[test]
    fn an_empty_plan_is_no_plan() {
        let mut p = plan("  ", true);
        p.qualifying_rounds = rows(&["", " "]);
        assert!(plan_is_empty(&p));
        p.pins.push(crate::models::CourtPin::default());
        assert!(!plan_is_empty(&p));
    }

    /// The auxiliary courts sit idle while the main courts have room.
    #[test]
    fn auxiliary_courts_go_unused_until_they_are_needed() {
        let resolved = ResolvedPlan::new(&plan("41-52", true), 52);
        for _ in 0..20 {
            let courts = assign_round_courts(&games_for(30), &CourtHistory::new(), 52, Some(&resolved), 1);
            assert!(courts.iter().all(|c| *c <= 40), "an auxiliary court was used: {:?}", courts);
            let unique: HashSet<i32> = courts.iter().copied().collect();
            assert_eq!(unique.len(), 30, "courts double-booked: {:?}", courts);
        }
    }

    /// And once they are, they take the overflow and nothing more.
    #[test]
    fn auxiliary_courts_take_only_the_overflow() {
        let resolved = ResolvedPlan::new(&plan("41-52", true), 52);
        for _ in 0..20 {
            let courts = assign_round_courts(&games_for(45), &CourtHistory::new(), 52, Some(&resolved), 1);
            let auxiliary = courts.iter().filter(|c| **c > 40).count();
            assert_eq!(auxiliary, 5, "{:?}", courts);
            let unique: HashSet<i32> = courts.iter().copied().collect();
            assert_eq!(unique.len(), 45, "courts double-booked: {:?}", courts);
        }
    }

    /// Ten fixed games on eight main and two auxiliary courts, over five
    /// rounds: two games a round have to go to the auxiliary courts, and there
    /// are exactly enough rounds for every game to take its turn once. That is
    /// reachable, so it is what is asserted.
    #[test]
    fn auxiliary_games_are_spread_across_the_field() {
        let resolved = ResolvedPlan::new(&plan("9-10", true), 10);
        let games = games_for(10);

        for _ in 0..20 {
            let mut history = CourtHistory::new();
            let mut on_auxiliary: HashMap<String, u32> = HashMap::new();

            for round in 1..=5 {
                let courts = assign_round_courts(&games, &history, 10, Some(&resolved), round);
                history.previous.clear();
                for (game, court) in games.iter().zip(&courts) {
                    for id in game {
                        history.record(id, *court, true);
                        if *court > 8 {
                            *on_auxiliary.entry(id.clone()).or_default() += 1;
                        }
                    }
                }
            }

            for (team, count) in &on_auxiliary {
                assert_eq!(*count, 1, "{} played {} games on the auxiliary courts", team, count);
            }
            assert_eq!(on_auxiliary.len(), 20);
        }
    }

    /// The ordering the director chose: when one game must either take a second
    /// auxiliary game or go back to the court it has just left, the second
    /// auxiliary game is the lesser evil.
    #[test]
    fn a_second_auxiliary_game_is_preferred_to_the_same_court_twice_running() {
        let resolved = ResolvedPlan::new(&plan("2", true), 2);

        let mut history = CourtHistory::new();
        // X played on auxiliary court 2 once, then on court 1 last round.
        for id in ["x1", "x2"] {
            history.record(id, 2, false);
            history.record(id, 1, true);
        }
        let games = vec![
            vec!["x1".to_string(), "x2".to_string()],
            vec!["y1".to_string(), "y2".to_string()],
        ];

        for _ in 0..20 {
            let courts = assign_round_courts(&games, &history, 2, Some(&resolved), 1);
            assert_eq!(courts[0], 2, "X went straight back to court 1 to dodge a second auxiliary game");
        }
    }

    /// Without the spreading rule the auxiliary courts are still filled last,
    /// but nobody is kept off them for having played there.
    #[test]
    fn spreading_is_the_operators_choice() {
        let resolved = ResolvedPlan::new(&plan("2", false), 2);
        let rules = resolved.rules(resolved.qualifying_courts(1), Vec::new(), HashSet::new());
        let mut history = CourtHistory::new();
        history.record("x1", 2, false);
        assert_eq!(history.cost_for(&["x1".to_string()], 2, &rules), SAME_COURT_AGAIN);
    }

    /// A pin sends its game to the named court, and the other games in the
    /// round keep off the courts pins are holding while they have anywhere
    /// else to go.
    #[test]
    fn pins_take_their_courts_and_hold_them() {
        let games = games_for(3);
        let pinned = vec![Some(HashSet::from([1])), Some(HashSet::from([1, 3])), None];
        let rules = DrawRules {
            available: (1..=6).collect(),
            pinned,
            reserved: HashSet::from([1, 3]),
            ..DrawRules::default()
        };
        for _ in 0..20 {
            let courts = assign_courts_from_pool(&games, &CourtHistory::new(), &[1, 2, 3, 4, 5, 6], &rules);
            assert_eq!(courts[0], 1);
            assert_eq!(courts[1], 3);
            assert!(![1, 3].contains(&courts[2]), "an unpinned game took a held court: {:?}", courts);
        }
    }

    /// A pin is the more specific instruction: it can send a game to a court
    /// the round otherwise does not offer, and nobody else follows it there.
    #[test]
    fn a_pin_overrides_the_rounds_courts() {
        let games = games_for(2);
        let rules = DrawRules {
            available: HashSet::from([1, 3]),
            pinned: vec![Some(HashSet::from([2]))],
            reserved: HashSet::from([2]),
            ..DrawRules::default()
        };
        for _ in 0..20 {
            let courts = assign_courts_from_pool(&games, &CourtHistory::new(), &[1, 2, 3], &rules);
            assert_eq!(courts[0], 2);
            assert_ne!(courts[1], 2);
        }
    }

    /// Courts a round does not offer are left alone even when they would
    /// spare a team a repeat.
    #[test]
    fn a_round_keeps_to_the_courts_it_offers() {
        let mut p = plan("", false);
        p.qualifying_rounds = rows(&["1,3,5"]);
        let resolved = ResolvedPlan::new(&p, 6);

        let mut history = CourtHistory::new();
        for id in ["A0", "B0", "A1", "B1", "A2", "B2"] {
            for court in [1, 3, 5] {
                history.record(id, court, court == 1);
            }
        }
        for _ in 0..20 {
            let courts = assign_round_courts(&games_for(3), &history, 6, Some(&resolved), 1);
            assert!(courts.iter().all(|c| [1, 3, 5].contains(c)), "{:?}", courts);
        }
    }

    /// A round bigger than its courts doubles up, main courts first.
    #[test]
    fn an_oversubscribed_round_doubles_up_on_the_main_courts() {
        let slots = plan_slots(5, &[1, 2, 3, 4], &HashSet::from([4]));
        assert_eq!(slots.len(), 5);
        assert_eq!(slots.iter().filter(|c| **c == 4).count(), 1);
    }

    /// A qualifying round offered fewer courts than it has games is refused,
    /// and a blank row is judged by the row it carries forward.
    #[test]
    fn a_round_offered_too_few_courts_is_refused() {
        let demand = CourtDemand {
            qualifying: vec![26, 26, 0],
            elimination: Vec::new(),
            wave_count: 0,
            brackets: Vec::new(),
        };
        let plan = |rows: &[&str]| CourtPlan {
            qualifying_rounds: rows.iter().map(|r| r.to_string()).collect(),
            ..Default::default()
        };
        assert!(check_court_capacity(&plan(&["", "1-26", ""]), 52, &demand).is_ok());
        let err = check_court_capacity(&plan(&["1-30", "1-20", ""]), 52, &demand).unwrap_err();
        assert!(err.starts_with("Qualifying round 2"), "{}", err);
        // Round 3 asks for nothing, so the row it carries is not judged.
        assert!(check_court_capacity(&plan(&["", "", "1-2"]), 52, &demand).is_ok());
        // Round 1 is blank, so it is the whole venue - too small for 26 games.
        assert!(check_court_capacity(&plan(&[""]), 20, &demand).is_err());
    }
}
