mod commands;
mod db;
mod models;

use db::Database;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    // Only macOS gets a menu bar by default, so only macOS needs this.
    #[cfg(target_os = "macos")]
    let builder = builder.menu(app_menu);

    builder
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
            commands::delete_all_teams,
            commands::get_standings,
            // Qualifying round commands
            commands::get_qualifying_rounds,
            commands::get_games_for_round,
            commands::generate_pairings,
            commands::generate_all_qualifying_rounds,
            commands::delete_all_qualifying_rounds,
            commands::update_game_score,
            commands::update_game_court,
            commands::complete_round,
            commands::reopen_round,
            // Bracket commands
            commands::get_brackets,
            commands::get_matches_for_bracket,
            commands::generate_brackets,
            commands::delete_brackets,
            commands::update_match_score,
            commands::update_match_court,
            commands::get_bracket_court_conflicts,
            commands::get_bracket_off_plan_courts,
            // Court plan
            commands::courts::get_court_plan,
            commands::courts::save_court_plan,
            commands::courts::get_round_court_offer,
            commands::courts::get_court_demand,
            // Panache commands
            commands::generate_panache_rounds,
            commands::generate_panache_round,
            commands::redraw_panache_rounds,
            commands::generate_panache_final,
            commands::get_sitouts_for_round,
            commands::set_team_champion,
            commands::set_team_withdrawn,
            // Printing / export capability
            commands::print_pdf,
            commands::printing_available,
            commands::file_export_available,
            // Whole-tournament backup / restore
            commands::export_tournament_backup,
            commands::import_tournament_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/// Tauri's default menu, with the About item named after the product.
///
/// macOS labels that item with the running process's name. An installed app
/// takes it from its bundle, but `tauri dev` runs the bare binary, which is
/// named after the crate, so the item read "About petanque-tournament".
#[cfg(target_os = "macos")]
fn app_menu<R: tauri::Runtime>(
    handle: &tauri::AppHandle<R>,
) -> tauri::Result<tauri::menu::Menu<R>> {
    use tauri::menu::{Menu, MenuItemKind};

    let menu = Menu::default(handle)?;
    // The application menu comes first, and About is the first thing in it.
    if let Some(MenuItemKind::Submenu(app)) = menu.items()?.into_iter().next() {
        if let Some(MenuItemKind::Predefined(about)) = app.items()?.into_iter().next() {
            about.set_text(format!("About {}", handle.package_info().name))?;
        }
    }
    Ok(menu)
}
