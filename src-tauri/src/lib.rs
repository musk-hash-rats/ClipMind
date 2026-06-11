use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use arboard::Clipboard;
use base64::{engine::general_purpose, Engine as _};
use chrono::{SecondsFormat, Utc};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum CaptureState {
    Active,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClipPrivacy {
    sensitive: bool,
    masked: bool,
    #[serde(rename = "localOnly")]
    local_only: bool,
    #[serde(rename = "burnAfterUse")]
    burn_after_use: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkSession {
    id: String,
    title: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "captureState")]
    capture_state: CaptureState,
    #[serde(rename = "defaultPrivacy")]
    default_privacy: ClipPrivacyDefaults,
    #[serde(rename = "clipCount")]
    clip_count: usize,
    #[serde(rename = "lastClipAt")]
    last_clip_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClipPrivacyDefaults {
    masked: bool,
    #[serde(rename = "localOnly")]
    local_only: bool,
    #[serde(rename = "burnAfterUse")]
    burn_after_use: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClipSource {
    #[serde(rename = "appName")]
    app_name: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "capturedVia")]
    captured_via: String,
    confidence: String,
    #[serde(rename = "fallbackReason")]
    fallback_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClipContentRef {
    #[serde(rename = "encryptedPayloadId")]
    encrypted_payload_id: String,
    #[serde(rename = "safePreview")]
    safe_preview: String,
    #[serde(rename = "byteSize")]
    byte_size: Option<usize>,
    #[serde(rename = "mimeType")]
    mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClipRecord {
    id: String,
    #[serde(rename = "type")]
    clip_type: String,
    title: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    #[serde(rename = "sessionId")]
    session_id: String,
    source: ClipSource,
    privacy: ClipPrivacy,
    content: ClipContentRef,
    transforms: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AuditEvent {
    id: String,
    #[serde(rename = "type")]
    event_type: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    actor: String,
    #[serde(rename = "targetId")]
    target_id: Option<String>,
    summary: String,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedPayload {
    nonce: String,
    ciphertext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AppStore {
    sessions: Vec<WorkSession>,
    clips: Vec<ClipRecord>,
    #[serde(rename = "auditEvents")]
    audit_events: Vec<AuditEvent>,
    payloads: HashMap<String, EncryptedPayload>,
}

#[derive(Debug, Clone, Serialize)]
struct ClientState {
    sessions: Vec<WorkSession>,
    clips: Vec<ClipRecord>,
    #[serde(rename = "auditEvents")]
    audit_events: Vec<AuditEvent>,
}

impl Default for AppStore {
    fn default() -> Self {
        let now = now_iso();

        Self {
            sessions: vec![WorkSession {
                id: "session-inbox".to_string(),
                title: "Inbox".to_string(),
                created_at: now.clone(),
                updated_at: now,
                capture_state: CaptureState::Paused,
                default_privacy: ClipPrivacyDefaults {
                    masked: true,
                    local_only: true,
                    burn_after_use: false,
                },
                clip_count: 0,
                last_clip_at: None,
            }],
            clips: Vec::new(),
            audit_events: Vec::new(),
            payloads: HashMap::new(),
        }
    }
}

impl AppStore {
    fn client_state(&self) -> ClientState {
        ClientState {
            sessions: self.sessions.clone(),
            clips: self.clips.clone(),
            audit_events: self.audit_events.clone(),
        }
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn unique_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let random = rand::random::<u32>();

    format!("{prefix}-{millis}-{random:x}")
}

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("store.json"))
}

fn export_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = data_dir(app)?.join("exports");
    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn key_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("clipmind.key"))
}

fn read_or_create_key(app: &AppHandle) -> Result<[u8; 32], String> {
    let path = key_path(app)?;

    if path.exists() {
        let key = fs::read(&path).map_err(|error| error.to_string())?;
        return key
            .try_into()
            .map_err(|_| "Stored encryption key is invalid".to_string());
    }

    let mut key = [0u8; 32];
    OsRng.fill_bytes(&mut key);
    write_private_file(&path, &key)?;
    Ok(key)
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt};

        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .mode(0o600)
            .open(path)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        return Ok(());
    }

    #[cfg(not(unix))]
    {
        fs::write(path, bytes).map_err(|error| error.to_string())
    }
}

fn load_store(app: &AppHandle) -> Result<AppStore, String> {
    let path = store_path(app)?;

    if !path.exists() {
        let store = AppStore::default();
        save_store(app, &store)?;
        return Ok(store);
    }

    let bytes = fs::read(&path).map_err(|error| error.to_string())?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

fn save_store(app: &AppHandle, store: &AppStore) -> Result<(), String> {
    let path = store_path(app)?;
    let bytes = serde_json::to_vec_pretty(store).map_err(|error| error.to_string())?;
    write_private_file(&path, &bytes)
}

fn encrypt_payload(app: &AppHandle, plaintext: &str) -> Result<EncryptedPayload, String> {
    let key = read_or_create_key(app)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|error| error.to_string())?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| "Payload encryption failed".to_string())?;

    Ok(EncryptedPayload {
        nonce: general_purpose::STANDARD.encode(nonce_bytes),
        ciphertext: general_purpose::STANDARD.encode(ciphertext),
    })
}

