#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    collections::HashMap,
    net::TcpListener,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::Mutex,
};

use config_secure::{read_legacy_env, KeyringProvider, SecureConfigStore};
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Manager, State,
};

#[derive(Default)]
struct ProcessState {
    child: Mutex<Option<Child>>,
    port: Mutex<u16>,
    auth_token_hint: Mutex<Option<String>>,
}

#[derive(Serialize)]
struct ImportResult {
    imported_count: usize,
    secure_path: String,
}

#[tauri::command]
fn runtime_base_url(state: State<'_, ProcessState>) -> String {
    let port = *state.port.lock().expect("port lock");
    format!("http://127.0.0.1:{port}")
}

#[tauri::command]
fn gateway_status(state: State<'_, ProcessState>) -> String {
    let mut guard = state.child.lock().expect("child lock");
    if let Some(child) = guard.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => {
                *guard = None;
                "stopped".to_string()
            }
            Ok(None) => "running".to_string(),
            Err(_) => "unknown".to_string(),
        }
    } else {
        "stopped".to_string()
    }
}

#[tauri::command]
fn auth_token_hint(state: State<'_, ProcessState>) -> Option<String> {
    state.auth_token_hint.lock().ok().and_then(|token| token.clone())
}

#[tauri::command]
fn start_gateway(app: AppHandle, state: State<'_, ProcessState>) -> Result<String, String> {
    start_gateway_impl(&app, &state)
}

#[tauri::command]
fn stop_gateway(state: State<'_, ProcessState>) -> Result<String, String> {
    let mut guard = state.child.lock().map_err(|e| e.to_string())?;
    if let Some(child) = guard.as_mut() {
        child.kill().map_err(|e| e.to_string())?;
        let _ = child.wait();
        *guard = None;
        Ok("stopped".to_string())
    } else {
        Ok("already stopped".to_string())
    }
}

#[tauri::command]
fn import_legacy_env(app: AppHandle, path: String) -> Result<ImportResult, String> {
    let vars = read_legacy_env(&path).map_err(|e| e.to_string())?;
    let config_dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;

    let secure_path = config_dir.join("secure-config.json");
    let provider = KeyringProvider::new("com.gemini.balance.desktop", "master-key");
    let store = SecureConfigStore::new(provider, &secure_path);
    store.save(&vars).map_err(|e| e.to_string())?;

    Ok(ImportResult {
        imported_count: vars.len(),
        secure_path: secure_path.to_string_lossy().to_string(),
    })
}

fn start_gateway_impl(app: &AppHandle, state: &State<'_, ProcessState>) -> Result<String, String> {
    {
        let mut guard = state.child.lock().map_err(|e| e.to_string())?;
        if let Some(child) = guard.as_mut() {
            if child.try_wait().map_err(|e| e.to_string())?.is_none() {
                return Ok("already running".to_string());
            }
            *guard = None;
        }
    }

    let port = find_available_port(18080, 18099).ok_or_else(|| "no free local ports".to_string())?;
    *state.port.lock().map_err(|e| e.to_string())? = port;

    let binary = resolve_gateway_binary(app)?;
    let mut cmd = Command::new(binary);
    let env_map = load_runtime_env_map(app);
    for (key, value) in &env_map {
        cmd.env(key, value);
    }

    if let Some(token) = env_map.get("AUTH_TOKEN").cloned() {
        if let Ok(mut guard) = state.auth_token_hint.lock() {
            *guard = Some(token);
        }
    }

    cmd.env("RUNTIME_BIND_HOST", "127.0.0.1")
        .env("RUNTIME_PORT_START", port.to_string())
        .env("RUNTIME_PORT_END", port.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().map_err(|e| e.to_string())?;
    *state.child.lock().map_err(|e| e.to_string())? = Some(child);
    Ok(format!("running on 127.0.0.1:{port}"))
}

fn resolve_gateway_binary(app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("GATEWAY_SERVER_BIN") {
        let pb = PathBuf::from(path);
        if pb.exists() {
            return Ok(pb);
        }
    }

    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .map_err(|e| e.to_string())?;

    let candidates = [
        root.join("core-rs/target/release/gateway-server"),
        root.join("core-rs/target/debug/gateway-server"),
    ];

    for path in candidates {
        if path.exists() {
            return Ok(path);
        }
    }

    if let Ok(resource_dir) = app.path().resource_dir() {
        let packaged = resource_dir.join("binaries/gateway-server");
        if packaged.exists() {
            return Ok(packaged);
        }
    }

    Err("gateway-server binary not found, please build core-rs first".to_string())
}

fn load_runtime_env_map(app: &AppHandle) -> HashMap<String, String> {
    let mut env_map = HashMap::new();

    if let Ok(root) = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
    {
        let legacy_env = root.join(".env");
        if legacy_env.exists() {
            if let Ok(map) = read_legacy_env(legacy_env) {
                for (key, value) in map {
                    env_map.insert(key, value);
                }
            }
        }
    }

    if let Ok(config_dir) = app.path().app_config_dir() {
        let secure_path = config_dir.join("secure-config.json");
        if secure_path.exists() {
            let provider = KeyringProvider::new("com.gemini.balance.desktop", "master-key");
            let store = SecureConfigStore::new(provider, &secure_path);
            if let Ok(Some(map)) = store.load::<HashMap<String, String>>() {
                for (key, value) in map {
                    env_map.insert(key, value);
                }
            }
        }
    }

    env_map
}

fn find_available_port(start: u16, end: u16) -> Option<u16> {
    (start..=end).find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
}

fn setup_tray(app: &App) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &quit_item])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "show" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .manage(ProcessState::default())
        .setup(|app| {
            setup_tray(app)?;
            if std::env::var("GB_AUTO_START")
                .unwrap_or_else(|_| "true".to_string())
                .to_lowercase()
                != "false"
            {
                let state = app.state::<ProcessState>();
                let _ = start_gateway_impl(&app.handle(), &state);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            start_gateway,
            stop_gateway,
            gateway_status,
            runtime_base_url,
            import_legacy_env,
            auth_token_hint
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
