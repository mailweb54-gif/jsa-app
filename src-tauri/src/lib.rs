// JSA Channels - app desktop Windows/Mac (21/11/2026)
//
// Apre la dashboard JSA in una finestra dedicata (motore web del sistema: WebView2 su Windows,
// WebKit su Mac) e gestisce lei la cartella dei download: si sceglie una volta, resta memorizzata
// in settings.json e non servono permessi da riconfermare. La dashboard la usa tramite i comandi
// qui sotto (blocco "jsaapp-20261121" in app/legacy/index.html).

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{
    ipc::{InvokeBody, Request},
    webview::DownloadEvent,
    AppHandle, Manager, State, WebviewUrl, WebviewWindowBuilder,
};
use tauri_plugin_dialog::DialogExt;

const DEFAULT_URL: &str = "https://music.jsadjsteam.com/";

#[derive(Default, Serialize, Deserialize, Clone)]
struct Settings {
    /// cartella dove salvare i brani
    #[serde(default)]
    folder: Option<String>,
    /// indirizzo della dashboard (vuoto = music.jsadjsteam.com)
    #[serde(default)]
    url: Option<String>,
    /// XML della libreria scelto a mano (vuoto = cerca da solo quello di Musica/iTunes)
    #[serde(default)]
    xml: Option<String>,
    /// ultimo XML mandato al motore: "utente|percorso|firma"
    #[serde(default)]
    xml_sent: Option<String>,
}

struct AppState {
    settings: Mutex<Settings>,
    file: PathBuf,
}

impl AppState {
    fn store(&self) {
        let s = self.settings.lock().unwrap().clone();
        if let Some(dir) = self.file.parent() {
            let _ = fs::create_dir_all(dir);
        }
        if let Ok(txt) = serde_json::to_string_pretty(&s) {
            let _ = fs::write(&self.file, txt);
        }
    }
    fn folder(&self) -> Option<PathBuf> {
        self.settings
            .lock()
            .unwrap()
            .folder
            .clone()
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
    }
}

#[derive(Serialize)]
struct FolderInfo {
    path: String,
    name: String,
}

fn info_of(p: &Path) -> FolderInfo {
    let name = p
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| p.to_string_lossy().to_string());
    FolderInfo { path: p.to_string_lossy().to_string(), name }
}

/// Solo il nome del file, senza percorsi e senza caratteri vietati da Windows/Mac.
fn clean_name(raw: &str) -> String {
    let base = raw.rsplit(|c| c == '/' || c == '\\').next().unwrap_or("");
    let mut s: String = base
        .chars()
        .map(|c| if c.is_control() || "<>:\"|?*".contains(c) { '_' } else { c })
        .collect();
    while s.ends_with('.') || s.ends_with(' ') {
        s.pop();
    }
    let s2 = s.trim_start().to_string();
    if s2.is_empty() || s2 == "." || s2 == ".." {
        "brano.mp3".to_string()
    } else {
        s2
    }
}

#[derive(Serialize)]
struct AppInfo {
    app: &'static str,
    version: &'static str,
    os: &'static str,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo { app: "JSA Channels", version: env!("CARGO_PKG_VERSION"), os: std::env::consts::OS }
}

#[tauri::command]
fn get_folder(state: State<'_, AppState>) -> Option<FolderInfo> {
    state.folder().map(|p| info_of(&p))
}

#[tauri::command]
async fn pick_folder(app: AppHandle, state: State<'_, AppState>) -> Result<Option<FolderInfo>, String> {
    let start = state.folder().or_else(|| app.path().desktop_dir().ok());
    let mut dlg = app.dialog().file().set_title("Cartella dove salvare i brani");
    if let Some(s) = start {
        dlg = dlg.set_directory(s);
    }
    let picked = match dlg.blocking_pick_folder() {
        Some(fp) => fp.into_path().map_err(|e| e.to_string())?,
        None => return Ok(None),
    };
    state.settings.lock().unwrap().folder = Some(picked.to_string_lossy().to_string());
    state.store();
    Ok(Some(info_of(&picked)))
}

#[tauri::command]
fn file_exists(state: State<'_, AppState>, name: String) -> bool {
    match state.folder() {
        Some(d) => d.join(clean_name(&name)).is_file(),
        None => false,
    }
}

