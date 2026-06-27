use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    env, fs,
    path::{Path, PathBuf},
    sync::Mutex,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager, PhysicalSize, WebviewWindow, WindowEvent,
};

#[derive(Default)]
struct AppState {
    previous_window: Mutex<Option<isize>>,
}

const PICKER_PHYSICAL_WIDTH: u32 = 360;
const PICKER_PHYSICAL_HEIGHT: u32 = 350;
const SETTINGS_PHYSICAL_WIDTH: u32 = 800;
const SETTINGS_PHYSICAL_HEIGHT: u32 = 700;
const HOOK_SCRIPT_NAME: &str = "prompt-flow-stop.ps1";
const RUN_ID_PREFIX: &str = "pf";
const HIDE_BEFORE_PASTE_DELAY_MS: u64 = 90;
const TARGET_FOCUS_DELAY_MS: u64 = 220;
const SUBMIT_AFTER_PASTE_DELAY_MS: u64 = 320;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PromptItem {
    id: String,
    title: String,
    category: String,
    content: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FlowItem {
    id: String,
    title: String,
    steps: Vec<String>,
    cursor: usize,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PromptStore {
    prompts: Vec<PromptItem>,
    flows: Vec<FlowItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FlowRunStart {
    title: String,
    steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FlowRunLaunch {
    run_id: String,
    first_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FlowHookStatus {
    codex_installed: bool,
    claude_installed: bool,
    codex_stale: bool,
    claude_stale: bool,
    codex_config_path: String,
    claude_config_path: String,
    script_path: String,
    state_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FlowHookInstallResult {
    client: String,
    installed: bool,
    config_path: String,
    script_path: String,
    backup_path: Option<String>,
    next_step: String,
}

fn default_store() -> PromptStore {
    PromptStore {
        prompts: vec![
            PromptItem {
                id: "explain".into(),
                title: "explain".into(),
                category: "General".into(),
                content: "Can you explain what this is, why it matters, and give a simple example?".into(),
                updated_at: "2026-01-01T00:00:00Z".into(),
            },
            PromptItem {
                id: "review".into(),
                title: "review".into(),
                category: "General".into(),
                content: "Review this carefully. Find concrete issues first, then suggest the smallest useful fix.".into(),
                updated_at: "2026-01-01T00:00:00Z".into(),
            },
            PromptItem {
                id: "plan".into(),
                title: "plan".into(),
                category: "General".into(),
                content: "Make a concise implementation plan. Keep it practical and list risks or unknowns.".into(),
                updated_at: "2026-01-01T00:00:00Z".into(),
            },
        ],
        flows: vec![FlowItem {
            id: "idea-check".into(),
            title: "idea-check".into(),
            steps: vec!["explain".into(), "review".into(), "plan".into()],
            cursor: 0,
            updated_at: "2026-01-01T00:00:00Z".into(),
        }],
    }
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data_dir(app)?.join("prompts.json"))
}

fn app_data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| format!("Could not resolve app data directory: {error}"))?;
    migrate_legacy_app_data(&dir)?;
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create app data directory: {error}"))?;
    Ok(dir)
}

fn migrate_legacy_app_data(new_dir: &Path) -> Result<(), String> {
    if new_dir.exists() {
        return Ok(());
    }
    let Some(appdata) = env::var_os("APPDATA").map(PathBuf::from) else {
        return Ok(());
    };
    let old_dir = appdata.join(legacy_app_identifier());
    if !old_dir.exists() {
        return Ok(());
    }
    // Keep existing user prompts after the app id changed from the prototype name.
    copy_dir_if_missing(&old_dir, new_dir)
}

fn legacy_app_identifier() -> String {
    format!("dev.prompt{}{}.desktop", "a", "ir")
}

fn copy_dir_if_missing(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to)
        .map_err(|error| format!("Could not create {}: {error}", to.display()))?;
    for entry in
        fs::read_dir(from).map_err(|error| format!("Could not read {}: {error}", from.display()))?
    {
        let entry =
            entry.map_err(|error| format!("Could not read legacy app data entry: {error}"))?;
        let source = entry.path();
        let target = to.join(entry.file_name());
        if target.exists() {
            continue;
        }
        if source.is_dir() {
            copy_dir_if_missing(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|error| {
                format!(
                    "Could not migrate {} to {}: {error}",
                    source.display(),
                    target.display()
                )
            })?;
        }
    }
    Ok(())
}

fn flow_runs_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("flow-runs");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create flow run directory: {error}"))?;
    Ok(dir)
}

fn active_flow_run_path(app: &AppHandle) -> Result<PathBuf, String> {
    // The Stop hook runs in a separate process, so it resumes flows through this shared state file.
    Ok(flow_runs_dir(app)?.join("active.json"))
}

fn hook_script_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data_dir(app)?.join("hooks");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("Could not create hook directory: {error}"))?;
    Ok(dir.join(HOOK_SCRIPT_NAME))
}

