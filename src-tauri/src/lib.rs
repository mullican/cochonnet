mod commands;
mod db;
mod models;

use db::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let database = Database::new(&app.handle())
                .expect("Failed to create database connection");
            database.initialize().expect("Failed to initialize database");
            app.manage(database);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // Tournament commands
            commands::get_tournaments,
            commands::get_tournament,
            commands::create_tournament,
            commands::update_tournament,
            commands::delete_tournament,
            commands::get_umpires,
            // Team commands
            commands::get_teams,
            commands::get_team,
            commands::create_team,
            commands::update_team,
            commands::delete_team,
            commands::import_teams,
            commands::get_standings,
            // Qualifying round commands
            commands::get_qualifying_rounds,
            commands::get_games_for_round,
            commands::generate_pairings,
            commands::generate_all_qualifying_rounds,
            commands::update_game_score,
            commands::complete_round,
            // Bracket commands
            commands::get_brackets,
            commands::get_matches_for_bracket,
            commands::generate_brackets,
            commands::delete_brackets,
            commands::update_match_score,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
