use ppaass_agent::bo::config::Config;
use ppaass_agent::server::AgentServer;
use std::sync::Arc;
const AGENT_CONFIG_FILE: &str = "config.toml";
// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let agent_server_config_file = std::fs::read_to_string(AGENT_CONFIG_FILE).expect("Fail to load configuration file content");
    let config = toml::from_str::<Config>(&agent_server_config_file).expect("Fail to parse agent configuration file");
    let agent_server = AgentServer::new(Arc::new(config));
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