fn user_home_dir() -> Result<PathBuf, String> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| "Could not resolve user home directory".to_string())
}

fn codex_hooks_path() -> Result<PathBuf, String> {
    Ok(user_home_dir()?.join(".codex").join("hooks.json"))
}

fn claude_settings_path() -> Result<PathBuf, String> {
    Ok(user_home_dir()?.join(".claude").join("settings.json"))
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn now_label() -> String {
    unix_timestamp().to_string()
}

fn new_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{RUN_ID_PREFIX}-{nanos:x}")
}

fn shell_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\\\""))
}

fn flow_hook_command(app: &AppHandle, client: &str) -> Result<String, String> {
    let script = hook_script_path(app)?;
    let state_dir = app_data_dir(app)?;
    // Keep hook arguments stable because Codex asks users to trust the exact command string.
    Ok(format!(
        "powershell.exe -NoProfile -ExecutionPolicy Bypass -File {} --client {} --state-dir {}",
        shell_quote(&script.to_string_lossy()),
        shell_quote(client),
        shell_quote(&state_dir.to_string_lossy())
    ))
}

fn flow_hook_script() -> &'static str {
    include_str!("prompt_flow_stop.ps1")
}

fn ensure_hook_script(app: &AppHandle) -> Result<PathBuf, String> {
    let path = hook_script_path(app)?;
    fs::write(&path, flow_hook_script().trim_start())
        .map_err(|error| format!("Could not write hook script: {error}"))?;
    Ok(path)
}

fn read_json_object(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let data = fs::read_to_string(path)
        .map_err(|error| format!("Could not read {}: {error}", path.display()))?;
    if data.trim().is_empty() {
        return Ok(json!({}));
    }
    let parsed: Value = serde_json::from_str(&data)
        .map_err(|error| format!("Could not parse {}: {error}", path.display()))?;
    Ok(if parsed.is_object() {
        parsed
    } else {
        json!({})
    })
}

fn backup_file(path: &Path) -> Result<Option<PathBuf>, String> {
    if !path.exists() {
        return Ok(None);
    }
    let backup = path.with_extension(format!("json.prompt-flow-{}.bak", now_label()));
    fs::copy(path, &backup)
        .map_err(|error| format!("Could not back up {}: {error}", path.display()))?;
    Ok(Some(backup))
}

