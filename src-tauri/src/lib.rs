// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use serde::Serialize;
use tauri::{Manager, RunEvent, State, WebviewUrl, WebviewWindowBuilder};

/// Bundled runtime paths inside the app bundle's Resources dir.
/// Populated by `scripts/prepare-runtime.mjs` before `tauri build`.
/// Tauri preserves the `resources/` prefix from tauri.conf.json.
const BUNDLED_NODE_REL: &str = "resources/node/bin/node";
const BUNDLED_DSH_REL: &str = "resources/dsh/node_modules/@deepseek-ai/dsh/lib/bin.js";

/// Boot status surfaced to the loading page via the `boot_status` command.
#[derive(Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum BootStatus {
    Starting,
    Ready { url: String },
    Failed { message: String },
}

struct AppState {
    status: Mutex<BootStatus>,
    child: Mutex<Option<Child>>,
}

/// The loading page polls this command every few hundred ms.
#[tauri::command]
fn boot_status(state: State<'_, Arc<AppState>>) -> BootStatus {
    state.status.lock().unwrap().clone()
}

/// Resolve the absolute path of the `node` binary.
///
/// Priority: bundled runtime in the app's Resources dir, then
/// `DSH_CLIENT_NODE_BIN` env, then system search (PATH / known locations).
/// Finder-launched .app bundles get a minimal PATH, so the bundled runtime
/// makes the app work out of the box.
fn resolve_node(resource_dir: &Path) -> Option<String> {
    let bundled = resource_dir.join(BUNDLED_NODE_REL);
    if bundled.is_file() {
        return Some(bundled.to_string_lossy().into_owned());
    }
    if let Ok(p) = std::env::var("DSH_CLIENT_NODE_BIN") {
        if Path::new(&p).exists() {
            return Some(p);
        }
    }
    if let Ok(out) = Command::new("sh").arg("-c").arg("command -v node").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() && Path::new(&s).exists() {
                return Some(s);
            }
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    for c in [
        "/opt/homebrew/bin/node".to_string(),
        "/usr/local/bin/node".to_string(),
        "/usr/bin/node".to_string(),
        format!("{home}/.nvm/versions/node/current/bin/node"),
        format!("{home}/.vite-plus/js_runtime/node/current/bin/node"),
    ] {
        if Path::new(&c).exists() {
            return Some(c);
        }
    }
    // Latest installed nvm / vite-plus runtime node. The version dirs may be
    // mixed with stray files (e.g. `index_cache.json`, `*.lock`), so scan every
    // child, keep only those that actually contain `bin/node`, newest mtime
    // wins — never sort by filename and take the last entry.
    for base in [
        format!("{home}/.nvm/versions/node"),
        format!("{home}/.vite-plus/js_runtime/node"),
    ] {
        if let Ok(rd) = std::fs::read_dir(&base) {
            let mut candidates: Vec<(SystemTime, String)> = Vec::new();
            for entry in rd.flatten() {
                let bin = entry.path().join("bin/node");
                if !bin.is_file() {
                    continue;
                }
                let mtime = std::fs::metadata(&bin)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                candidates.push((mtime, bin.to_string_lossy().into_owned()));
            }
            candidates.sort_by_key(|(t, _)| *t);
            if let Some((_, p)) = candidates.last() {
                return Some(p.clone());
            }
        }
    }
    None
}

/// Resolve the absolute path of the `dsh` launcher script.
fn resolve_dsh(resource_dir: &Path) -> Option<String> {
    let bundled = resource_dir.join(BUNDLED_DSH_REL);
    if bundled.is_file() {
        return Some(bundled.to_string_lossy().into_owned());
    }
    if let Ok(p) = std::env::var("DSH_CLIENT_DSH_BIN") {
        if Path::new(&p).exists() {
            return Some(p);
        }
    }
    if let Ok(out) = Command::new("sh").arg("-c").arg("command -v dsh").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() && Path::new(&s).exists() {
                return Some(s);
            }
        }
    }
    // npx cache: ~/.npm/_npx/<hash>/node_modules/.bin/dsh (take the newest).
    let home = std::env::var("HOME").unwrap_or_default();
    let npx = format!("{home}/.npm/_npx");
    if let Ok(rd) = std::fs::read_dir(&npx) {
        let mut found: Vec<(SystemTime, String)> = Vec::new();
        for e in rd.flatten() {
            let bin = e.path().join("node_modules/.bin/dsh");
            if bin.exists() {
                let mtime = std::fs::metadata(&bin)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let canon = std::fs::canonicalize(&bin).unwrap_or(bin);
                found.push((mtime, canon.to_string_lossy().into_owned()));
            }
        }
        found.sort_by_key(|(t, _)| *t);
        if let Some((_, p)) = found.last() {
            return Some(p.clone());
        }
    }
    None
}