fn decrypt_payload(app: &AppHandle, payload: &EncryptedPayload) -> Result<String, String> {
    let key = read_or_create_key(app)?;
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|error| error.to_string())?;
    let nonce = general_purpose::STANDARD
        .decode(&payload.nonce)
        .map_err(|error| error.to_string())?;
    let ciphertext = general_purpose::STANDARD
        .decode(&payload.ciphertext)
        .map_err(|error| error.to_string())?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| "Payload decryption failed".to_string())?;

    String::from_utf8(plaintext).map_err(|_| "Payload was not valid UTF-8".to_string())
}

fn audit_event(event_type: &str, summary: String, target_id: Option<String>) -> AuditEvent {
    AuditEvent {
        id: unique_id("audit"),
        event_type: event_type.to_string(),
        created_at: now_iso(),
        actor: "local-user".to_string(),
        target_id,
        summary,
        metadata: None,
    }
}

fn refresh_session_counts(store: &mut AppStore) {
    for session in &mut store.sessions {
        let clips: Vec<&ClipRecord> = store
            .clips
            .iter()
            .filter(|clip| clip.session_id == session.id)
            .collect();
        session.clip_count = clips.len();
        session.last_clip_at = clips.iter().map(|clip| clip.created_at.clone()).max();
        session.updated_at = now_iso();
    }
}

fn safe_preview(text: &str) -> String {
    const MAX_CHARS: usize = 120;
    let mut preview = text.chars().take(MAX_CHARS).collect::<String>();

    if text.chars().count() > MAX_CHARS {
        preview.push_str("...");
    }

    preview
}

#[tauri::command]
fn app_status(app: AppHandle) -> Result<serde_json::Value, String> {
    let dir = data_dir(&app)?;

    Ok(serde_json::json!({
        "capture": "manual",
        "encryption": "local_aes_256_gcm",
        "storage": "initialized",
        "dataDir": dir
    }))
}

#[tauri::command]
fn load_state(app: AppHandle) -> Result<ClientState, String> {
    let store = load_store(&app)?;
    Ok(store.client_state())
}

