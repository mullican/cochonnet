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
            SELECT id, tournament_id, captain, player2, player3, region, club, created_at
            FROM teams
            WHERE tournament_id = ?1
            ORDER BY captain
            "#,
        )
        .map_err(|e| e.to_string())?;

    let teams = stmt
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

    Ok(teams)
}

#[tauri::command]
pub fn get_team(db: State<Database>, id: String) -> Result<Team, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let team = conn
        .query_row(
            r#"
            SELECT id, tournament_id, captain, player2, player3, region, club, created_at
            FROM teams
            WHERE id = ?1
            "#,
            params![id],
            |row| {
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
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(team)
}

#[tauri::command]
pub fn create_team(db: State<Database>, data: CreateTeamData) -> Result<Team, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO teams (id, tournament_id, captain, player2, player3, region, club, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
        "#,
        params![
            id,
            data.tournament_id,
            data.captain,
            data.player2,
            data.player3,
            data.region,
            data.club,
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
        captain: data.captain,
        player2: data.player2,
        player3: data.player3,
        region: data.region,
        club: data.club,
        created_at: now,
    };

    Ok(team)
}

#[tauri::command]
pub fn update_team(db: State<Database>, id: String, data: CreateTeamData) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute(
        r#"
        UPDATE teams SET
            captain = ?2,
            player2 = ?3,
            player3 = ?4,
            region = ?5,
            club = ?6
        WHERE id = ?1
        "#,
        params![id, data.captain, data.player2, data.player3, data.region, data.club],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn delete_team(db: State<Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

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

    let now = Utc::now().to_rfc3339();
    let mut count = 0;

    for team_data in teams {
        let id = Uuid::new_v4().to_string();

        conn.execute(
            r#"
            INSERT INTO teams (id, tournament_id, captain, player2, player3, region, club, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
            params![
                id,
                tournament_id,
                team_data.captain,
                team_data.player2,
                team_data.player3,
                team_data.region,
                team_data.club,
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
            SELECT id, tournament_id, team_id, wins, losses, points_for, points_against, differential, buchholz_score, rank
            FROM team_standings
            WHERE tournament_id = ?1
            ORDER BY rank ASC, wins DESC, differential DESC, points_for DESC
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
                rank: row.get(9)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(standings)
}

pub fn get_team_by_id(conn: &rusqlite::Connection, id: &str) -> Result<Option<Team>, String> {
    match conn.query_row(
        r#"
        SELECT id, tournament_id, captain, player2, player3, region, club, created_at
        FROM teams
        WHERE id = ?1
        "#,
        params![id],
        |row| {
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
        },
    ) {
        Ok(team) => Ok(Some(team)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
