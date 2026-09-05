//! Whole-tournament backup and restore.
//!
//! The backup is assembled here rather than in the frontend so that it covers
//! every table a tournament touches. The earlier frontend-assembled version
//! quietly omitted umpires, pairing and court history, and all three panache
//! tables, which made "Full Tournament Backup" a promise it did not keep.
//!
//! A restore always creates a *new* tournament: every id is remapped to a fresh
//! UUID. That way importing can never overwrite work already on the device, and
//! the same file can be imported more than once (to keep a copy before an
//! experiment, say).

use crate::db::Database;
use crate::models::{
    Bracket, BracketMatch, QualifyingGame, QualifyingRound, Team, TeamStanding, Tournament, Umpire,
};
use chrono::Utc;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

/// Rows for the tables that have no model of their own because nothing but the
/// backup reads them whole.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanacheTeamRow {
    pub id: String,
    pub tournament_id: String,
    pub round_id: String,
    pub team_index: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanacheTeamMemberRow {
    pub id: String,
    pub panache_team_id: String,
    pub team_id: String,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PanacheSitoutRow {
    pub id: String,
    pub tournament_id: String,
    pub round_id: String,
    pub team_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingHistoryRow {
    pub id: String,
    pub tournament_id: String,
    pub team1_id: String,
    pub team2_id: String,
    pub round_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CourtHistoryRow {
    pub id: String,
    pub tournament_id: String,
    pub team_id: String,
    pub court_number: i32,
    pub round_id: String,
}

/// Everything needed to reconstruct one tournament.
///
/// Every collection is `#[serde(default)]` so a file written by an older build
/// - or by a format that never had panache - still restores.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TournamentBackup {
    /// Bumped only when the shape changes incompatibly.
    #[serde(default = "default_format_version")]
    pub format_version: i32,
    #[serde(default)]
    pub exported_at: String,
    pub tournament: Tournament,
    #[serde(default)]
    pub umpires: Vec<Umpire>,
    #[serde(default)]
    pub teams: Vec<Team>,
    #[serde(default)]
    pub qualifying_rounds: Vec<QualifyingRound>,
    #[serde(default)]
    pub qualifying_games: Vec<QualifyingGame>,
    #[serde(default)]
    pub standings: Vec<TeamStanding>,
    #[serde(default)]
    pub brackets: Vec<Bracket>,
    #[serde(default)]
    pub bracket_matches: Vec<BracketMatch>,
    #[serde(default)]
    pub panache_teams: Vec<PanacheTeamRow>,
    #[serde(default)]
    pub panache_team_members: Vec<PanacheTeamMemberRow>,
    #[serde(default)]
    pub panache_sitouts: Vec<PanacheSitoutRow>,
    #[serde(default)]
    pub pairing_history: Vec<PairingHistoryRow>,
    #[serde(default)]
    pub court_history: Vec<CourtHistoryRow>,
}

fn default_format_version() -> i32 {
    1
}

/// Reads one tournament and everything hanging off it.
pub fn collect_backup(conn: &Connection, tournament_id: &str) -> Result<TournamentBackup, String> {
    let tournament = conn
        .query_row(
            r#"
            SELECT id, name, team_composition, tournament_type, start_date, end_date, director,
                   head_umpire, format, number_of_courts, number_of_qualifying_rounds,
                   has_consolante, advance_all, advance_count, bracket_size, pairing_method,
                   region_avoidance, created_at, updated_at
            FROM tournaments WHERE id = ?1
            "#,
            params![tournament_id],
            |row| {
                Ok(Tournament {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    team_composition: row.get(2)?,
                    tournament_type: row.get(3)?,
                    start_date: row.get(4)?,
                    end_date: row.get(5)?,
                    director: row.get(6)?,
                    head_umpire: row.get(7)?,
                    format: row.get(8)?,
                    number_of_courts: row.get(9)?,
                    number_of_qualifying_rounds: row.get(10)?,
                    has_consolante: row.get::<_, i32>(11)? != 0,
                    advance_all: row.get::<_, i32>(12)? != 0,
                    advance_count: row.get(13)?,
                    bracket_size: row.get(14)?,
                    pairing_method: row.get(15)?,
                    region_avoidance: row.get::<_, i32>(16)? != 0,
                    created_at: row.get(17)?,
                    updated_at: row.get(18)?,
                })
            },
        )
        .map_err(|e| format!("Tournament not found: {}", e))?;

    macro_rules! rows {
        ($sql:expr, $map:expr) => {{
            let mut stmt = conn.prepare($sql).map_err(|e| e.to_string())?;
            let out = stmt
                .query_map(params![tournament_id], $map)
                .map_err(|e| e.to_string())?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|e| e.to_string())?;
            out
        }};
    }

    let umpires = rows!(
        "SELECT id, tournament_id, name FROM umpires WHERE tournament_id = ?1",
        |row| Ok(Umpire {
            id: row.get(0)?,
            tournament_id: row.get(1)?,
            name: row.get(2)?,
        })
    );

    let teams = rows!(
        "SELECT id, tournament_id, team_number, captain, player2, player3, region, club, \
         is_champion, created_at FROM teams WHERE tournament_id = ?1",
        |row| Ok(Team {
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
    );

    let qualifying_rounds = rows!(
        "SELECT id, tournament_id, round_number, is_complete, is_final, created_at \
         FROM qualifying_rounds WHERE tournament_id = ?1",
        |row| Ok(QualifyingRound {
            id: row.get(0)?,
            tournament_id: row.get(1)?,
            round_number: row.get(2)?,
            is_complete: row.get::<_, i32>(3)? != 0,
            is_final: row.get::<_, i32>(4)? != 0,
            created_at: row.get(5)?,
        })
    );

    let qualifying_games = rows!(
        "SELECT g.id, g.round_id, g.court_number, g.team1_id, g.team2_id, g.team1_score, \
         g.team2_score, g.is_bye, g.side1_id, g.side2_id \
         FROM qualifying_games g JOIN qualifying_rounds r ON r.id = g.round_id \
         WHERE r.tournament_id = ?1",
        |row| Ok(QualifyingGame {
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
    );

    let standings = rows!(
        "SELECT id, tournament_id, team_id, wins, losses, points_for, points_against, \
         differential, buchholz_score, fine_buchholz_score, point_quotient, is_eliminated, rank \
         FROM team_standings WHERE tournament_id = ?1",
        |row| Ok(TeamStanding {
            id: row.get(0)?,
            tournament_id: row.get(1)?,
            team_id: row.get(2)?,
            wins: row.get(3)?,
            losses: row.get(4)?,
            points_for: row.get(5)?,
            points_against: row.get(6)?,
            differential: row.get(7)?,
            buchholz_score: row.get(8)?,
            fine_buchholz_score: row.get(9)?,
            point_quotient: row.get(10)?,
            is_eliminated: row.get::<_, i32>(11)? != 0,
            rank: row.get(12)?,
        })
    );

    let brackets = rows!(
        "SELECT id, tournament_id, name, is_consolante, size, is_complete, created_at \
         FROM brackets WHERE tournament_id = ?1",
        |row| Ok(Bracket {
            id: row.get(0)?,
            tournament_id: row.get(1)?,
            name: row.get(2)?,
            is_consolante: row.get::<_, i32>(3)? != 0,
            size: row.get(4)?,
            is_complete: row.get::<_, i32>(5)? != 0,
            created_at: row.get(6)?,
        })
    );

    let bracket_matches = rows!(
        "SELECT m.id, m.bracket_id, m.round_number, m.match_number, m.court_number, m.team1_id, \
         m.team2_id, m.team1_score, m.team2_score, m.winner_id, m.next_match_id, m.is_bye \
         FROM bracket_matches m JOIN brackets b ON b.id = m.bracket_id \
         WHERE b.tournament_id = ?1",
        |row| Ok(BracketMatch {
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
        })
    );

    let panache_teams = rows!(
        "SELECT id, tournament_id, round_id, team_index FROM panache_teams WHERE tournament_id = ?1",
        |row| Ok(PanacheTeamRow {
            id: row.get(0)?,
            tournament_id: row.get(1)?,
            round_id: row.get(2)?,
            team_index: row.get(3)?,
        })
    );

    let panache_team_members = rows!(
        "SELECT m.id, m.panache_team_id, m.team_id, m.position \
         FROM panache_team_members m JOIN panache_teams p ON p.id = m.panache_team_id \
         WHERE p.tournament_id = ?1",
        |row| Ok(PanacheTeamMemberRow {
            id: row.get(0)?,
            panache_team_id: row.get(1)?,
            team_id: row.get(2)?,
            position: row.get(3)?,
        })
    );

    let panache_sitouts = rows!(
        "SELECT id, tournament_id, round_id, team_id FROM panache_sitouts WHERE tournament_id = ?1",
        |row| Ok(PanacheSitoutRow {
            id: row.get(0)?,
            tournament_id: row.get(1)?,
            round_id: row.get(2)?,
            team_id: row.get(3)?,
        })
    );

    let pairing_history = rows!(
        "SELECT id, tournament_id, team1_id, team2_id, round_id FROM pairing_history \
         WHERE tournament_id = ?1",
        |row| Ok(PairingHistoryRow {
            id: row.get(0)?,
            tournament_id: row.get(1)?,
            team1_id: row.get(2)?,
            team2_id: row.get(3)?,
            round_id: row.get(4)?,
        })
    );

    let court_history = rows!(
        "SELECT id, tournament_id, team_id, court_number, round_id FROM court_history \
         WHERE tournament_id = ?1",
        |row| Ok(CourtHistoryRow {
            id: row.get(0)?,
            tournament_id: row.get(1)?,
            team_id: row.get(2)?,
            court_number: row.get(3)?,
            round_id: row.get(4)?,
        })
    );

    Ok(TournamentBackup {
        format_version: default_format_version(),
        exported_at: Utc::now().to_rfc3339(),
        tournament,
        umpires,
        teams,
        qualifying_rounds,
        qualifying_games,
        standings,
        brackets,
        bracket_matches,
        panache_teams,
        panache_team_members,
        panache_sitouts,
        pairing_history,
        court_history,
    })
}

#[tauri::command]
pub fn export_tournament_backup(
    db: State<Database>,
    tournament_id: String,
) -> Result<String, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    let backup = collect_backup(&conn, &tournament_id)?;
    serde_json::to_string_pretty(&backup).map_err(|e| e.to_string())
}

/// Fresh id for an old one, stable within a single restore.
fn remap(map: &mut HashMap<String, String>, old: &str) -> String {
    map.entry(old.to_string())
        .or_insert_with(|| Uuid::new_v4().to_string())
        .clone()
}

/// A reference to a row that may not be in the backup resolves to NULL, which
/// is what every one of these columns already means by "missing".
fn remap_opt(map: &HashMap<String, String>, old: &Option<String>) -> Option<String> {
    old.as_ref().and_then(|id| map.get(id).cloned())
}

/// Restores a backup as a brand new tournament.
pub fn restore_backup(conn: &Connection, backup: &TournamentBackup) -> Result<String, String> {
    let mut tournaments = HashMap::new();
    let mut teams = HashMap::new();
    let mut rounds = HashMap::new();
    let mut games = HashMap::new();
    let mut brackets = HashMap::new();
    let mut matches = HashMap::new();
    let mut panache = HashMap::new();

    // Build every id up front so self-references (a match pointing at the next
    // match) can be written in one pass.
    let tournament_id = remap(&mut tournaments, &backup.tournament.id);
    for t in &backup.teams {
        remap(&mut teams, &t.id);
    }
    for r in &backup.qualifying_rounds {
        remap(&mut rounds, &r.id);
    }
    for g in &backup.qualifying_games {
        remap(&mut games, &g.id);
    }
    for b in &backup.brackets {
        remap(&mut brackets, &b.id);
    }
    for m in &backup.bracket_matches {
        remap(&mut matches, &m.id);
    }
    for p in &backup.panache_teams {
        remap(&mut panache, &p.id);
    }

    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    // Rows arrive in whatever order the file lists them; check references once
    // at commit instead of demanding a topological insert order.
    tx.execute_batch("PRAGMA defer_foreign_keys = ON;")
        .map_err(|e| e.to_string())?;

    let t = &backup.tournament;
    tx.execute(
        r#"
        INSERT INTO tournaments (id, name, team_composition, tournament_type, start_date, end_date,
            director, head_umpire, format, day_type, number_of_courts, number_of_qualifying_rounds,
            has_consolante, advance_all, advance_count, bracket_size, pairing_method,
            region_avoidance, created_at, updated_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'single', ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        "#,
        params![
            tournament_id, t.name, t.team_composition, t.tournament_type, t.start_date, t.end_date,
            t.director, t.head_umpire, t.format, t.number_of_courts, t.number_of_qualifying_rounds,
            t.has_consolante as i32, t.advance_all as i32, t.advance_count, t.bracket_size,
            t.pairing_method, t.region_avoidance as i32, t.created_at, Utc::now().to_rfc3339(),
        ],
    )
    .map_err(|e| format!("Could not restore the tournament: {}", e))?;

    for u in &backup.umpires {
        tx.execute(
            "INSERT INTO umpires (id, tournament_id, name) VALUES (?1, ?2, ?3)",
            params![Uuid::new_v4().to_string(), tournament_id, u.name],
        )
        .map_err(|e| e.to_string())?;
    }

    for team in &backup.teams {
        tx.execute(
            r#"
            INSERT INTO teams (id, tournament_id, team_number, captain, player2, player3, region,
                club, is_champion, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                teams[&team.id], tournament_id, team.team_number, team.captain, team.player2,
                team.player3, team.region, team.club, team.is_champion as i32, team.created_at,
            ],
        )
        .map_err(|e| format!("Could not restore team {}: {}", team.team_number, e))?;
    }

    for r in &backup.qualifying_rounds {
        tx.execute(
            r#"
            INSERT INTO qualifying_rounds (id, tournament_id, round_number, is_complete, is_final, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
            "#,
            params![
                rounds[&r.id], tournament_id, r.round_number, r.is_complete as i32,
                r.is_final as i32, r.created_at,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for p in &backup.panache_teams {
        tx.execute(
            "INSERT INTO panache_teams (id, tournament_id, round_id, team_index) VALUES (?1, ?2, ?3, ?4)",
            params![
                panache[&p.id],
                tournament_id,
                remap_opt(&rounds, &Some(p.round_id.clone())),
                p.team_index
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for m in &backup.panache_team_members {
        tx.execute(
            "INSERT INTO panache_team_members (id, panache_team_id, team_id, position) VALUES (?1, ?2, ?3, ?4)",
            params![
                Uuid::new_v4().to_string(),
                remap_opt(&panache, &Some(m.panache_team_id.clone())),
                remap_opt(&teams, &Some(m.team_id.clone())),
                m.position
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for s in &backup.panache_sitouts {
        tx.execute(
            "INSERT INTO panache_sitouts (id, tournament_id, round_id, team_id) VALUES (?1, ?2, ?3, ?4)",
            params![
                Uuid::new_v4().to_string(),
                tournament_id,
                remap_opt(&rounds, &Some(s.round_id.clone())),
                remap_opt(&teams, &Some(s.team_id.clone()))
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for g in &backup.qualifying_games {
        tx.execute(
            r#"
            INSERT INTO qualifying_games (id, round_id, court_number, team1_id, team2_id,
                team1_score, team2_score, is_bye, side1_id, side2_id)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
            "#,
            params![
                games[&g.id],
                remap_opt(&rounds, &Some(g.round_id.clone())),
                g.court_number,
                remap_opt(&teams, &g.team1_id),
                remap_opt(&teams, &g.team2_id),
                g.team1_score,
                g.team2_score,
                g.is_bye as i32,
                remap_opt(&panache, &g.side1_id),
                remap_opt(&panache, &g.side2_id),
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for s in &backup.standings {
        tx.execute(
            r#"
            INSERT INTO team_standings (id, tournament_id, team_id, wins, losses, points_for,
                points_against, differential, buchholz_score, fine_buchholz_score, point_quotient,
                is_eliminated, rank)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
            "#,
            params![
                Uuid::new_v4().to_string(), tournament_id,
                remap_opt(&teams, &Some(s.team_id.clone())),
                s.wins, s.losses, s.points_for, s.points_against, s.differential,
                s.buchholz_score, s.fine_buchholz_score, s.point_quotient,
                s.is_eliminated as i32, s.rank,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for b in &backup.brackets {
        tx.execute(
            r#"
            INSERT INTO brackets (id, tournament_id, name, is_consolante, size, is_complete, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                brackets[&b.id], tournament_id, b.name, b.is_consolante as i32, b.size,
                b.is_complete as i32, b.created_at,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for m in &backup.bracket_matches {
        tx.execute(
            r#"
            INSERT INTO bracket_matches (id, bracket_id, round_number, match_number, court_number,
                team1_id, team2_id, team1_score, team2_score, winner_id, next_match_id, is_bye)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
            "#,
            params![
                matches[&m.id],
                remap_opt(&brackets, &Some(m.bracket_id.clone())),
                m.round_number,
                m.match_number,
                m.court_number,
                remap_opt(&teams, &m.team1_id),
                remap_opt(&teams, &m.team2_id),
                m.team1_score,
                m.team2_score,
                remap_opt(&teams, &m.winner_id),
                remap_opt(&matches, &m.next_match_id),
                m.is_bye as i32,
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for p in &backup.pairing_history {
        tx.execute(
            "INSERT INTO pairing_history (id, tournament_id, team1_id, team2_id, round_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Uuid::new_v4().to_string(), tournament_id,
                remap_opt(&teams, &Some(p.team1_id.clone())),
                remap_opt(&teams, &Some(p.team2_id.clone())),
                remap_opt(&rounds, &Some(p.round_id.clone())),
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    for c in &backup.court_history {
        tx.execute(
            "INSERT INTO court_history (id, tournament_id, team_id, court_number, round_id) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Uuid::new_v4().to_string(), tournament_id,
                remap_opt(&teams, &Some(c.team_id.clone())),
                c.court_number,
                remap_opt(&rounds, &Some(c.round_id.clone())),
            ],
        )
        .map_err(|e| e.to_string())?;
    }

    tx.commit()
        .map_err(|e| format!("Restore failed, nothing was changed: {}", e))?;

    Ok(tournament_id)
}

#[tauri::command]
pub fn import_tournament_backup(db: State<Database>, json: String) -> Result<String, String> {
    let backup: TournamentBackup = serde_json::from_str(&json)
        .map_err(|e| format!("That file is not a tournament backup: {}", e))?;

    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    restore_backup(&conn, &backup)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    fn seed(conn: &Connection) -> String {
        schema::create_tables(conn).unwrap();
        conn.execute(
            r#"
            INSERT INTO tournaments (id, name, team_composition, tournament_type, start_date,
                end_date, director, head_umpire, format, day_type, number_of_courts,
                number_of_qualifying_rounds, has_consolante, advance_all, bracket_size,
                pairing_method, region_avoidance, created_at, updated_at)
            VALUES ('t1','Cup','select','open','2026-01-01','2026-01-01','d','u','double',
                'single', 4, 3, 1, 1, 8, 'swissHotel', 0, 'now', 'now')
            "#,
            [],
        )
        .unwrap();
        for (id, num, cap) in [("a", 1, "Ann"), ("b", 2, "Bob")] {
            conn.execute(
                "INSERT INTO teams (id, tournament_id, team_number, captain, player2, is_champion, created_at)
                 VALUES (?1,'t1',?2,?3,'x',0,'now')",
                params![id, num, cap],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO team_standings (id, tournament_id, team_id, wins, losses, points_for,
                    points_against, differential, rank) VALUES (?1,'t1',?2,1,0,13,7,6,1)",
                params![format!("s{}", id), id],
            )
            .unwrap();
        }
        conn.execute(
            "INSERT INTO qualifying_rounds (id, tournament_id, round_number, is_complete, created_at)
             VALUES ('r1','t1',1,1,'now')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO qualifying_games (id, round_id, court_number, team1_id, team2_id,
                team1_score, team2_score, is_bye) VALUES ('g1','r1',1,'a','b',13,7,0)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO umpires (id, tournament_id, name) VALUES ('u1','t1','Ref')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO pairing_history (id, tournament_id, team1_id, team2_id, round_id)
             VALUES ('p1','t1','a','b','r1')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO brackets (id, tournament_id, name, is_consolante, size, is_complete, created_at)
             VALUES ('b1','t1','A',0,2,0,'now')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO bracket_matches (id, bracket_id, round_number, match_number, court_number,
                team1_id, team2_id, next_match_id, is_bye) VALUES ('m1','b1',1,1,3,'a','b',NULL,0)",
            [],
        )
        .unwrap();
        "t1".to_string()
    }

    #[test]
    fn a_backup_covers_the_tables_the_frontend_version_dropped() {
        let conn = Connection::open_in_memory().unwrap();
        let id = seed(&conn);
        let backup = collect_backup(&conn, &id).unwrap();

        assert_eq!(backup.teams.len(), 2);
        assert_eq!(backup.qualifying_games.len(), 1);
        assert_eq!(backup.standings.len(), 2);
        assert_eq!(backup.bracket_matches.len(), 1);
        // The three the old frontend-assembled backup silently omitted:
        assert_eq!(backup.umpires.len(), 1, "umpires must be captured");
        assert_eq!(backup.pairing_history.len(), 1, "pairing history must be captured");
        assert_eq!(backup.court_history.len(), 0);
    }

    #[test]
    fn a_restore_rebuilds_the_tournament_under_new_ids() {
        let conn = Connection::open_in_memory().unwrap();
        let id = seed(&conn);
        let backup = collect_backup(&conn, &id).unwrap();

        let new_id = restore_backup(&conn, &backup).unwrap();
        assert_ne!(new_id, id, "restore must not reuse the original id");

        let teams: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM teams WHERE tournament_id = ?1",
                params![new_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(teams, 2);

        // References must point inside the new tournament, not the old one.
        let (t1, t2): (String, String) = conn
            .query_row(
                "SELECT g.team1_id, g.team2_id FROM qualifying_games g
                 JOIN qualifying_rounds r ON r.id = g.round_id WHERE r.tournament_id = ?1",
                params![new_id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        for team_id in [&t1, &t2] {
            let owner: String = conn
                .query_row(
                    "SELECT tournament_id FROM teams WHERE id = ?1",
                    params![team_id],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(&owner, &new_id, "game still points at the original team row");
        }

        // The original is untouched.
        let original: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM teams WHERE tournament_id = 't1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(original, 2);
    }

    #[test]
    fn importing_the_same_file_twice_makes_two_tournaments() {
        let conn = Connection::open_in_memory().unwrap();
        let id = seed(&conn);
        let backup = collect_backup(&conn, &id).unwrap();

        let first = restore_backup(&conn, &backup).unwrap();
        let second = restore_backup(&conn, &backup).unwrap();
        assert_ne!(first, second);

        let total: i32 = conn
            .query_row("SELECT COUNT(*) FROM tournaments", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 3, "original plus two imports");
    }

    /// A file from the older, thinner backup must still restore.
    #[test]
    fn a_backup_missing_the_newer_sections_still_restores() {
        let conn = Connection::open_in_memory().unwrap();
        seed(&conn);
        let json = r#"{
            "tournament": {
                "id": "old", "name": "Legacy", "teamComposition": "select", "type": "open",
                "startDate": "2026-01-01", "endDate": "2026-01-01", "director": "d",
                "headUmpire": "u", "format": "double", "numberOfCourts": 4,
                "numberOfQualifyingRounds": 3, "hasConsolante": false, "advanceAll": true,
                "advanceCount": null, "bracketSize": 8, "pairingMethod": "swiss",
                "regionAvoidance": false, "createdAt": "now", "updatedAt": "now"
            },
            "teams": []
        }"#;
        let backup: TournamentBackup = serde_json::from_str(json).unwrap();
        let new_id = restore_backup(&conn, &backup).unwrap();

        let name: String = conn
            .query_row(
                "SELECT name FROM tournaments WHERE id = ?1",
                params![new_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(name, "Legacy");
    }
}
