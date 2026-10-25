use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tournament {
    pub id: String,
    pub name: String,
    pub team_composition: String,
    #[serde(rename = "type")]
    pub tournament_type: String,
    pub start_date: String,
    pub end_date: String,
    pub director: String,
    pub head_umpire: String,
    pub format: String,
    pub number_of_courts: i32,
    pub number_of_qualifying_rounds: i32,
    pub has_consolante: bool,
    pub advance_all: bool,
    pub advance_count: Option<i32>,
    pub bracket_size: i32,
    pub pairing_method: String,
    pub region_avoidance: bool,
    /// Optional tournament logo as a data URI, printed top-right on every PDF.
    #[serde(default)]
    pub logo: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTournamentData {
    pub name: String,
    pub team_composition: String,
    #[serde(rename = "type")]
    pub tournament_type: String,
    pub start_date: String,
    pub end_date: String,
    pub director: String,
    pub head_umpire: String,
    pub additional_umpires: Option<Vec<String>>,
    pub format: String,
    pub number_of_courts: i32,
    pub number_of_qualifying_rounds: i32,
    pub has_consolante: bool,
    pub advance_all: bool,
    pub advance_count: Option<i32>,
    pub bracket_size: i32,
    pub pairing_method: String,
    pub region_avoidance: bool,
    #[serde(default)]
    pub logo: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Umpire {
    pub id: String,
    pub tournament_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: String,
    pub tournament_id: String,
    pub team_number: i32,
    pub captain: String,
    pub player2: String,
    pub player3: Option<String>,
    pub region: Option<String>,
    pub club: Option<String>,
    /// Panache only: an expert the draw keeps off the same team as other champions.
    pub is_champion: bool,
    /// Pulled out mid-tournament: kept for the games already played, skipped by
    /// every draw from here on. Defaulted so older backups still restore.
    #[serde(default)]
    pub is_withdrawn: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamData {
    pub tournament_id: String,
    pub team_number: Option<i32>,
    pub captain: String,
    pub player2: String,
    pub player3: Option<String>,
    pub region: Option<String>,
    pub club: Option<String>,
    /// None means "leave as-is" on update, and "not a champion" on create.
    pub is_champion: Option<bool>,
    /// None means "leave as-is" on update, and "still playing" on create.
    pub is_withdrawn: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualifyingRound {
    pub id: String,
    pub tournament_id: String,
    pub round_number: i32,
    pub is_complete: bool,
    /// Panache only: the single championship game played after the qualifying rounds.
    pub is_final: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualifyingGame {
    pub id: String,
    pub round_id: String,
    pub court_number: i32,
    pub team1_id: Option<String>,
    pub team2_id: Option<String>,
    pub team1_score: Option<i32>,
    pub team2_score: Option<i32>,
    pub is_bye: bool,
    /// Panache only: for these games team1_id/team2_id are NULL and the two sides
    /// are temporary teams drawn for this round.
    pub side1_id: Option<String>,
    pub side2_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GameWithTeams {
    pub id: String,
    pub round_id: String,
    pub court_number: i32,
    pub team1_id: Option<String>,
    pub team2_id: Option<String>,
    pub team1_score: Option<i32>,
    pub team2_score: Option<i32>,
    pub is_bye: bool,
    pub team1: Option<Team>,
    pub team2: Option<Team>,
    /// Panache only; null for every other format, so existing consumers are unaffected.
    pub side1: Option<PanacheSide>,
    pub side2: Option<PanacheSide>,
}

/// One temporary team in a panache game, with its members resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanacheSide {
    pub id: String,
    pub team_index: i32,
    pub members: Vec<Team>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamStanding {
    pub id: String,
    pub tournament_id: String,
    pub team_id: String,
    pub wins: i32,
    pub losses: i32,
    pub points_for: i32,
    pub points_against: i32,
    pub differential: i32,
    pub buchholz_score: f64,
    pub fine_buchholz_score: f64,
    pub point_quotient: f64,
    pub is_eliminated: bool,
    pub rank: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bracket {
    pub id: String,
    pub tournament_id: String,
    pub name: String,
    pub is_consolante: bool,
    pub size: i32,
    pub is_complete: bool,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BracketMatch {
    pub id: String,
    pub bracket_id: String,
    pub round_number: i32,
    pub match_number: i32,
    pub court_number: Option<i32>,
    pub team1_id: Option<String>,
    pub team2_id: Option<String>,
    pub team1_score: Option<i32>,
    pub team2_score: Option<i32>,
    pub winner_id: Option<String>,
    pub next_match_id: Option<String>,
    pub is_bye: bool,
    /// The court was set by hand, so the automatic renumbering leaves it alone.
    #[serde(default)]
    pub court_is_manual: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchWithTeams {
    pub id: String,
    pub bracket_id: String,
    pub round_number: i32,
    pub match_number: i32,
    pub court_number: Option<i32>,
    pub team1_id: Option<String>,
    pub team2_id: Option<String>,
    pub team1_score: Option<i32>,
    pub team2_score: Option<i32>,
    pub winner_id: Option<String>,
    pub next_match_id: Option<String>,
    pub is_bye: bool,
    #[serde(default)]
    pub court_is_manual: bool,
    pub team1: Option<Team>,
    pub team2: Option<Team>,
    pub winner: Option<Team>,
}


/// The operator's court rules, written before play and read by every draw.
///
/// Court lists are kept as the text the operator typed - "1,3,5,6-11" - and
/// parsed by `courts::parse_court_list` when a round is drawn, so the plan reads
/// back exactly as it was entered. Every field defaults, which is what lets a
/// plan saved by an older build, or a backup without one, load at all.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CourtPlan {
    /// Courts added for the event: used only when the main courts run out.
    pub auxiliary: String,
    /// Keep any one competitor from playing more than one game on them.
    pub spread_auxiliary: bool,
    /// Courts available in each qualifying round, by position. A blank entry
    /// means "the same as the round before".
    pub qualifying_rounds: Vec<String>,
    /// The same for each elimination wave, carrying on from the qualifying rows.
    pub elimination_rounds: Vec<String>,
    /// Bracket matches sent to particular courts.
    pub pins: Vec<CourtPin>,
}

/// One featured court: "bracket A's final goes on court 1".
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CourtPin {
    /// The bracket's name as it is drawn: "A", "B", "AA".
    pub bracket: String,
    /// Rounds from the end of that bracket: 0 the final, 1 the semifinals, 2 the
    /// quarterfinals. Counted from the end so one pin works for a bracket of
    /// any size.
    pub stage: i32,
    pub courts: String,
}