/// Riceve i byte del brano (corpo "raw") e il nome nell'intestazione x-name (encodeURIComponent).
#[tauri::command]
async fn save_file(state: State<'_, AppState>, request: Request<'_>) -> Result<String, String> {
    let dir = state.folder().ok_or("Nessuna cartella scelta (o cartella non trovata)")?;
    let raw_name = request
        .headers()
        .get("x-name")
        .and_then(|v| v.to_str().ok())
        .ok_or("Nome del file mancante")?;
    let name = clean_name(&percent_encoding::percent_decode_str(raw_name).decode_utf8_lossy());
    let bytes = match request.body() {
        InvokeBody::Raw(b) => b,
        _ => return Err("Contenuto del file mancante".into()),
    };
    let dest = dir.join(&name);
    let tmp = dir.join(format!("{}.part", name));
    fs::write(&tmp, bytes).map_err(|e| format!("Scrittura non riuscita: {e}"))?;
    if let Err(e) = fs::rename(&tmp, &dest) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Scrittura non riuscita: {e}"));
    }
    Ok(dest.to_string_lossy().to_string())
}

#[tauri::command]
fn open_folder(state: State<'_, AppState>) -> Result<(), String> {
    let d = state.folder().ok_or("Nessuna cartella scelta")?;
    #[cfg(target_os = "windows")]
    let prog = "explorer";
    #[cfg(target_os = "macos")]
    let prog = "open";
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let prog = "xdg-open";
    std::process::Command::new(prog).arg(d).spawn().map_err(|e| e.to_string())?;
    Ok(())
}

// ---------- XML della libreria (21/11/2026) ----------
// L'app trova l'XML (scelto a mano, oppure quello che Musica/iTunes aggiorna da solo) e lo passa alla dashboard
// solo se e' cambiato dall'ultimo invio: la dashboard lo manda al motore (/api/rbindex) con la sessione gia' aperta,
// cosi' il segno "gia' nella mia libreria" resta aggiornato senza fare nulla.

/// XML che Musica/iTunes tengono aggiornati da soli (Musica: Impostazioni > File > "Condividi XML della libreria").
fn auto_xml_candidates(app: &AppHandle) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(audio) = app.path().audio_dir() {
        v.push(audio.join("Music").join("Library.xml"));
        v.push(audio.join("iTunes").join("iTunes Music Library.xml"));
        v.push(audio.join("iTunes").join("iTunes Library.xml"));
    }
    v
}

