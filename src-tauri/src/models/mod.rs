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
    pub day_type: String,
    pub number_of_courts: i32,
    pub number_of_qualifying_rounds: i32,
    pub has_consolante: bool,
    pub advance_all: bool,
    pub advance_count: Option<i32>,
    pub bracket_size: i32,
    pub pairing_method: String,
    pub region_avoidance: bool,
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
    pub day_type: String,
    pub number_of_courts: i32,
    pub number_of_qualifying_rounds: i32,
    pub has_consolante: bool,
    pub advance_all: bool,
    pub advance_count: Option<i32>,
    pub bracket_size: i32,
    pub pairing_method: String,
    pub region_avoidance: bool,
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
    pub captain: String,
    pub player2: String,
    pub player3: Option<String>,
    pub region: Option<String>,
    pub club: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTeamData {
    pub tournament_id: String,
    pub captain: String,
    pub player2: String,
    pub player3: Option<String>,
    pub region: Option<String>,
    pub club: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QualifyingRound {
    pub id: String,
    pub tournament_id: String,
    pub round_number: i32,
    pub is_complete: bool,
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
    pub rank: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StandingWithTeam {
    pub id: String,
    pub tournament_id: String,
    pub team_id: String,
    pub wins: i32,
    pub losses: i32,
    pub points_for: i32,
    pub points_against: i32,
    pub differential: i32,
    pub buchholz_score: f64,
    pub rank: i32,
    pub team: Team,
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
    pub team1: Option<Team>,
    pub team2: Option<Team>,
    pub winner: Option<Team>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingHistory {
    pub id: String,
    pub tournament_id: String,
    pub team1_id: String,
    pub team2_id: String,
    pub round_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourtHistory {
    pub id: String,
    pub tournament_id: String,
    pub team_id: String,
    pub court_number: i32,
    pub round_id: String,
}