fn remove_flow_hooks(event_groups: &mut Vec<Value>) {
    let legacy_hook_name = legacy_hook_script_name();
    for group in event_groups.iter_mut() {
        let Some(hooks) = group.get_mut("hooks").and_then(Value::as_array_mut) else {
            continue;
        };
        hooks.retain(|hook| {
            let command = hook
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_lowercase();
            let command_windows = hook
                .get("commandWindows")
                .or_else(|| hook.get("command_windows"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_lowercase();
            // Remove both current and prototype hook names so reinstalling never creates duplicates.
            !(command.contains(HOOK_SCRIPT_NAME)
                || command_windows.contains(HOOK_SCRIPT_NAME)
                || command.contains(&legacy_hook_name)
                || command_windows.contains(&legacy_hook_name))
        });
    }
    event_groups.retain(|group| {
        group
            .get("hooks")
            .and_then(Value::as_array)
            .map(|hooks| !hooks.is_empty())
            .unwrap_or(true)
    });
}

fn legacy_hook_script_name() -> String {
    format!("prompt-{}-stop.ps1", "air")
}

fn install_hook_config(app: &AppHandle, client: &str) -> Result<FlowHookInstallResult, String> {
    let script_path = ensure_hook_script(app)?;
    let config_path = match client {
        "codex" => codex_hooks_path()?,
        "claude" => claude_settings_path()?,
        _ => return Err("Unknown hook client".into()),
    };

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create config directory: {error}"))?;
    }

    let backup_path = backup_file(&config_path)?;
    let mut root = read_json_object(&config_path)?;
    let command = flow_hook_command(app, client)?;
    let hook_entry = if client == "codex" {
        json!({
            "type": "command",
            "command": command,
            "commandWindows": command,
            "timeout": 10,
            "statusMessage": "prompt-flow"
        })
    } else {
        json!({
            "type": "command",
            "command": command,
            "timeout": 10
        })
    };

    if !root.get("hooks").map(Value::is_object).unwrap_or(false) {
        root["hooks"] = json!({});
    }
    if !root["hooks"]
        .get("Stop")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        root["hooks"]["Stop"] = json!([]);
    }

    // Replace only our previous hook entry and leave unrelated user hooks intact.
    let stop_groups = root["hooks"]["Stop"]
        .as_array_mut()
        .ok_or_else(|| "Could not create Stop hook config".to_string())?;
    remove_flow_hooks(stop_groups);
    stop_groups.push(json!({
        "hooks": [hook_entry]
    }));

    let data = serde_json::to_string_pretty(&root)
        .map_err(|error| format!("Could not serialize hook config: {error}"))?;
    fs::write(&config_path, data)
        .map_err(|error| format!("Could not write {}: {error}", config_path.display()))?;

    Ok(FlowHookInstallResult {
        client: client.into(),
        installed: true,
        config_path: config_path.to_string_lossy().to_string(),
        script_path: script_path.to_string_lossy().to_string(),
        backup_path: backup_path.map(|path| path.to_string_lossy().to_string()),
        next_step: if client == "codex" {
            "Open Codex and run /hooks, then trust the prompt-flow hook.".into()
        } else {
            "Restart Claude Code if it was already open.".into()
        },
    })
}

fn config_contains_flow_hook(path: &Path) -> bool {
    let Ok(data) = fs::read_to_string(path) else {
        return false;
    };
    let data = data.to_lowercase();
    data.contains(HOOK_SCRIPT_NAME)
}

fn config_contains_legacy_flow_hook(path: &Path) -> bool {
    let Ok(data) = fs::read_to_string(path) else {
        return false;
    };
    data.to_lowercase().contains(&legacy_hook_script_name())
}

fn center_picker(window: &WebviewWindow) -> Result<(), String> {
    window
        .set_size(PhysicalSize::new(
            PICKER_PHYSICAL_WIDTH,
            PICKER_PHYSICAL_HEIGHT,
        ))
        .map_err(|error| format!("Could not size picker: {error}"))?;
    window
        .center()
        .map_err(|error| format!("Could not center picker: {error}"))
}

fn center_settings(window: &WebviewWindow) -> Result<(), String> {
    window
        .set_size(PhysicalSize::new(
            SETTINGS_PHYSICAL_WIDTH,
            SETTINGS_PHYSICAL_HEIGHT,
        ))
        .map_err(|error| format!("Could not size settings: {error}"))?;
    window
        .center()
        .map_err(|error| format!("Could not center settings: {error}"))
}