#[tauri::command]
fn create_session(app: AppHandle, title: String) -> Result<ClientState, String> {
    let cleaned_title = title.trim();
    if cleaned_title.is_empty() {
        return Err("Session title is required".to_string());
    }

    let mut store = load_store(&app)?;
    let now = now_iso();
    let session_id = unique_id("session");
    store.sessions.insert(
        0,
        WorkSession {
            id: session_id.clone(),
            title: cleaned_title.chars().take(80).collect(),
            created_at: now.clone(),
            updated_at: now,
            capture_state: CaptureState::Paused,
            default_privacy: ClipPrivacyDefaults {
                masked: true,
                local_only: true,
                burn_after_use: false,
            },
            clip_count: 0,
            last_clip_at: None,
        },
    );
    store.audit_events.insert(
        0,
        audit_event(
            "session-created",
            format!("Created session: {cleaned_title}"),
            Some(session_id),
        ),
    );
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn set_capture_state(
    app: AppHandle,
    session_id: String,
    capture_state: CaptureState,
) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let session = store
        .sessions
        .iter_mut()
        .find(|session| session.id == session_id)
        .ok_or_else(|| "Session not found".to_string())?;

    session.capture_state = capture_state.clone();
    session.updated_at = now_iso();
    let (event_type, summary) = match capture_state {
        CaptureState::Active => ("capture-started", "Clipboard capture started"),
        CaptureState::Paused | CaptureState::Stopped => {
            ("capture-paused", "Clipboard capture paused")
        }
    };
    store.audit_events.insert(
        0,
        audit_event(event_type, summary.to_string(), Some(session_id)),
    );
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn capture_clipboard_text(app: AppHandle, session_id: String) -> Result<ClientState, String> {
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;
    let text = clipboard.get_text().map_err(|error| error.to_string())?;
    let mut store = load_store(&app)?;
    let now = now_iso();
    let session_id = if store
        .sessions
        .iter()
        .any(|session| session.id == session_id)
    {
        session_id
    } else {
        store
            .sessions
            .first()
            .map(|session| session.id.clone())
            .ok_or_else(|| "No session exists".to_string())?
    };
    if let Some(latest) = store.clips.first() {
        if latest.session_id == session_id {
            if let Some(payload) = store.payloads.get(&latest.content.encrypted_payload_id) {
                if decrypt_payload(&app, payload).ok().as_deref() == Some(text.as_str()) {
                    return Ok(store.client_state());
                }
            }
        }
    }

    let payload_id = unique_id("payload");
    let clip_id = unique_id("clip");
    let encrypted_payload = encrypt_payload(&app, &text)?;

    store.payloads.insert(payload_id.clone(), encrypted_payload);
    store.clips.insert(
        0,
        ClipRecord {
            id: clip_id.clone(),
            clip_type: "text".to_string(),
            title: "Clipboard text".to_string(),
            created_at: now.clone(),
            updated_at: now,
            session_id,
            source: ClipSource {
                app_name: "System Clipboard".to_string(),
                device_id: "local-desktop".to_string(),
                captured_via: "clipboard-listener".to_string(),
                confidence: "fallback".to_string(),
                fallback_reason: Some("Manual native capture; source app unavailable".to_string()),
            },
            privacy: ClipPrivacy {
                sensitive: true,
                masked: true,
                local_only: true,
                burn_after_use: false,
            },
            content: ClipContentRef {
                encrypted_payload_id: payload_id,
                safe_preview: safe_preview(&text),
                byte_size: Some(text.len()),
                mime_type: Some("text/plain".to_string()),
            },
            transforms: Vec::new(),
        },
    );
    store.audit_events.insert(
        0,
        audit_event(
            "capture-started",
            "Captured clipboard text into encrypted local store".to_string(),
            Some(clip_id),
        ),
    );
    refresh_session_counts(&mut store);
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn reveal_clip(app: AppHandle, clip_id: String) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let clip = store
        .clips
        .iter_mut()
        .find(|clip| clip.id == clip_id)
        .ok_or_else(|| "Clip not found".to_string())?;
    let payload = store
        .payloads
        .get(&clip.content.encrypted_payload_id)
        .ok_or_else(|| "Encrypted payload not found".to_string())?;
    let plaintext = decrypt_payload(&app, payload)?;

    clip.privacy.masked = false;
    clip.content.safe_preview = safe_preview(&plaintext);
    clip.updated_at = now_iso();
    store.audit_events.insert(
        0,
        audit_event(
            "clip-revealed",
            format!("Revealed selected clip: {}", clip.title),
            Some(clip.id.clone()),
        ),
    );
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn export_clip(
    app: AppHandle,
    clip_id: String,
    user_confirmed: bool,
) -> Result<ClientState, String> {
    if !user_confirmed {
        return Err("Export requires explicit user confirmation".to_string());
    }

    let mut store = load_store(&app)?;
    let clip = store
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .cloned()
        .ok_or_else(|| "Clip not found".to_string())?;

    if clip.privacy.sensitive && clip.privacy.masked {
        return Err("Reveal or unmask this sensitive clip before export".to_string());
    }

    let payload = store
        .payloads
        .get(&clip.content.encrypted_payload_id)
        .ok_or_else(|| "Encrypted payload not found".to_string())?;
    let plaintext = decrypt_payload(&app, payload)?;
    let export_id = unique_id("export");
    let export_path = export_dir(&app)?.join(format!("{export_id}.json"));
    let body = serde_json::json!({
        "id": export_id,
        "createdAt": now_iso(),
        "destination": "agent handoff",
        "scope": "selected-clip",
        "clip": {
            "id": clip.id,
            "title": clip.title,
            "createdAt": clip.created_at,
            "sessionId": clip.session_id,
            "source": {
                "appName": clip.source.app_name,
                "capturedVia": clip.source.captured_via,
                "confidence": clip.source.confidence
            },
            "content": plaintext
        },
        "redactions": ["private source metadata"]
    });
    let bytes = serde_json::to_vec_pretty(&body).map_err(|error| error.to_string())?;
    write_private_file(&export_path, &bytes)?;

    store.audit_events.insert(
        0,
        audit_event(
            "agent-export-created",
            format!("Exported selected clip: {}", clip.title),
            Some(clip.id.clone()),
        ),
    );
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn panic_wipe_clip(
    app: AppHandle,
    clip_id: String,
    confirmation: String,
) -> Result<ClientState, String> {
    if confirmation != "WIPE" {
        return Err("Panic wipe requires WIPE confirmation".to_string());
    }

    let mut store = load_store(&app)?;
    let clip = store
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .cloned()
        .ok_or_else(|| "Clip not found".to_string())?;

    store
        .payloads
        .remove(&clip.content.encrypted_payload_id)
        .ok_or_else(|| "Encrypted payload was already missing".to_string())?;
    store.clips.retain(|item| item.id != clip.id);
    store.audit_events.insert(
        0,
        audit_event(
            "panic-wipe-completed",
            format!("Panic wiped selected clip: {}", clip.title),
            Some(clip.id),
        ),
    );
    refresh_session_counts(&mut store);
    save_store(&app, &store)?;

    Ok(store.client_state())
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_status,
            load_state,
            create_session,
            set_capture_state,
            capture_clipboard_text,
            reveal_clip,
            export_clip,
            panic_wipe_clip
        ])
        .run(tauri::generate_context!())
        .expect("error while running ClipMind");
}
