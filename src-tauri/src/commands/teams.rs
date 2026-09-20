use crate::db::Database;
use crate::models::{CreateTeamData, Team, TeamStanding};
use chrono::Utc;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_teams(db: State<Database>, tournament_id: String) -> Result<Vec<Team>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, tournament_id, team_number, captain, player2, player3, region, club, is_champion, is_withdrawn, created_at
            FROM teams
            WHERE tournament_id = ?1
            ORDER BY team_number
            "#,
        )
        .map_err(|e| e.to_string())?;

    let teams = stmt
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

    Ok(teams)
}

#[tauri::command]
pub fn get_team(db: State<Database>, id: String) -> Result<Team, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let team = conn
        .query_row(
            r#"
            SELECT id, tournament_id, team_number, captain, player2, player3, region, club, is_champion, is_withdrawn, created_at
            FROM teams
            WHERE id = ?1
            "#,
            params![id],
            |row| {
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
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(team)
}

#[tauri::command]
pub fn create_team(db: State<Database>, data: CreateTeamData) -> Result<Team, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    check_roster_capacity(&conn, &data.tournament_id, 1)?;

    let team_number = resolve_team_number(&conn, &data.tournament_id, data.team_number, None)?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO teams (id, tournament_id, team_number, captain, player2, player3, region, club, is_champion, is_withdrawn, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            id,
            data.tournament_id,
            team_number,
            data.captain,
            data.player2,
            data.player3,
            data.region,
            data.club,
            if data.is_champion.unwrap_or(false) { 1 } else { 0 },
            if data.is_withdrawn.unwrap_or(false) { 1 } else { 0 },
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Initialize team standing
    let standing_id = Uuid::new_v4().to_string();
    conn.execute(
        r#"
        INSERT INTO team_standings (id, tournament_id, team_id, wins, losses, points_for, points_against, differential, buchholz_score, rank)
        VALUES (?1, ?2, ?3, 0, 0, 0, 0, 0, 0, 0)
        "#,
        params![standing_id, data.tournament_id, id],
    )
    .map_err(|e| e.to_string())?;

    let team = Team {
        id,
        tournament_id: data.tournament_id,
        team_number,
        captain: data.captain,
        player2: data.player2,
        player3: data.player3,
        region: data.region,
        club: data.club,
        is_champion: data.is_champion.unwrap_or(false),
        is_withdrawn: data.is_withdrawn.unwrap_or(false),
        created_at: now,
    };

    Ok(team)
}

/// Refuses a roster that could not all play at once.
///
/// The team formats seat two teams per court. Panache registers individuals and
/// seats a whole doubles or triples game per court, so the same court count holds
/// two, four or six times as many entries depending on the format.
fn check_roster_capacity(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    adding: i32,
) -> Result<(), String> {
    let (number_of_courts, pairing_method, format): (i32, String, String) = conn
        .query_row(
            "SELECT number_of_courts, pairing_method, format FROM tournaments WHERE id = ?1",
            params![tournament_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .map_err(|e| e.to_string())?;

    let is_panache = pairing_method == "panache";
    let per_court = if is_panache {
        match format.as_str() {
            "triple" => 6,
            _ => 4,
        }
    } else {
        2
    };
    let noun = if is_panache { "players" } else { "teams" };

    // Withdrawn entrants are not going on a court again, so they do not hold a
    // slot: a replacement can take the place of a team that has pulled out.
    let current: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM teams WHERE tournament_id = ?1 AND is_withdrawn = 0",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    let max_entries = number_of_courts * per_court;
    if current + adding > max_entries {
        let available = (max_entries - current).max(0);
        if adding == 1 {
            return Err(format!(
                "Cannot add more {}. Maximum is {} {} ({} courts × {}).",
                noun, max_entries, noun, number_of_courts, per_court
            ));
        }
        return Err(format!(
            "Cannot import {} {}. Maximum is {} {} ({} courts × {}). Currently have {}, only {} slots available.",
            adding, noun, max_entries, noun, number_of_courts, per_court, current, available
        ));
    }

    Ok(())
}

/// Resolves the team number to use for a create/update: validates an
/// explicitly-provided number is unique within the tournament, or assigns
/// the next available number when none was provided. `exclude_id` should be
/// the team's own id when updating, so it doesn't conflict with itself.
fn resolve_team_number(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    requested_number: Option<i32>,
    exclude_id: Option<&str>,
) -> Result<i32, String> {
    match requested_number {
        Some(number) => {
            let conflict_exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM teams WHERE tournament_id = ?1 AND team_number = ?2 AND id != ?3",
                    params![tournament_id, number, exclude_id.unwrap_or("")],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            if conflict_exists {
                return Err(format!("Team number {} is already in use.", number));
            }

            Ok(number)
        }
        None => {
            let max_number: Option<i32> = conn
                .query_row(
                    "SELECT MAX(team_number) FROM teams WHERE tournament_id = ?1",
                    params![tournament_id],
                    |row| row.get(0),
                )
                .map_err(|e| e.to_string())?;

            Ok(max_number.unwrap_or(0) + 1)
        }
    }
}

#[tauri::command]
pub fn update_team(db: State<Database>, id: String, data: CreateTeamData) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // On update, a missing team_number means "leave it as-is" rather than
    // auto-assigning a new one (unlike create, where it means "auto-assign").
    let team_number = if data.team_number.is_some() {
        resolve_team_number(&conn, &data.tournament_id, data.team_number, Some(&id))?
    } else {
        conn.query_row(
            "SELECT team_number FROM teams WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?
    };

    conn.execute(
        r#"
        UPDATE teams SET
            team_number = ?2,
            captain = ?3,
            player2 = ?4,
            player3 = ?5,
            region = ?6,
            club = ?7,
            is_champion = COALESCE(?8, is_champion),
            is_withdrawn = COALESCE(?9, is_withdrawn)
        WHERE id = ?1
        "#,
        params![
            id,
            team_number,
            data.captain,
            data.player2,
            data.player3,
            data.region,
            data.club,
            data.is_champion.map(|c| if c { 1 } else { 0 }),
            data.is_withdrawn.map(|w| if w { 1 } else { 0 })
        ],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_team(db: State<Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Get the team's tournament_id
    let tournament_id: String = conn
        .query_row(
            "SELECT tournament_id FROM teams WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    // Check if qualifying rounds have been generated
    let rounds_exist: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM qualifying_rounds WHERE tournament_id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if rounds_exist > 0 {
        return Err("Cannot delete teams after qualifying rounds have been generated. Delete all rounds first.".to_string());
    }

    conn.execute("DELETE FROM teams WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn import_teams(
    db: State<Database>,
    tournament_id: String,
    teams: Vec<CreateTeamData>,
) -> Result<i32, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    check_roster_capacity(&conn, &tournament_id, teams.len() as i32)?;

    let now = Utc::now().to_rfc3339();
    let mut count = 0;

    for team_data in teams {
        let id = Uuid::new_v4().to_string();
        let team_number = resolve_team_number(&conn, &tournament_id, team_data.team_number, None)?;

        conn.execute(
            r#"
            INSERT INTO teams (id, tournament_id, team_number, captain, player2, player3, region, club, is_champion, is_withdrawn, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
            "#,
            params![
                id,
                tournament_id,
                team_number,
                team_data.captain,
                team_data.player2,
                team_data.player3,
                team_data.region,
                team_data.club,
                if team_data.is_champion.unwrap_or(false) { 1 } else { 0 },
                if team_data.is_withdrawn.unwrap_or(false) { 1 } else { 0 },
                now,
            ],
        )
        .map_err(|e| e.to_string())?;

        // Initialize team standing
        let standing_id = Uuid::new_v4().to_string();
        conn.execute(
            r#"
            INSERT INTO team_standings (id, tournament_id, team_id, wins, losses, points_for, points_against, differential, buchholz_score, rank)
            VALUES (?1, ?2, ?3, 0, 0, 0, 0, 0, 0, 0)
            "#,
            params![standing_id, tournament_id, id],
        )
        .map_err(|e| e.to_string())?;

        count += 1;
    }

    Ok(count)
}

#[tauri::command]
pub fn get_standings(db: State<Database>, tournament_id: String) -> Result<Vec<TeamStanding>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, tournament_id, team_id, wins, losses, points_for, points_against, differential, buchholz_score, fine_buchholz_score, point_quotient, is_eliminated, rank
            FROM team_standings
            WHERE tournament_id = ?1
            ORDER BY rank ASC, wins DESC, buchholz_score DESC, fine_buchholz_score DESC, differential DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let standings = stmt
        .query_map(params![tournament_id], |row| {
            Ok(TeamStanding {
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
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(standings)
}

#[tauri::command]
pub fn delete_all_teams(db: State<Database>, tournament_id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Check if qualifying rounds have been generated
    let rounds_exist: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM qualifying_rounds WHERE tournament_id = ?1",
            params![tournament_id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if rounds_exist > 0 {
        return Err("Cannot delete teams after qualifying rounds have been generated. Delete all rounds first.".to_string());
    }

    // Delete team standings
    conn.execute(
        "DELETE FROM team_standings WHERE tournament_id = ?1",
        params![tournament_id],
    )
    .map_err(|e| e.to_string())?;

    // Delete teams
    conn.execute(
        "DELETE FROM teams WHERE tournament_id = ?1",
        params![tournament_id],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn get_team_by_id(conn: &rusqlite::Connection, id: &str) -> Result<Option<Team>, String> {
    match conn.query_row(
        r#"
        SELECT id, tournament_id, team_number, captain, player2, player3, region, club, is_champion, is_withdrawn, created_at
        FROM teams
        WHERE id = ?1
        "#,
        params![id],
        |row| {
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
        },
    ) {
        Ok(team) => Ok(Some(team)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