fn mtime_ms(p: &Path) -> u128 {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

/// (percorso, scelto a mano?)
fn current_xml(app: &AppHandle, state: &AppState) -> Option<(PathBuf, bool)> {
    if let Some(p) = state.settings.lock().unwrap().xml.clone().map(PathBuf::from) {
        if p.is_file() {
            return Some((p, true));
        }
    }
    auto_xml_candidates(app)
        .into_iter()
        .filter(|p| p.is_file())
        .max_by_key(|p| mtime_ms(p))
        .map(|p| (p, false))
}

fn sig_of(p: &Path) -> String {
    let len = fs::metadata(p).map(|m| m.len()).unwrap_or(0);
    format!("{}|{}", mtime_ms(p), len)
}

#[derive(Serialize)]
struct XmlInfo {
    path: String,
    name: String,
    manual: bool,
}

#[derive(Serialize)]
struct XmlPayload {
    path: String,
    name: String,
    manual: bool,
    sig: String,
    text: String,
}

fn xml_info_of(p: &Path, manual: bool) -> XmlInfo {
    XmlInfo {
        path: p.to_string_lossy().to_string(),
        name: p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        manual,
    }
}

#[tauri::command]
fn xml_info(app: AppHandle, state: State<'_, AppState>) -> Option<XmlInfo> {
    current_xml(&app, &state).map(|(p, m)| xml_info_of(&p, m))
}

#[tauri::command]
async fn xml_pick(app: AppHandle, state: State<'_, AppState>) -> Result<Option<XmlInfo>, String> {
    let mut dlg = app
        .dialog()
        .file()
        .set_title("XML della tua libreria (rekordbox o Musica/iTunes)")
        .add_filter("XML", &["xml"]);
    if let Some((p, _)) = current_xml(&app, &state) {
        if let Some(d) = p.parent() {
            dlg = dlg.set_directory(d);
        }
    }
    let picked = match dlg.blocking_pick_file() {
        Some(fp) => fp.into_path().map_err(|e| e.to_string())?,
        None => return Ok(None),
    };
    {
        let mut s = state.settings.lock().unwrap();
        s.xml = Some(picked.to_string_lossy().to_string());
        s.xml_sent = None;
    }
    state.store();
    Ok(Some(xml_info_of(&picked, true)))
}

/// Torna l'XML solo se e' cambiato dall'ultimo invio per questo utente (o se force).
#[tauri::command]
async fn xml_read(
    app: AppHandle,
    state: State<'_, AppState>,
    user: String,
    force: Option<bool>,
) -> Result<Option<XmlPayload>, String> {
    let (p, manual) = match current_xml(&app, &state) {
        Some(x) => x,
        None => return Err("nessun XML".into()),
    };
    let sig = sig_of(&p);
    let key = format!("{}|{}|{}", user.to_lowercase(), p.to_string_lossy(), sig);
    if !force.unwrap_or(false) && state.settings.lock().unwrap().xml_sent.as_deref() == Some(key.as_str()) {
        return Ok(None);
    }
    let bytes = fs::read(&p).map_err(|e| format!("XML non leggibile: {e}"))?;
    let info = xml_info_of(&p, manual);
    Ok(Some(XmlPayload {
        path: info.path,
        name: info.name,
        manual,
        sig,
        text: String::from_utf8_lossy(&bytes).into_owned(),
    }))
}

#[tauri::command]
fn xml_sent(state: State<'_, AppState>, user: String, path: String, sig: String) {
    state.settings.lock().unwrap().xml_sent = Some(format!("{}|{}|{}", user.to_lowercase(), path, sig));
    state.store();
}

/// 21/11/2026 - "Versioni in rekordbox": legge un brano della collezione dal disco di questo PC
/// (D:\Consolle ha lo stesso percorso su SERVER, PcStudio e V3) e lo passa alla dashboard, che lo suona.
/// Solo file audio; se il file qui non c'e' la dashboard lo prende dal server.
#[tauri::command]
async fn local_audio(path: String) -> Result<tauri::ipc::Response, String> {
    let p = PathBuf::from(path.trim());
    let ext = p
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    const AUDIO: [&str; 9] = ["mp3", "m4a", "mp4", "aac", "wav", "flac", "aif", "aiff", "ogg"];
    if !AUDIO.contains(&ext.as_str()) {
        return Err("non e' un file audio".into());
    }
    if !p.is_file() {
        return Err("file non trovato su questo PC".into());
    }
    let bytes = tauri::async_runtime::spawn_blocking(move || fs::read(&p))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("file non leggibile: {e}"))?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_info,
            get_folder,
            pick_folder,
            file_exists,
            save_file,
            open_folder,
            xml_info,
            xml_pick,
            xml_read,
            xml_sent,
            local_audio
        ])
        .setup(|app| {
            let file = app
                .path()
                .app_config_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("settings.json");
            let settings: Settings = fs::read_to_string(&file)
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default();
            let url = settings
                .url
                .clone()
                .filter(|u| !u.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_URL.to_string());
            let state = AppState { settings: Mutex::new(settings), file };
            state.store(); // crea settings.json la prima volta (cosi' si puo' cambiare l'indirizzo a mano)
            app.manage(state);

            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title("JSA Channels")
                .inner_size(1440.0, 900.0)
                .min_inner_size(900.0, 600.0)
                .maximized(true)
                // ogni altro download (link, blob, APK...) finisce anche lui nella cartella scelta
                .on_download(|webview, event| {
                    if let DownloadEvent::Requested { url, destination } = event {
                        let folder = webview.app_handle().state::<AppState>().folder();
                        if let Some(dir) = folder {
                            let fname = destination
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .filter(|n| !n.is_empty())
                                .unwrap_or_else(|| {
                                    url.path_segments()
                                        .and_then(|mut s| s.next_back().map(|x| x.to_string()))
                                        .unwrap_or_default()
                                });
                            *destination = dir.join(clean_name(&fname));
                        }
                    }
                    true
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("errore all'avvio di JSA Channels");
}