#[tauri::command]
fn load_store(app: AppHandle) -> Result<PromptStore, String> {
    let path = store_path(&app)?;
    if !path.exists() {
        let store = default_store();
        let data = serde_json::to_string_pretty(&store)
            .map_err(|error| format!("Could not serialize default store: {error}"))?;
        fs::write(&path, data)
            .map_err(|error| format!("Could not write default store: {error}"))?;
        return Ok(store);
    }

    let data = fs::read_to_string(&path)
        .map_err(|error| format!("Could not read prompt store: {error}"))?;
    serde_json::from_str(&data).map_err(|error| format!("Could not parse prompt store: {error}"))
}

#[tauri::command]
fn save_store(app: AppHandle, store: PromptStore) -> Result<(), String> {
    let path = store_path(&app)?;
    let data = serde_json::to_string_pretty(&store)
        .map_err(|error| format!("Could not serialize prompt store: {error}"))?;
    fs::write(&path, data).map_err(|error| format!("Could not save prompt store: {error}"))
}

#[tauri::command]
fn flow_hook_status(app: AppHandle) -> Result<FlowHookStatus, String> {
    let script_path = hook_script_path(&app)?;
    let state_dir = app_data_dir(&app)?;
    let codex_config_path = codex_hooks_path()?;
    let claude_config_path = claude_settings_path()?;
    let codex_installed = config_contains_flow_hook(&codex_config_path);
    let claude_installed = config_contains_flow_hook(&claude_config_path);
    Ok(FlowHookStatus {
        codex_installed,
        claude_installed,
        codex_stale: !codex_installed && config_contains_legacy_flow_hook(&codex_config_path),
        claude_stale: !claude_installed && config_contains_legacy_flow_hook(&claude_config_path),
        codex_config_path: codex_config_path.to_string_lossy().to_string(),
        claude_config_path: claude_config_path.to_string_lossy().to_string(),
        script_path: script_path.to_string_lossy().to_string(),
        state_dir: state_dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
fn install_flow_hook(app: AppHandle, client: String) -> Result<FlowHookInstallResult, String> {
    install_hook_config(&app, client.as_str())
}

#[tauri::command]
fn start_flow_run(
    app: AppHandle,
    state: tauri::State<AppState>,
    flow: FlowRunStart,
) -> Result<FlowRunLaunch, String> {
    ensure_hook_script(&app)?;

    // Capture happens when the picker opens; the hook later reuses this same target window.
    let target_hwnd = *state
        .previous_window
        .lock()
        .map_err(|_| "Could not lock previous window state".to_string())?;
    let target_hwnd = target_hwnd.ok_or_else(|| {
        "No target window captured. Focus Codex or Claude Code, then press Ctrl+Alt+P to open prompt-flow.".to_string()
    })?;

    let steps: Vec<String> = flow
        .steps
        .into_iter()
        .map(|step| step.trim().to_string())
        .filter(|step| !step.is_empty())
        .collect();
    if steps.is_empty() {
        return Err("Flow has no usable steps".into());
    }

    let run_id = new_run_id();
    let title = match flow.title.trim() {
        "" => "Untitled flow",
        value => value,
    };
    let first_prompt = steps[0].clone();
    let now = unix_timestamp();
    // Step 0 is sent immediately by the app. The hook advances this file after each answer.
    let run = json!({
        "run_id": run_id,
        "title": title,
        "status": "active",
        "current_step": 0,
        "steps": steps,
        "target_hwnd": target_hwnd,
        "session_id": "",
        "transcript_path": "",
        "created_at": now,
        "updated_at": now
    });
    let path = active_flow_run_path(&app)?;
    let data = serde_json::to_string_pretty(&run)
        .map_err(|error| format!("Could not serialize flow run: {error}"))?;
    fs::write(&path, data).map_err(|error| format!("Could not write active flow run: {error}"))?;

    Ok(FlowRunLaunch {
        run_id,
        first_prompt,
    })
}

#[tauri::command]
fn show_picker(app: AppHandle, state: tauri::State<AppState>) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    if window
        .is_visible()
        .map_err(|error| format!("Could not inspect window state: {error}"))?
    {
        window
            .hide()
            .map_err(|error| format!("Could not hide window: {error}"))?;
        return Ok(());
    }

    // Remember the CLI before showing prompt-flow so paste/flow actions know where to return.
    remember_foreground_window(&state);
    let _ = window.emit("picker-opened", ());
    window
        .unminimize()
        .map_err(|error| format!("Could not restore window: {error}"))?;
    center_picker(&window)?;
    window
        .show()
        .map_err(|error| format!("Could not show window: {error}"))?;
    center_picker(&window)?;
    window
        .set_focus()
        .map_err(|error| format!("Could not focus window: {error}"))?;
    Ok(())
}

#[tauri::command]
fn show_settings(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    let _ = window.emit("settings-opened", ());
    window
        .unminimize()
        .map_err(|error| format!("Could not restore settings: {error}"))?;
    // Center after restore/show as Windows may ignore geometry changes while minimized.
    center_settings(&window)?;
    window
        .show()
        .map_err(|error| format!("Could not show settings: {error}"))?;
    center_settings(&window)?;
    window
        .set_focus()
        .map_err(|error| format!("Could not focus settings: {error}"))
}

#[tauri::command]
fn minimize_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;
    window
        .minimize()
        .map_err(|error| format!("Could not minimize window: {error}"))
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only http and https links can be opened".into());
    }
    open_url_with_system(&url)
}