/// Spawn `dsh web --port 0` and return the child plus its piped stdout.
fn spawn_dsh(resource_dir: &Path) -> Result<(Child, BufReader<std::process::ChildStdout>), String> {
    let node = resolve_node(resource_dir).ok_or_else(|| {
        "未找到 Node.js：请安装 Node.js，或设置环境变量 DSH_CLIENT_NODE_BIN".to_string()
    })?;
    let dsh = resolve_dsh(resource_dir).ok_or_else(|| {
        "未找到 dsh 命令：请安装 @deepseek-ai/dsh，或设置环境变量 DSH_CLIENT_DSH_BIN".to_string()
    })?;

    let mut cmd = Command::new(&node);
    cmd.arg(&dsh)
        .args(["web", "--port", "0", "--host", "127.0.0.1"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let ws = std::env::var("DSH_CLIENT_WORKSPACE")
        .unwrap_or_else(|_| std::env::var("HOME").unwrap_or_else(|_| "/".into()));
    cmd.current_dir(ws);

    let mut child = cmd.spawn().map_err(|e| format!("启动 dsh 失败: {e}"))?;
    let stdout = child.stdout.take().ok_or_else(|| "无法读取 dsh 输出".to_string())?;
    Ok((child, BufReader::new(stdout)))
}

/// Background thread: read `dsh web` stdout, parse the printed URL, and
/// update the boot status. stderr is drained so the child never blocks.
fn boot_worker(app_state: Arc<AppState>, resource_dir: std::path::PathBuf) {
    let (mut child, reader) = match spawn_dsh(&resource_dir) {
        Ok(v) => v,
        Err(e) => {
            *app_state.status.lock().unwrap() = BootStatus::Failed { message: e };
            return;
        }
    };

    // Drain stderr in a side thread so a full pipe can never wedge the child.
    let stderr_holder = Arc::new(Mutex::new(String::new()));
    if let Some(err) = child.stderr.take() {
        let holder = stderr_holder.clone();
        std::thread::spawn(move || {
            for line in BufReader::new(err).lines() {
                if let Ok(l) = line {
                    *holder.lock().unwrap() = l;
                }
            }
        });
    }

    // The child is fully owned by the state now; the reader below borrows the
    // already-taken stdout pipe, which stays alive independently.
    *app_state.child.lock().unwrap() = Some(child);

    let mut saw_url = false;
    for line in reader.lines() {
        match line {
            Ok(l) => {
                if !saw_url {
                    if let Some(rest) = l.strip_prefix("dsh web: ") {
                        let url = rest.trim().to_string();
                        saw_url = true;
                        *app_state.status.lock().unwrap() = BootStatus::Ready { url };
                    }
                }
                // Keep draining stdout after the URL line so the child never
                // hits EPIPE and we notice an early exit.
            }
            Err(_) => break,
        }
    }

    // EOF: the server process exited.
    if saw_url {
        let msg = {
            let err = stderr_holder.lock().unwrap().clone();
            if err.is_empty() {
                "dsh 服务已退出".to_string()
            } else {
                format!("dsh 服务已退出：{err}")
            }
        };
        *app_state.status.lock().unwrap() = BootStatus::Failed { message: msg };
    } else {
        let msg = {
            let err = stderr_holder.lock().unwrap().clone();
            if err.is_empty() {
                "dsh 启动失败：未收到服务地址".to_string()
            } else {
                format!("dsh 启动失败：{err}")
            }
        };
        *app_state.status.lock().unwrap() = BootStatus::Failed { message: msg };
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(Arc::new(AppState {
            status: Mutex::new(BootStatus::Starting),
            child: Mutex::new(None),
        }))
        .invoke_handler(tauri::generate_handler![boot_status])
        .setup(|app| {
            // Kick off the dsh server first so it warms up while the window opens.
            let app_state = app.state::<Arc<AppState>>().inner().clone();
            let resource_dir = app
                .path()
                .resource_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            std::thread::spawn(move || boot_worker(app_state, resource_dir));

            WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("DeepSeek Harness")
                .inner_size(1280.0, 800.0)
                .min_inner_size(900.0, 600.0)
                .build()?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                window.app_handle().exit(0);
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // SIGTERM/SIGINT (e.g. `kill` from terminal) would otherwise terminate the
    // process without running cleanup. Convert them into a graceful exit so the
    // dsh child is killed via the RunEvent::Exit handler below.
    {
        let handle = app.handle().clone();
        std::thread::spawn(move || {
            if let Ok(mut signals) = signal_hook::iterator::Signals::new([
                signal_hook::consts::SIGTERM,
                signal_hook::consts::SIGINT,
            ]) {
                for _ in signals.forever() {
                    handle.exit(0);
                    break;
                }
            }
        });
    }

    app.run(|app_handle, event| {
        if let RunEvent::Exit = event {
            if let Some(state) = app_handle.try_state::<Arc<AppState>>() {
                if let Some(mut child) = state.child.lock().unwrap().take() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
    });
}
