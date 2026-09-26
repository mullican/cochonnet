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
                   pairing_method, region_avoidance, logo, created_at, updated_at
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
                logo: row.get(17)?,
                created_at: row.get(18)?,
                updated_at: row.get(19)?,
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
                   pairing_method, region_avoidance, logo, created_at, updated_at
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
                    logo: row.get(17)?,
                    created_at: row.get(18)?,
                    updated_at: row.get(19)?,
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
            pairing_method, region_avoidance, logo, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'single', ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)
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
            data.logo,
            now,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    if let Some(umpires) = &data.additional_umpires {
        replace_umpires(&conn, &id, umpires)?;
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
        logo: data.logo,
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

    // Courts cap the roster: a game needs a court, and a panache game needs a
    // whole one. Registration enforces that when teams are added, but nothing
    // stopped the courts being lowered afterwards, which left a tournament
    // holding more teams than it could ever put on the ground.
    let registered: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM teams WHERE tournament_id = ?1 AND is_withdrawn = 0",
            params![id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;

    if registered > 0 {
        let per_court = if data.pairing_method == "panache" {
            if data.format == "triple" { 6 } else { 4 }
        } else {
            2
        };
        let capacity = data.number_of_courts * per_court;
        if registered > capacity {
            let noun = if data.pairing_method == "panache" { "players" } else { "teams" };
            return Err(format!(
                "{} {} are registered, but {} courts only hold {} ({} courts x {}). Remove {} or add courts.",
                registered, noun, data.number_of_courts, capacity,
                data.number_of_courts, per_court, noun
            ));
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
            logo = ?18,
            updated_at = ?19
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
            data.logo,
            now,
        ],
    )
    .map_err(|e| e.to_string())?;

    if let Some(umpires) = &data.additional_umpires {
        replace_umpires(&conn, &id, umpires)?;
    }

    Ok(())
}

/// Sets a tournament's additional umpires to exactly this list.
///
/// Callers pass `None` to mean "leave them alone", the same convention
/// `CreateTeamData` uses for its flags. That distinction is the fix for a bug
/// worth spelling out: the delete used to run unconditionally on update, and
/// the frontend never sent the list at all, so umpires typed into the form were
/// dropped on the way in and then deleted by the first edit. An empty list is
/// still a real instruction - it is how the last umpire is removed.
fn replace_umpires(
    conn: &rusqlite::Connection,
    tournament_id: &str,
    umpires: &[String],
) -> Result<(), String> {
    conn.execute(
        "DELETE FROM umpires WHERE tournament_id = ?1",
        params![tournament_id],
    )
    .map_err(|e| e.to_string())?;

    for name in umpires {
        // Blank rows are what an operator leaves behind after clicking Add and
        // changing their mind; they are not umpires.
        if name.trim().is_empty() {
            continue;
        }
        conn.execute(
            "INSERT INTO umpires (id, tournament_id, name) VALUES (?1, ?2, ?3)",
            params![Uuid::new_v4().to_string(), tournament_id, name.trim()],
        )
        .map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    const TID: &str = "tour";

    fn seed() -> Connection {
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
                'D', 'U', 'double', 'single', 8, 5, 0, 1, NULL, 16,
                'swiss', 0, 'now', 'now'
            )
            "#,
            params![TID],
        )
        .unwrap();
        conn
    }

    fn names(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM umpires WHERE tournament_id = ?1 ORDER BY name")
            .unwrap();
        stmt.query_map(params![TID], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    #[test]
    fn umpires_are_replaced_wholesale() {
        let conn = seed();

        replace_umpires(&conn, TID, &["Ana".into(), "Bo".into()]).unwrap();
        assert_eq!(names(&conn), vec!["Ana", "Bo"]);

        // A second call is the whole list again, not an addition.
        replace_umpires(&conn, TID, &["Cy".into()]).unwrap();
        assert_eq!(names(&conn), vec!["Cy"]);

        // And an empty list is how the last one is removed.
        replace_umpires(&conn, TID, &[]).unwrap();
        assert!(names(&conn).is_empty());
    }

    /// Clicking Add and changing your mind leaves an empty row in the form.
    #[test]
    fn blank_rows_are_not_umpires() {
        let conn = seed();
        replace_umpires(&conn, TID, &["".into(), "  ".into(), " Dee ".into()]).unwrap();
        assert_eq!(names(&conn), vec!["Dee"]);
    }
}