fn quit_app(app: &AppHandle) {
    app.exit(0);
}

fn focus_existing_settings_window(app: &AppHandle) {
    let Some(window) = app.get_webview_window("main") else {
        return;
    };
    let _ = window.unminimize();
    let _ = center_settings(&window);
    let _ = window.show();
    let _ = center_settings(&window);
    let _ = window.set_focus();
    let _ = window.emit("settings-opened", ());
}

#[cfg(target_os = "windows")]
fn open_url_with_system(url: &str) -> Result<(), String> {
    std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn()
        .map_err(|error| format!("Could not open link: {error}"))?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_url_with_system(url: &str) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(url)
        .spawn()
        .map_err(|error| format!("Could not open link: {error}"))?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_url_with_system(url: &str) -> Result<(), String> {
    std::process::Command::new("xdg-open")
        .arg(url)
        .spawn()
        .map_err(|error| format!("Could not open link: {error}"))?;
    Ok(())
}

#[tauri::command]
fn insert_text(
    app: AppHandle,
    state: tauri::State<AppState>,
    text: String,
    submit: bool,
) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "Main window not found".to_string())?;

    let target = *state
        .previous_window
        .lock()
        .map_err(|_| "Could not lock previous window state".to_string())?;

    let mut clipboard =
        arboard::Clipboard::new().map_err(|error| format!("Clipboard unavailable: {error}"))?;
    clipboard
        .set_text(text)
        .map_err(|error| format!("Could not set clipboard text: {error}"))?;

    // Hide first so the captured CLI regains visual priority before Ctrl+V/Enter are sent.
    window
        .hide()
        .map_err(|error| format!("Could not hide picker: {error}"))?;
    thread::sleep(Duration::from_millis(HIDE_BEFORE_PASTE_DELAY_MS));

    #[cfg(target_os = "windows")]
    {
        let hwnd = target.ok_or_else(|| {
            "No target window captured. Focus Codex or Claude Code, then press Ctrl+Alt+P to open prompt-flow.".to_string()
        })?;
        restore_window_and_paste(hwnd, submit)?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = target;
        return Err("Auto-insert is currently implemented for Windows only.".into());
    }

    Ok(())
}

