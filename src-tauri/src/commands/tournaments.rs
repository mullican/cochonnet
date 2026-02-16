use crate::db::Database;
use crate::models::{CreateTournamentData, Tournament, Umpire};
use chrono::Utc;
use rusqlite::params;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_tournaments(db: State<Database>) -> Result<Vec<Tournament>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare(
            r#"
            SELECT id, name, team_composition, tournament_type, start_date, end_date,
                   director, head_umpire, format, number_of_courts,
                   number_of_qualifying_rounds, has_consolante, advance_all, advance_count, bracket_size,
                   pairing_method, region_avoidance, created_at, updated_at
            FROM tournaments
            ORDER BY created_at DESC
            "#,
        )
        .map_err(|e| e.to_string())?;

    let tournaments = stmt
        .query_map([], |row| {
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
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(tournaments)
}

#[tauri::command]
pub fn get_tournament(db: State<Database>, id: String) -> Result<Tournament, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let tournament = conn
        .query_row(
            r#"
            SELECT id, name, team_composition, tournament_type, start_date, end_date,
                   director, head_umpire, format, number_of_courts,
                   number_of_qualifying_rounds, has_consolante, advance_all, advance_count, bracket_size,
                   pairing_method, region_avoidance, created_at, updated_at
            FROM tournaments
            WHERE id = ?1
            "#,
            params![id],
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
        .map_err(|e| e.to_string())?;

    Ok(tournament)
}

#[tauri::command]
pub fn create_tournament(
    db: State<Database>,
    data: CreateTournamentData,
) -> Result<Tournament, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        INSERT INTO tournaments (
            id, name, team_composition, tournament_type, start_date, end_date,
            director, head_umpire, format, day_type, number_of_courts,
            number_of_qualifying_rounds, has_consolante, advance_all, advance_count, bracket_size,
            pairing_method, region_avoidance, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'single', ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        "#,
        params![
            id,
            data.name,
            data.team_composition,
            data.tournament_type,
            data.start_date,
            data.end_date,
            data.director,
            data.head_umpire,
            data.format,
            data.number_of_courts,
            data.number_of_qualifying_rounds,
            if data.has_consolante { 1 } else { 0 },
            if data.advance_all { 1 } else { 0 },
            data.advance_count,
            data.bracket_size,
            data.pairing_method,
            if data.region_avoidance { 1 } else { 0 },
            now,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Insert additional umpires if provided
    if let Some(umpires) = data.additional_umpires {
        for umpire_name in umpires {
            if !umpire_name.trim().is_empty() {
                let umpire_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO umpires (id, tournament_id, name) VALUES (?1, ?2, ?3)",
                    params![umpire_id, id, umpire_name],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    let tournament = Tournament {
        id,
        name: data.name,
        team_composition: data.team_composition,
        tournament_type: data.tournament_type,
        start_date: data.start_date,
        end_date: data.end_date,
        director: data.director,
        head_umpire: data.head_umpire,
        format: data.format,
        number_of_courts: data.number_of_courts,
        number_of_qualifying_rounds: data.number_of_qualifying_rounds,
        has_consolante: data.has_consolante,
        advance_all: data.advance_all,
        advance_count: data.advance_count,
        bracket_size: data.bracket_size,
        pairing_method: data.pairing_method,
        region_avoidance: data.region_avoidance,
        created_at: now.clone(),
        updated_at: now,
    };

    Ok(tournament)
}

#[tauri::command]
pub fn update_tournament(
    db: State<Database>,
    id: String,
    data: CreateTournamentData,
) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    // Check if qualifying rounds have been generated
    let rounds_exist: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM qualifying_rounds WHERE tournament_id = ?1",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if rounds_exist > 0 {
        // Get current tournament settings to check if locked fields are being changed
        let (current_courts, current_rounds, current_pairing): (i32, i32, String) = conn
            .query_row(
                "SELECT number_of_courts, number_of_qualifying_rounds, pairing_method FROM tournaments WHERE id = ?1",
                params![id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|e| e.to_string())?;

        if data.number_of_courts != current_courts {
            return Err("Cannot change number of courts after qualifying rounds have been generated. Delete all rounds first.".to_string());
        }
        if data.number_of_qualifying_rounds != current_rounds {
            return Err("Cannot change number of qualifying rounds after rounds have been generated. Delete all rounds first.".to_string());
        }
        if data.pairing_method != current_pairing {
            return Err("Cannot change pairing method after qualifying rounds have been generated. Delete all rounds first.".to_string());
        }
    }

    let now = Utc::now().to_rfc3339();

    conn.execute(
        r#"
        UPDATE tournaments SET
            name = ?2,
            team_composition = ?3,
            tournament_type = ?4,
            start_date = ?5,
            end_date = ?6,
            director = ?7,
            head_umpire = ?8,
            format = ?9,
            number_of_courts = ?10,
            number_of_qualifying_rounds = ?11,
            has_consolante = ?12,
            advance_all = ?13,
            advance_count = ?14,
            bracket_size = ?15,
            pairing_method = ?16,
            region_avoidance = ?17,
            updated_at = ?18
        WHERE id = ?1
        "#,
        params![
            id,
            data.name,
            data.team_composition,
            data.tournament_type,
            data.start_date,
            data.end_date,
            data.director,
            data.head_umpire,
            data.format,
            data.number_of_courts,
            data.number_of_qualifying_rounds,
            if data.has_consolante { 1 } else { 0 },
            if data.advance_all { 1 } else { 0 },
            data.advance_count,
            data.bracket_size,
            data.pairing_method,
            if data.region_avoidance { 1 } else { 0 },
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    // Update additional umpires
    conn.execute("DELETE FROM umpires WHERE tournament_id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    if let Some(umpires) = data.additional_umpires {
        for umpire_name in umpires {
            if !umpire_name.trim().is_empty() {
                let umpire_id = Uuid::new_v4().to_string();
                conn.execute(
                    "INSERT INTO umpires (id, tournament_id, name) VALUES (?1, ?2, ?3)",
                    params![umpire_id, id, umpire_name],
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(())
}

#[tauri::command]
pub fn delete_tournament(db: State<Database>, id: String) -> Result<(), String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    conn.execute("DELETE FROM tournaments WHERE id = ?1", params![id])
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub fn get_umpires(db: State<Database>, tournament_id: String) -> Result<Vec<Umpire>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;

    let mut stmt = conn
        .prepare("SELECT id, tournament_id, name FROM umpires WHERE tournament_id = ?1")
        .map_err(|e| e.to_string())?;

    let umpires = stmt
        .query_map(params![tournament_id], |row| {
            Ok(Umpire {
                id: row.get(0)?,
                tournament_id: row.get(1)?,
                name: row.get(2)?,
            })
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;

    Ok(umpires)
}