fn remember_foreground_window(state: &tauri::State<AppState>) {
    #[cfg(target_os = "windows")]
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        let hwnd = GetForegroundWindow();
        if !hwnd.0.is_null() {
            if let Ok(mut previous) = state.previous_window.lock() {
                *previous = Some(hwnd.0 as isize);
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn restore_window_and_paste(hwnd: isize, submit: bool) -> Result<(), String> {
    use windows::Win32::{
        Foundation::HWND,
        UI::{
            Input::KeyboardAndMouse::{SendInput, INPUT, VK_CONTROL, VK_RETURN, VK_V},
            WindowsAndMessaging::{
                BringWindowToTop, GetForegroundWindow, IsIconic, IsWindow, SetForegroundWindow,
                ShowWindow, SW_RESTORE,
            },
        },
    };

    let hwnd = HWND(hwnd as *mut core::ffi::c_void);
    unsafe {
        if !IsWindow(Some(hwnd)).as_bool() {
            return Err("The captured target window is no longer available. Focus Codex or Claude Code and open prompt-flow again.".into());
        }
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = BringWindowToTop(hwnd);
        let _ = SetForegroundWindow(hwnd);
    }
    thread::sleep(Duration::from_millis(TARGET_FOCUS_DELAY_MS));

    let focused = unsafe { GetForegroundWindow() };
    if focused.0 != hwnd.0 {
        return Err("Could not focus the captured target window. Click Codex or Claude Code and open prompt-flow again.".into());
    }

    let mut paste_inputs = [
        key_input(VK_CONTROL, false),
        key_input(VK_V, false),
        key_input(VK_V, true),
        key_input(VK_CONTROL, true),
    ];

    let sent = unsafe { SendInput(&mut paste_inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err("Could not send paste shortcut".into());
    }

    if submit {
        thread::sleep(Duration::from_millis(SUBMIT_AFTER_PASTE_DELAY_MS));
        let mut enter_inputs = [key_input(VK_RETURN, false), key_input(VK_RETURN, true)];
        let sent = unsafe { SendInput(&mut enter_inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent == 0 {
            return Err("Could not send Enter key".into());
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn key_input(
    key: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY,
    up: bool,
) -> windows::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    };

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: if up {
                    KEYEVENTF_KEYUP
                } else {
                    Default::default()
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            focus_existing_settings_window(app);
        }))
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            #[cfg(any(target_os = "macos", windows, target_os = "linux"))]
            {
                use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
                let shortcut = Shortcut::new(Some(Modifiers::CONTROL | Modifiers::ALT), Code::KeyP);
                app.handle()
                    .plugin(tauri_plugin_global_shortcut::Builder::new().build())?;
                if let Err(error) =
                    app.global_shortcut()
                        .on_shortcut(shortcut, |app, _shortcut, event| {
                            if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed
                            {
                                let state = app.state::<AppState>();
                                let _ = show_picker(app.clone(), state);
                            }
                        })
                {
                    log::warn!("Could not register Ctrl+Alt+P global shortcut: {error}");
                }
            }

            let settings_item = MenuItemBuilder::with_id("settings", "Settings").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let tray_menu = MenuBuilder::new(app)
                .item(&settings_item)
                .separator()
                .item(&quit_item)
                .build()?;
            let tray_icon = app
                .default_window_icon()
                .cloned()
                .ok_or_else(|| "Default app icon is unavailable".to_string())?;

            TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .tooltip("prompt-flow")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "settings" => {
                        let _ = show_settings(app.clone());
                    }
                    "quit" => quit_app(app),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        let _ = show_settings(app.clone());
                    }
                })
                .build(app)?;

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.minimize();
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_store,
            save_store,
            flow_hook_status,
            install_flow_hook,
            start_flow_run,
            show_picker,
            show_settings,
            minimize_window,
            open_external_url,
            insert_text
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
