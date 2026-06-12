use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use arboard::Clipboard;
use base64::{engine::general_purpose, Engine as _};
use chrono::{SecondsFormat, Utc};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, State,
};
use url::Url;

#[derive(Default)]
struct RuntimeSecrets {
    data_key: Option<[u8; 32]>,
}

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
    #[serde(rename = "windowTitle", default, skip_serializing_if = "Option::is_none")]
    window_title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(rename = "filePath", default, skip_serializing_if = "Option::is_none")]
    file_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    sender: Option<String>,
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
    #[serde(rename = "revealedPayload", default, skip_serializing_if = "Option::is_none")]
    revealed_payload: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum TransformKind {
    CleanFormatting,
    Markdown,
    Summarize,
    Translate,
    FixJson,
    StripTracking,
    Ocr,
    ExtractLinks,
    ResizeImage,
    NoteToTask,
    NoteToMessage,
    NoteToEmail,
}

impl TransformKind {
    fn as_str(&self) -> &'static str {
        match self {
            TransformKind::CleanFormatting => "clean-formatting",
            TransformKind::Markdown => "markdown",
            TransformKind::Summarize => "summarize",
            TransformKind::Translate => "translate",
            TransformKind::FixJson => "fix-json",
            TransformKind::StripTracking => "strip-tracking",
            TransformKind::Ocr => "ocr",
            TransformKind::ExtractLinks => "extract-links",
            TransformKind::ResizeImage => "resize-image",
            TransformKind::NoteToTask => "note-to-task",
            TransformKind::NoteToMessage => "note-to-message",
            TransformKind::NoteToEmail => "note-to-email",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            TransformKind::CleanFormatting => "Clean formatting",
            TransformKind::Markdown => "Markdown",
            TransformKind::Summarize => "Summary",
            TransformKind::Translate => "Translation draft",
            TransformKind::FixJson => "Fixed JSON",
            TransformKind::StripTracking => "Tracking links stripped",
            TransformKind::Ocr => "OCR placeholder",
            TransformKind::ExtractLinks => "Extracted links",
            TransformKind::ResizeImage => "Resize placeholder",
            TransformKind::NoteToTask => "Task draft",
            TransformKind::NoteToMessage => "Message draft",
            TransformKind::NoteToEmail => "Email draft",
        }
    }
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
    #[serde(default = "default_locked")]
    locked: bool,
    #[serde(rename = "authSalt")]
    auth_salt: Option<String>,
    #[serde(rename = "authHash")]
    auth_hash: Option<String>,
    #[serde(rename = "wrappedDataKey")]
    wrapped_data_key: Option<EncryptedPayload>,
    #[serde(rename = "semanticIndex")]
    semantic_index: Option<EncryptedPayload>,
    #[serde(rename = "kdfAlgorithm")]
    kdf_algorithm: Option<String>,
    #[serde(rename = "kdfVersion")]
    kdf_version: Option<u32>,
    #[serde(rename = "kdfMemoryKiB")]
    kdf_memory_kib: Option<u32>,
    #[serde(rename = "kdfIterations")]
    kdf_iterations: Option<u32>,
    #[serde(rename = "kdfParallelism")]
    kdf_parallelism: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
struct ClientState {
    sessions: Vec<WorkSession>,
    clips: Vec<ClipRecord>,
    #[serde(rename = "auditEvents")]
    audit_events: Vec<AuditEvent>,
    locked: bool,
    #[serde(rename = "lastExportPath")]
    last_export_path: Option<String>,
    #[serde(rename = "authConfigured")]
    auth_configured: bool,
}

struct CapturedClipboard {
    clip_type: String,
    title: String,
    payload: String,
    safe_preview: String,
    byte_size: Option<usize>,
    mime_type: Option<String>,
    audit_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemanticIndex {
    version: u32,
    model: String,
    #[serde(rename = "builtAt")]
    built_at: String,
    records: Vec<SemanticRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SemanticRecord {
    #[serde(rename = "clipId")]
    clip_id: String,
    #[serde(rename = "payloadId")]
    payload_id: String,
    #[serde(rename = "updatedAt")]
    updated_at: String,
    vector: Vec<f32>,
}

fn default_locked() -> bool {
    true
}

const KDF_ALGORITHM: &str = "argon2id";
const KDF_VERSION: u32 = 1;
const KDF_MEMORY_KIB: u32 = 19_456;
const KDF_ITERATIONS: u32 = 2;
const KDF_PARALLELISM: u32 = 1;

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
            locked: true,
            auth_salt: None,
            auth_hash: None,
            wrapped_data_key: None,
            semantic_index: None,
            kdf_algorithm: None,
            kdf_version: None,
            kdf_memory_kib: None,
            kdf_iterations: None,
            kdf_parallelism: None,
        }
    }
}

impl AppStore {
    fn client_state(&self) -> ClientState {
        self.client_state_with_revealed(None)
    }

    fn client_state_with_revealed(&self, revealed: Option<(&str, &str)>) -> ClientState {
        let mut sessions = self.sessions.clone();
        let mut clips = self.clips.clone();

        if self.locked {
            for session in &mut sessions {
                session.title = "Locked session".to_string();
                session.last_clip_at = None;
            }

            for clip in &mut clips {
                clip.title = "Locked clip".to_string();
                clip.created_at = "1970-01-01T00:00:00.000Z".to_string();
                clip.updated_at = "1970-01-01T00:00:00.000Z".to_string();
                clip.source.app_name = "Locked".to_string();
                clip.source.window_title = None;
                clip.source.url = None;
                clip.source.file_path = None;
                clip.source.sender = None;
                clip.source.device_id = "Locked".to_string();
                clip.source.confidence = "fallback".to_string();
                clip.source.fallback_reason = Some("App is locked".to_string());
                clip.content.safe_preview = "Locked".to_string();
                for transform in &mut clip.transforms {
                    if let Some(object) = transform.as_object_mut() {
                        object.insert(
                            "safePreview".to_string(),
                            serde_json::Value::String("Locked".to_string()),
                        );
                    }
                }
            }
        } else if let Some((clip_id, plaintext)) = revealed {
            if let Some(clip) = clips.iter_mut().find(|clip| clip.id == clip_id) {
                clip.content.safe_preview = safe_preview(plaintext);
                clip.content.revealed_payload = Some(plaintext.to_string());
            }
        }

        ClientState {
            sessions,
            clips,
            audit_events: self.audit_events.clone(),
            locked: self.locked,
            last_export_path: None,
            auth_configured: self.auth_hash.is_some(),
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

fn encrypt_bytes_with_key(key: &[u8; 32], plaintext: &[u8]) -> Result<EncryptedPayload, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|error| error.to_string())?;
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| "Payload encryption failed".to_string())?;

    Ok(EncryptedPayload {
        nonce: general_purpose::STANDARD.encode(nonce_bytes),
        ciphertext: general_purpose::STANDARD.encode(ciphertext),
    })
}

fn decrypt_bytes_with_key(key: &[u8; 32], payload: &EncryptedPayload) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|error| error.to_string())?;
    let nonce = general_purpose::STANDARD
        .decode(&payload.nonce)
        .map_err(|error| error.to_string())?;
    let ciphertext = general_purpose::STANDARD
        .decode(&payload.ciphertext)
        .map_err(|error| error.to_string())?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
        .map_err(|_| "Payload decryption failed".to_string())?;

    Ok(plaintext)
}

fn encrypt_payload_with_key(key: &[u8; 32], plaintext: &str) -> Result<EncryptedPayload, String> {
    encrypt_bytes_with_key(key, plaintext.as_bytes())
}

fn decrypt_payload_with_key(key: &[u8; 32], payload: &EncryptedPayload) -> Result<String, String> {
    let plaintext = decrypt_bytes_with_key(key, payload)?;
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

fn require_unlocked(store: &AppStore, runtime: &State<'_, Mutex<RuntimeSecrets>>) -> Result<[u8; 32], String> {
    if store.locked {
        return Err("ClipMind is locked".to_string());
    } else {
        runtime
            .lock()
            .map_err(|_| "Runtime lock failed".to_string())?
            .data_key
            .ok_or_else(|| "ClipMind unlock key is not available".to_string())
    }
}

fn set_current_kdf_metadata(store: &mut AppStore) {
    store.kdf_algorithm = Some(KDF_ALGORITHM.to_string());
    store.kdf_version = Some(KDF_VERSION);
    store.kdf_memory_kib = Some(KDF_MEMORY_KIB);
    store.kdf_iterations = Some(KDF_ITERATIONS);
    store.kdf_parallelism = Some(KDF_PARALLELISM);
}

fn kdf_params_from_store(store: &AppStore) -> Result<Params, String> {
    if let Some(algorithm) = &store.kdf_algorithm {
        if algorithm != KDF_ALGORITHM {
            return Err(format!("Unsupported key derivation algorithm: {algorithm}"));
        }
    }

    if let Some(version) = store.kdf_version {
        if version != KDF_VERSION {
            return Err(format!("Unsupported key derivation version: {version}"));
        }
    }

    Params::new(
        store.kdf_memory_kib.unwrap_or(KDF_MEMORY_KIB),
        store.kdf_iterations.unwrap_or(KDF_ITERATIONS),
        store.kdf_parallelism.unwrap_or(KDF_PARALLELISM),
        Some(32),
    )
    .map_err(|error| error.to_string())
}

fn derive_wrapping_key(salt: &str, passphrase: &str, params: Params) -> Result<[u8; 32], String> {
    let salt_bytes = general_purpose::STANDARD
        .decode(salt)
        .map_err(|error| error.to_string())?;
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), &salt_bytes, &mut key)
        .map_err(|error| error.to_string())?;
    Ok(key)
}

fn create_auth_salt() -> String {
    let mut bytes = [0u8; 16];
    OsRng.fill_bytes(&mut bytes);
    general_purpose::STANDARD.encode(bytes)
}

fn unlock_data_key(
    app: &AppHandle,
    store: &mut AppStore,
    passphrase: &str,
) -> Result<[u8; 32], String> {
    let passphrase = passphrase.trim();
    if passphrase.len() < 12 {
        return Err("Unlock passphrase must be at least 12 characters".to_string());
    }

    if store.auth_salt.is_none() {
        store.auth_salt = Some(create_auth_salt());
    }

    let salt = store
        .auth_salt
        .clone()
        .ok_or_else(|| "Auth salt unavailable".to_string())?;
    if store.kdf_algorithm.is_none() {
        set_current_kdf_metadata(store);
    }
    let wrapping_key = derive_wrapping_key(&salt, passphrase, kdf_params_from_store(store)?)?;
    let auth_hash = general_purpose::STANDARD.encode(Sha256::digest(wrapping_key));

    if let Some(expected_hash) = &store.auth_hash {
        if *expected_hash != auth_hash {
            return Err("Unlock passphrase did not match".to_string());
        }
    } else {
        store.auth_hash = Some(auth_hash);
    }

    if let Some(wrapped_key) = &store.wrapped_data_key {
        let bytes = decrypt_bytes_with_key(&wrapping_key, wrapped_key)?;
        return bytes
            .try_into()
            .map_err(|_| "Wrapped data key was invalid".to_string());
    }

    let data_key = if key_path(app)?.exists() {
        read_or_create_key(app)?
    } else {
        let mut key = [0u8; 32];
        OsRng.fill_bytes(&mut key);
        key
    };
    set_current_kdf_metadata(store);
    store.wrapped_data_key = Some(encrypt_bytes_with_key(&wrapping_key, &data_key)?);
    let legacy_key_path = key_path(app)?;
    if legacy_key_path.exists() {
        fs::remove_file(legacy_key_path).map_err(|error| error.to_string())?;
    }
    Ok(data_key)
}

fn burn_clip(store: &mut AppStore, clip_id: &str, summary: String) -> Result<(), String> {
    let clip_index = store
        .clips
        .iter()
        .position(|clip| clip.id == clip_id)
        .ok_or_else(|| "Clip not found for burn-after-use".to_string())?;
    let clip = store.clips.remove(clip_index);
    store.payloads.remove(&clip.content.encrypted_payload_id);
    store.audit_events.insert(
        0,
        audit_event("burn-after-use-consumed", summary, Some(clip.id)),
    );
    refresh_session_counts(store);
    Ok(())
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

fn set_capture_state_in_store(
    store: &mut AppStore,
    session_id: &str,
    capture_state: CaptureState,
) -> Result<(&'static str, &'static str), String> {
    let session_exists = store.sessions.iter().any(|session| session.id == session_id);
    if !session_exists {
        return Err("Session not found".to_string());
    }

    for session in &mut store.sessions {
        if session.id == session_id {
            session.capture_state = capture_state.clone();
        } else if matches!(capture_state, CaptureState::Active)
            && matches!(session.capture_state, CaptureState::Active)
        {
            session.capture_state = CaptureState::Paused;
        }
        session.updated_at = now_iso();
    }

    Ok(match capture_state {
        CaptureState::Active => ("capture-started", "Clipboard capture started"),
        CaptureState::Paused | CaptureState::Stopped => {
            ("capture-paused", "Clipboard capture paused")
        }
    })
}

fn pause_all_capture_sessions(app: &AppHandle) -> Result<ClientState, String> {
    let mut store = load_store(app)?;
    for session in &mut store.sessions {
        if matches!(session.capture_state, CaptureState::Active) {
            session.capture_state = CaptureState::Paused;
            session.updated_at = now_iso();
        }
    }
    store.audit_events.insert(
        0,
        audit_event("capture-paused", "Clipboard capture paused from tray".to_string(), None),
    );
    save_store(app, &store)?;
    Ok(store.client_state())
}

fn resume_primary_capture_session(app: &AppHandle) -> Result<ClientState, String> {
    let mut store = load_store(app)?;
    if store.locked {
        return Err("ClipMind is locked".to_string());
    }
    let session_id = store
        .sessions
        .first()
        .map(|session| session.id.clone())
        .ok_or_else(|| "No session exists".to_string())?;
    let (event_type, summary) =
        set_capture_state_in_store(&mut store, &session_id, CaptureState::Active)?;
    store
        .audit_events
        .insert(0, audit_event(event_type, format!("{summary} from tray"), Some(session_id)));
    save_store(app, &store)?;
    Ok(store.client_state())
}

fn safe_preview(text: &str) -> String {
    const MAX_CHARS: usize = 120;
    let mut preview = text.chars().take(MAX_CHARS).collect::<String>();

    if text.chars().count() > MAX_CHARS {
        preview.push_str("...");
    }

    preview
}

fn image_payload(width: usize, height: usize, bytes: &[u8]) -> String {
    serde_json::json!({
        "kind": "image-rgba",
        "width": width,
        "height": height,
        "bytes": general_purpose::STANDARD.encode(bytes)
    })
    .to_string()
}

fn imported_file_payload(file_name: &str, mime_type: &str, bytes_base64: &str) -> String {
    serde_json::json!({
        "kind": "file-bytes",
        "fileName": file_name,
        "mimeType": mime_type,
        "bytes": bytes_base64
    })
    .to_string()
}

fn safe_file_name(file_name: &str) -> String {
    Path::new(file_name)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("imported-file")
        .chars()
        .filter(|char| !char.is_control())
        .take(120)
        .collect::<String>()
}

fn read_supported_clipboard() -> Result<CapturedClipboard, String> {
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;

    if let Ok(text) = clipboard.get_text() {
        if !text.trim().is_empty() {
            return Ok(CapturedClipboard {
                clip_type: "text".to_string(),
                title: "Clipboard text".to_string(),
                safe_preview: safe_preview(&text),
                byte_size: Some(text.len()),
                mime_type: Some("text/plain".to_string()),
                audit_summary: "Captured clipboard text into encrypted local store".to_string(),
                payload: text,
            });
        }
    }

    if let Ok(image) = clipboard.get_image() {
        return Ok(CapturedClipboard {
            clip_type: "image".to_string(),
            title: "Clipboard image".to_string(),
            payload: image_payload(image.width, image.height, image.bytes.as_ref()),
            safe_preview: format!("Image {}x{}", image.width, image.height),
            byte_size: Some(image.bytes.len()),
            mime_type: Some("application/clipmind-rgba".to_string()),
            audit_summary: "Captured clipboard image into encrypted local store".to_string(),
        });
    }

    Err("Clipboard does not contain supported text or image data".to_string())
}

fn capture_clipboard_payload(
    app: &AppHandle,
    data_key: &[u8; 32],
    session_id: String,
    masked: bool,
) -> Result<(ClientState, bool), String> {
    let captured = read_supported_clipboard()?;
    let mut store = load_store(app)?;
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
                if decrypt_payload_with_key(data_key, payload).ok().as_deref()
                    == Some(captured.payload.as_str())
                {
                    return Ok((store.client_state(), false));
                }
            }
        }
    }

    let payload_id = unique_id("payload");
    let clip_id = unique_id("clip");
    let encrypted_payload = encrypt_payload_with_key(data_key, &captured.payload)?;

    store.payloads.insert(payload_id.clone(), encrypted_payload);
    store.clips.insert(
        0,
        ClipRecord {
            id: clip_id.clone(),
            clip_type: captured.clip_type,
            title: captured.title,
            created_at: now.clone(),
            updated_at: now,
            session_id,
            source: ClipSource {
                app_name: "System Clipboard".to_string(),
                window_title: None,
                url: None,
                file_path: None,
                sender: None,
                device_id: "local-desktop".to_string(),
                captured_via: "clipboard-listener".to_string(),
                confidence: "fallback".to_string(),
                fallback_reason: Some("Native lifecycle capture; source app unavailable".to_string()),
            },
            privacy: ClipPrivacy {
                sensitive: true,
                masked,
                local_only: true,
                burn_after_use: false,
            },
            content: ClipContentRef {
                encrypted_payload_id: payload_id,
                safe_preview: captured.safe_preview,
                revealed_payload: None,
                byte_size: captured.byte_size,
                mime_type: captured.mime_type,
            },
            transforms: Vec::new(),
        },
    );
    store.audit_events.insert(
        0,
        audit_event("capture-started", captured.audit_summary, Some(clip_id)),
    );
    refresh_session_counts(&mut store);
    save_store(app, &store)?;

    Ok((store.client_state(), true))
}

fn set_clipboard_from_payload(clip: &ClipRecord, plaintext: &str) -> Result<(), String> {
    set_clipboard_from_typed_payload(&clip.clip_type, plaintext)
}

fn set_clipboard_from_typed_payload(clip_type: &str, plaintext: &str) -> Result<(), String> {
    let mut clipboard = Clipboard::new().map_err(|error| error.to_string())?;

    if clip_type == "image" || clip_type == "screenshot" {
        let value: serde_json::Value =
            serde_json::from_str(plaintext).map_err(|error| format!("Image payload parse failed: {error}"))?;
        let width = value
            .get("width")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "Image payload missing width".to_string())? as usize;
        let height = value
            .get("height")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "Image payload missing height".to_string())? as usize;
        let bytes = value
            .get("bytes")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "Image payload missing bytes".to_string())
            .and_then(|encoded| general_purpose::STANDARD.decode(encoded).map_err(|error| error.to_string()))?;

        clipboard
            .set_image(arboard::ImageData {
                width,
                height,
                bytes: bytes.into(),
            })
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    clipboard
        .set_text(plaintext.to_string())
        .map_err(|error| error.to_string())
}

fn clean_formatting(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<&str>>()
        .join("\n")
}

fn markdown_from_text(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| {
            if line.starts_with("- ") || line.starts_with("#") {
                line.to_string()
            } else {
                format!("- {line}")
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn summarize_text(text: &str) -> String {
    let cleaned = clean_formatting(text).replace('\n', " ");
    let mut summary = cleaned.chars().take(280).collect::<String>();

    if cleaned.chars().count() > 280 {
        summary.push_str("...");
    }

    summary
}

fn strip_tracking_from_token(token: &str) -> String {
    let trimmed = token.trim_matches(|c: char| matches!(c, ',' | ')' | ']' | '>' | '"' | '\''));
    let Ok(mut url) = Url::parse(trimmed) else {
        return token.to_string();
    };

    let filtered = url
        .query_pairs()
        .filter(|(key, _)| {
            let key = key.to_ascii_lowercase();
            !(key.starts_with("utm_")
                || matches!(
                    key.as_str(),
                    "fbclid" | "gclid" | "dclid" | "mc_cid" | "mc_eid" | "igshid" | "ref"
                ))
        })
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect::<Vec<(String, String)>>();

    url.set_query(None);
    if !filtered.is_empty() {
        let mut pairs = url.query_pairs_mut();
        for (key, value) in filtered {
            pairs.append_pair(&key, &value);
        }
    }

    token.replace(trimmed, url.as_str())
}

fn strip_tracking_links(text: &str) -> String {
    text.split_whitespace()
        .map(strip_tracking_from_token)
        .collect::<Vec<String>>()
        .join(" ")
}

fn extract_links(text: &str) -> String {
    let links = text
        .split_whitespace()
        .map(|token| token.trim_matches(|c: char| matches!(c, ',' | ')' | ']' | '>' | '"' | '\'')))
        .filter(|token| Url::parse(token).is_ok())
        .map(str::to_string)
        .collect::<Vec<String>>();

    if links.is_empty() {
        "No links found.".to_string()
    } else {
        links.join("\n")
    }
}

fn note_to_task(text: &str) -> String {
    clean_formatting(text)
        .lines()
        .map(|line| {
            if line.starts_with("- [ ] ") {
                line.to_string()
            } else {
                format!("- [ ] {line}")
            }
        })
        .collect::<Vec<String>>()
        .join("\n")
}

fn transform_text_by_name(kind: &str, plaintext: &str) -> Result<String, String> {
    match kind {
        "clean-formatting" => Ok(clean_formatting(plaintext)),
        "markdown" => Ok(markdown_from_text(plaintext)),
        "summarize" => Ok(summarize_text(plaintext)),
        "fix-json" => {
            let json: serde_json::Value =
                serde_json::from_str(plaintext).map_err(|error| format!("JSON parse failed: {error}"))?;
            serde_json::to_string_pretty(&json).map_err(|error| error.to_string())
        }
        "strip-tracking" => Ok(strip_tracking_links(plaintext)),
        "extract-links" => Ok(extract_links(plaintext)),
        "note-to-task" => Ok(note_to_task(plaintext)),
        "note-to-message" => Ok(clean_formatting(plaintext)),
        "note-to-email" => Ok(format!("Hi,\n\n{}\n\nBest,", clean_formatting(plaintext))),
        _ => Err("This transform is not available for text clips yet".to_string()),
    }
}

fn transform_text(kind: &TransformKind, text: &str) -> Result<String, String> {
    transform_text_by_name(kind.as_str(), text)
}

const SEMANTIC_INDEX_VERSION: u32 = 1;
const SEMANTIC_VECTOR_SIZE: usize = 64;

fn semantic_vector(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0f32; SEMANTIC_VECTOR_SIZE];
    for token in text
        .split(|char: char| !char.is_alphanumeric())
        .filter(|token| token.len() > 2)
    {
        let hash = Sha256::digest(token.to_ascii_lowercase().as_bytes());
        let index = hash[0] as usize % SEMANTIC_VECTOR_SIZE;
        vector[index] += 1.0;
    }

    let magnitude = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if magnitude > 0.0 {
        for value in &mut vector {
            *value /= magnitude;
        }
    }
    vector
}

fn cosine_similarity(left: &[f32], right: &[f32]) -> f32 {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| left * right)
        .sum::<f32>()
}

fn semantic_document_text(clip: &ClipRecord, plaintext: &str) -> String {
    format!(
        "{} {} {} {} {} {} {}",
        clip.title,
        clip.clip_type,
        clip.source.app_name,
        clip.source.file_path.clone().unwrap_or_default(),
        clip.source.fallback_reason.clone().unwrap_or_default(),
        clip.content.safe_preview,
        plaintext
    )
}

fn build_semantic_index(store: &AppStore, data_key: &[u8; 32]) -> Result<SemanticIndex, String> {
    let mut records = Vec::new();
    for clip in &store.clips {
        if clip.privacy.burn_after_use {
            continue;
        }
        let Some(payload) = store.payloads.get(&clip.content.encrypted_payload_id) else {
            continue;
        };
        let plaintext = decrypt_payload_with_key(data_key, payload)?;
        records.push(SemanticRecord {
            clip_id: clip.id.clone(),
            payload_id: clip.content.encrypted_payload_id.clone(),
            updated_at: clip.updated_at.clone(),
            vector: semantic_vector(&semantic_document_text(clip, &plaintext)),
        });
    }

    Ok(SemanticIndex {
        version: SEMANTIC_INDEX_VERSION,
        model: "clipmind-local-hash-v1".to_string(),
        built_at: now_iso(),
        records,
    })
}

fn decrypt_semantic_index(store: &AppStore, data_key: &[u8; 32]) -> Result<SemanticIndex, String> {
    let payload = store
        .semantic_index
        .as_ref()
        .ok_or_else(|| "Semantic index has not been built".to_string())?;
    let plaintext = decrypt_payload_with_key(data_key, payload)?;
    serde_json::from_str(&plaintext).map_err(|error| error.to_string())
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
fn load_state(app: AppHandle, runtime: State<'_, Mutex<RuntimeSecrets>>) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    runtime
        .lock()
        .map_err(|_| "Runtime lock failed".to_string())?
        .data_key = None;
    if !store.locked {
        store.locked = true;
        store.audit_events.insert(
            0,
            audit_event(
                "app-locked",
                "ClipMind app locked on startup".to_string(),
                None,
            ),
        );
        save_store(&app, &store)?;
    }
    Ok(store.client_state())
}

#[tauri::command]
fn refresh_state(app: AppHandle) -> Result<ClientState, String> {
    let store = load_store(&app)?;
    Ok(store.client_state())
}

fn lock_store(app: &AppHandle, runtime: &Mutex<RuntimeSecrets>) -> Result<ClientState, String> {
    let mut store = load_store(app)?;
    store.locked = true;
    runtime
        .lock()
        .map_err(|_| "Runtime lock failed".to_string())?
        .data_key = None;
    store.audit_events.insert(
        0,
        audit_event(
            "app-locked",
            "ClipMind app locked".to_string(),
            None,
        ),
    );
    save_store(app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn lock_app(app: AppHandle, runtime: State<'_, Mutex<RuntimeSecrets>>) -> Result<ClientState, String> {
    lock_store(&app, &runtime)
}

#[tauri::command]
fn unlock_app(app: AppHandle, passphrase: String) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let was_configured = store.auth_hash.is_some();
    let data_key = unlock_data_key(&app, &mut store, &passphrase)?;
    store.locked = false;
    app.state::<Mutex<RuntimeSecrets>>()
        .lock()
        .map_err(|_| "Runtime lock failed".to_string())?
        .data_key = Some(data_key);
    store.audit_events.insert(
        0,
        audit_event(
            "app-unlocked",
            if was_configured {
                "ClipMind app unlocked".to_string()
            } else {
                "ClipMind unlock passphrase set".to_string()
            },
            None,
        ),
    );
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn reset_store(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    confirmation: String,
) -> Result<ClientState, String> {
    if confirmation != "RESET" {
        return Err("Reset requires RESET confirmation".to_string());
    }

    runtime
        .lock()
        .map_err(|_| "Runtime lock failed".to_string())?
        .data_key = None;

    let legacy_key_path = key_path(&app)?;
    if legacy_key_path.exists() {
        fs::remove_file(legacy_key_path).map_err(|error| error.to_string())?;
    }

    let exports = export_dir(&app)?;
    if exports.exists() {
        fs::remove_dir_all(&exports).map_err(|error| error.to_string())?;
    }

    let mut store = AppStore::default();
    store.audit_events.insert(
        0,
        audit_event(
            "local-store-reset",
            "Reset local ClipMind store after passphrase recovery request".to_string(),
            None,
        ),
    );
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn create_session(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    title: String,
    mask_by_default: Option<bool>,
) -> Result<ClientState, String> {
    let cleaned_title = title.trim();
    if cleaned_title.is_empty() {
        return Err("Session title is required".to_string());
    }

    let mut store = load_store(&app)?;
    let _data_key = require_unlocked(&store, &runtime)?;
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
                masked: mask_by_default.unwrap_or(true),
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
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    session_id: String,
    capture_state: CaptureState,
) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let _data_key = require_unlocked(&store, &runtime)?;
    let (event_type, summary) = set_capture_state_in_store(&mut store, &session_id, capture_state)?;
    store.audit_events.insert(
        0,
        audit_event(event_type, summary.to_string(), Some(session_id)),
    );
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn capture_clipboard_text(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    session_id: String,
    masked: Option<bool>,
) -> Result<ClientState, String> {
    let store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    capture_clipboard_payload(&app, &data_key, session_id, masked.unwrap_or(true))
        .map(|(state, _changed)| state)
}

#[tauri::command]
fn import_file_clip(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    session_id: String,
    file_name: String,
    mime_type: Option<String>,
    bytes_base64: String,
    masked: Option<bool>,
    screenshot: Option<bool>,
) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    let decoded = general_purpose::STANDARD
        .decode(bytes_base64.as_bytes())
        .map_err(|error| format!("File import payload was invalid base64: {error}"))?;
    if decoded.is_empty() {
        return Err("Imported file was empty".to_string());
    }

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
    let file_name = safe_file_name(&file_name);
    let mime_type = mime_type
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let clip_type = if screenshot.unwrap_or(false) {
        "screenshot"
    } else {
        "file"
    };
    let payload = imported_file_payload(&file_name, &mime_type, &bytes_base64);
    let payload_id = unique_id("payload");
    let clip_id = unique_id("clip");
    let encrypted_payload = encrypt_payload_with_key(&data_key, &payload)?;

    store.payloads.insert(payload_id.clone(), encrypted_payload);
    store.clips.insert(
        0,
        ClipRecord {
            id: clip_id.clone(),
            clip_type: clip_type.to_string(),
            title: if clip_type == "screenshot" {
                format!("Screenshot import: {file_name}")
            } else {
                format!("File import: {file_name}")
            },
            created_at: now.clone(),
            updated_at: now,
            session_id,
            source: ClipSource {
                app_name: "Manual Import".to_string(),
                window_title: None,
                url: None,
                file_path: Some(file_name.clone()),
                sender: None,
                device_id: "local-desktop".to_string(),
                captured_via: "manual-import".to_string(),
                confidence: "medium".to_string(),
                fallback_reason: Some("User-selected file import; full local path not stored".to_string()),
            },
            privacy: ClipPrivacy {
                sensitive: true,
                masked: masked.unwrap_or(true),
                local_only: true,
                burn_after_use: false,
            },
            content: ClipContentRef {
                encrypted_payload_id: payload_id,
                safe_preview: format!("{file_name} · {} bytes", decoded.len()),
                revealed_payload: None,
                byte_size: Some(decoded.len()),
                mime_type: Some(mime_type),
            },
            transforms: Vec::new(),
        },
    );
    store.audit_events.insert(
        0,
        audit_event(
            "capture-started",
            format!("Imported encrypted {clip_type}: {file_name}"),
            Some(clip_id),
        ),
    );
    refresh_session_counts(&mut store);
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn reveal_clip(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    clip_id: String,
) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    let clip = store
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .cloned()
        .ok_or_else(|| "Clip not found".to_string())?;
    let payload = store
        .payloads
        .get(&clip.content.encrypted_payload_id)
        .ok_or_else(|| "Encrypted payload not found".to_string())?;
    let plaintext = decrypt_payload_with_key(&data_key, payload)?;

    store.audit_events.insert(
        0,
        audit_event(
            "clip-revealed",
            format!("Revealed selected clip: {}", clip.title),
            Some(clip.id.clone()),
        ),
    );

    if clip.privacy.burn_after_use {
        burn_clip(
            &mut store,
            &clip.id,
            format!("Revealed and burned selected clip: {}", clip.title),
        )?;
        save_store(&app, &store)?;
        return Ok(store.client_state());
    }

    save_store(&app, &store)?;

    Ok(store.client_state_with_revealed(Some((&clip.id, &plaintext))))
}

#[tauri::command]
fn transform_clip(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    clip_id: String,
    kind: TransformKind,
) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    let clip_index = store
        .clips
        .iter()
        .position(|clip| clip.id == clip_id)
        .ok_or_else(|| "Clip not found".to_string())?;
    let clip_type = store.clips[clip_index].clip_type.clone();
    let input_payload_id = store.clips[clip_index].content.encrypted_payload_id.clone();
    let burn_after_use = store.clips[clip_index].privacy.burn_after_use;
    let payload = store
        .payloads
        .get(&input_payload_id)
        .ok_or_else(|| "Encrypted payload not found".to_string())?;
    let plaintext = decrypt_payload_with_key(&data_key, payload)?;
    let transformed = if clip_type == "text" || clip_type == "link" {
        transform_text(&kind, &plaintext)?
    } else if (clip_type == "image" || clip_type == "screenshot") && matches!(kind, TransformKind::ResizeImage) {
        resize_image_payload(&plaintext)?
    } else if (clip_type == "image" || clip_type == "screenshot") && matches!(kind, TransformKind::Ocr) {
        ocr_image_payload(&app, &plaintext)?
    } else {
        return Err("This transform is not available for this clip type".to_string());
    };
    let output_payload_id = unique_id("payload");
    let transform_id = unique_id("transform");
    let now = now_iso();
    let encrypted_payload = encrypt_payload_with_key(&data_key, &transformed)?;

    store
        .payloads
        .insert(output_payload_id.clone(), encrypted_payload);

    let clip = &mut store.clips[clip_index];
    clip.transforms.insert(
        0,
        serde_json::json!({
            "id": transform_id,
            "kind": kind.as_str(),
            "createdAt": now,
            "outputPayloadId": output_payload_id,
            "safePreview": safe_preview(&transformed)
        }),
    );
    clip.updated_at = now_iso();
    let audit_title = clip.title.clone();
    let audit_clip_id = clip.id.clone();

    if matches!(kind, TransformKind::ResizeImage) {
        set_clipboard_from_typed_payload(&clip_type, &transformed)?;
    } else {
        Clipboard::new()
            .map_err(|error| error.to_string())?
            .set_text(transformed)
            .map_err(|error| error.to_string())?;
    }

    store.audit_events.insert(
        0,
        audit_event(
            "transform-created",
            format!("Created {} transform for {}", kind.label(), audit_title),
            Some(audit_clip_id.clone()),
        ),
    );
    if burn_after_use {
        store.payloads.remove(&output_payload_id);
        burn_clip(
            &mut store,
            &audit_clip_id,
            format!(
                "Transformed and burned selected clip after {}: {}",
                kind.label(),
                audit_title
            ),
        )?;
    }
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn copy_clip_to_clipboard(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    clip_id: String,
) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    let clip_index = store
        .clips
        .iter()
        .position(|clip| clip.id == clip_id)
        .ok_or_else(|| "Clip not found".to_string())?;
    let clip = store.clips[clip_index].clone();
    let payload = store
        .payloads
        .get(&clip.content.encrypted_payload_id)
        .ok_or_else(|| "Encrypted payload not found".to_string())?;
    let plaintext = decrypt_payload_with_key(&data_key, payload)?;

    set_clipboard_from_payload(&clip, &plaintext)?;

    if clip.privacy.burn_after_use {
        burn_clip(
            &mut store,
            &clip.id,
            format!("Copied and burned selected clip: {}", clip.title),
        )?;
    } else {
        store.audit_events.insert(
            0,
            audit_event(
                "clip-copied",
                format!("Copied selected clip to clipboard: {}", clip.title),
                Some(clip.id),
            ),
        );
    }

    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn copy_transform_to_clipboard(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    clip_id: String,
    transform_id: String,
) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    let clip = store
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .cloned()
        .ok_or_else(|| "Clip not found".to_string())?;
    let transform = clip
        .transforms
        .iter()
        .find(|transform| transform.get("id").and_then(serde_json::Value::as_str) == Some(transform_id.as_str()))
        .ok_or_else(|| "Transform not found".to_string())?;
    let output_payload_id = transform
        .get("outputPayloadId")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "Transform output payload not found".to_string())?;
    let payload = store
        .payloads
        .get(output_payload_id)
        .ok_or_else(|| "Encrypted transform output not found".to_string())?;
    let plaintext = decrypt_payload_with_key(&data_key, payload)?;

    Clipboard::new()
        .map_err(|error| error.to_string())?
        .set_text(plaintext)
        .map_err(|error| error.to_string())?;
    store.audit_events.insert(
        0,
        audit_event(
            "clip-copied",
            format!("Copied transform output for clip: {}", clip.title),
            Some(clip.id),
        ),
    );
    save_store(&app, &store)?;

    Ok(store.client_state())
}

#[tauri::command]
fn export_clip(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    clip_id: String,
    user_confirmed: bool,
) -> Result<ClientState, String> {
    if !user_confirmed {
        return Err("Export requires explicit user confirmation".to_string());
    }

    let mut store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    let clip = store
        .clips
        .iter()
        .find(|clip| clip.id == clip_id)
        .cloned()
        .ok_or_else(|| "Clip not found".to_string())?;

    let payload = store
        .payloads
        .get(&clip.content.encrypted_payload_id)
        .ok_or_else(|| "Encrypted payload not found".to_string())?;
    let plaintext = decrypt_payload_with_key(&data_key, payload)?;
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
            "privacy": {
                "localOnly": clip.privacy.local_only,
                "burnAfterUse": clip.privacy.burn_after_use,
                "maskedAtExport": clip.privacy.masked
            },
            "content": plaintext
        },
        "redactions": if clip.privacy.local_only {
            vec!["private source metadata", "local-only clip exported as plaintext by explicit selected export confirmation"]
        } else {
            vec!["private source metadata"]
        }
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
    if clip.privacy.burn_after_use {
        burn_clip(
            &mut store,
            &clip.id,
            format!("Exported and burned selected clip: {}", clip.title),
        )?;
    }
    save_store(&app, &store)?;

    let mut state = store.client_state();
    state.last_export_path = Some(export_path.display().to_string());
    Ok(state)
}

#[tauri::command]
fn export_session(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    session_id: String,
    user_confirmed: bool,
) -> Result<ClientState, String> {
    if !user_confirmed {
        return Err("Export requires explicit user confirmation".to_string());
    }

    let mut store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    let session = store
        .sessions
        .iter()
        .find(|session| session.id == session_id)
        .cloned()
        .ok_or_else(|| "Session not found".to_string())?;
    let mut exported_clips = Vec::new();
    let mut redactions = Vec::new();
    let mut burn_after_export = Vec::new();

    for clip in store
        .clips
        .iter()
        .filter(|clip| clip.session_id == session.id)
        .cloned()
        .collect::<Vec<ClipRecord>>()
    {
        if clip.privacy.sensitive && clip.privacy.masked {
            redactions.push(format!("{} skipped because it is masked", clip.title));
            continue;
        }

        let payload = store
            .payloads
            .get(&clip.content.encrypted_payload_id)
            .ok_or_else(|| "Encrypted payload not found".to_string())?;
        let content = if clip.privacy.local_only {
            redactions.push(format!(
                "{} exported as safe preview because it is local-only",
                clip.title
            ));
            clip.content.safe_preview.clone()
        } else {
            decrypt_payload_with_key(&data_key, payload)?
        };

        exported_clips.push(serde_json::json!({
            "id": clip.id,
            "title": clip.title,
            "type": clip.clip_type,
            "createdAt": clip.created_at,
            "source": {
                "appName": clip.source.app_name,
                "capturedVia": clip.source.captured_via,
                "confidence": clip.source.confidence
            },
            "content": content
        }));

        if clip.privacy.burn_after_use {
            burn_after_export.push((clip.id.clone(), clip.title.clone()));
        }
    }

    if exported_clips.is_empty() {
        return Err("No exportable clips in this session".to_string());
    }

    let export_id = unique_id("export");
    let export_path = export_dir(&app)?.join(format!("{export_id}.json"));
    let body = serde_json::json!({
        "id": export_id,
        "createdAt": now_iso(),
        "destination": "agent handoff",
        "scope": "session",
        "session": {
            "id": session.id,
            "title": session.title
        },
        "clips": exported_clips,
        "redactions": redactions
    });
    let bytes = serde_json::to_vec_pretty(&body).map_err(|error| error.to_string())?;
    write_private_file(&export_path, &bytes)?;

    store.audit_events.insert(
        0,
        audit_event(
            "agent-export-created",
            format!("Exported session: {}", session.title),
            Some(session.id),
        ),
    );
    for (clip_id, title) in burn_after_export {
        burn_clip(
            &mut store,
            &clip_id,
            format!("Exported and burned session clip: {title}"),
        )?;
    }
    save_store(&app, &store)?;

    let mut state = store.client_state();
    state.last_export_path = Some(export_path.display().to_string());
    Ok(state)
}

#[tauri::command]
fn search_clips(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    query: String,
    session_id: Option<String>,
    clip_type: Option<String>,
    semantic: Option<bool>,
) -> Result<ClientState, String> {
    let store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return Ok(store.client_state());
    }

    let mut state = store.client_state();
    if semantic.unwrap_or(false) {
        let index = decrypt_semantic_index(&store, &data_key)?;
        let query_vector = semantic_vector(&query);
        let mut scored = index
            .records
            .iter()
            .filter_map(|record| {
                let clip = store.clips.iter().find(|clip| {
                    clip.id == record.clip_id
                        && clip.content.encrypted_payload_id == record.payload_id
                        && clip.updated_at == record.updated_at
                })?;
                if !session_id
                    .as_ref()
                    .map(|session_id| clip.session_id == *session_id)
                    .unwrap_or(true)
                    || !clip_type
                        .as_ref()
                        .map(|clip_type| clip.clip_type == *clip_type)
                        .unwrap_or(true)
                {
                    return None;
                }
                let score = cosine_similarity(&query_vector, &record.vector);
                if score <= 0.05 {
                    return None;
                }
                Some((score, clip.clone()))
            })
            .collect::<Vec<(f32, ClipRecord)>>();
        scored.sort_by(|left, right| right.0.partial_cmp(&left.0).unwrap_or(std::cmp::Ordering::Equal));
        state.clips = scored
            .into_iter()
            .map(|(score, mut clip)| {
                clip.content.safe_preview = format!("Semantic match · {:.0}%", score * 100.0);
                clip
            })
            .collect();
        return Ok(state);
    }

    state.clips = store
        .clips
        .iter()
        .filter(|clip| {
            session_id
                .as_ref()
                .map(|session_id| clip.session_id == *session_id)
                .unwrap_or(true)
                && clip_type
                    .as_ref()
                    .map(|clip_type| clip.clip_type == *clip_type)
                    .unwrap_or(true)
        })
        .filter_map(|clip| {
            let payload = store.payloads.get(&clip.content.encrypted_payload_id)?;
            let plaintext = decrypt_payload_with_key(&data_key, payload).ok()?;
            let haystack = format!(
                "{} {} {} {} {}",
                clip.title,
                clip.source.app_name,
                clip.source.fallback_reason.clone().unwrap_or_default(),
                clip.content.safe_preview,
                plaintext
            )
            .to_ascii_lowercase();

            if haystack.contains(&query) {
                let mut clip = clip.clone();
                clip.content.safe_preview = safe_preview(&plaintext);
                Some(clip)
            } else {
                None
            }
        })
        .collect();

    Ok(state)
}

#[tauri::command]
fn rebuild_semantic_index(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
) -> Result<ClientState, String> {
    let mut store = load_store(&app)?;
    let data_key = require_unlocked(&store, &runtime)?;
    let index = build_semantic_index(&store, &data_key)?;
    let payload = serde_json::to_string(&index).map_err(|error| error.to_string())?;
    store.semantic_index = Some(encrypt_payload_with_key(&data_key, &payload)?);
    store.audit_events.insert(
        0,
        audit_event(
            "transform-created",
            format!("Rebuilt local semantic index with {} records", index.records.len()),
            None,
        ),
    );
    save_store(&app, &store)?;
    Ok(store.client_state())
}

#[tauri::command]
fn panic_wipe_clip(
    app: AppHandle,
    runtime: State<'_, Mutex<RuntimeSecrets>>,
    clip_id: String,
    confirmation: String,
) -> Result<ClientState, String> {
    if confirmation != "WIPE" {
        return Err("Panic wipe requires WIPE confirmation".to_string());
    }

    let mut store = load_store(&app)?;
    let _data_key = require_unlocked(&store, &runtime)?;
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

fn spawn_clipboard_watcher(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_millis(1500));

        let Ok(store) = load_store(&app) else {
            continue;
        };
        if store.locked {
            continue;
        }

        let Some(session) = store
            .sessions
            .iter()
            .find(|session| matches!(session.capture_state, CaptureState::Active))
            .cloned()
        else {
            continue;
        };
        let masked = session.default_privacy.masked;
        let session_id = session.id;
        let runtime_state = app.state::<Mutex<RuntimeSecrets>>();
        let Ok(runtime) = runtime_state.lock() else {
            continue;
        };
        let Some(data_key) = runtime.data_key else {
            continue;
        };
        drop(runtime);

        if let Ok((state, true)) = capture_clipboard_payload(&app, &data_key, session_id, masked) {
            let _ = app.emit("clipmind://state-changed", state);
        }
    });
}

fn setup_tray(app: &AppHandle) -> Result<(), String> {
    let show = MenuItem::with_id(app, "tray-show", "Show ClipMind", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let pause = MenuItem::with_id(app, "tray-pause", "Pause Capture", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let resume = MenuItem::with_id(app, "tray-resume", "Resume Capture", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let lock = MenuItem::with_id(app, "tray-lock", "Lock ClipMind", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let quit = MenuItem::with_id(app, "tray-quit", "Quit", true, None::<&str>)
        .map_err(|error| error.to_string())?;
    let menu = Menu::with_items(app, &[&show, &pause, &resume, &lock, &quit])
        .map_err(|error| error.to_string())?;

    TrayIconBuilder::with_id("clipmind")
        .tooltip("ClipMind")
        .icon(tray_icon_image())
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| {
            let id = event.id();
            if id == "tray-show" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                return;
            }

            if id == "tray-pause" {
                if let Ok(state) = pause_all_capture_sessions(app) {
                    let _ = app.emit("clipmind://state-changed", state);
                }
                return;
            }

            if id == "tray-resume" {
                if let Ok(state) = resume_primary_capture_session(app) {
                    let _ = app.emit("clipmind://state-changed", state);
                }
                return;
            }

            if id == "tray-lock" {
                let runtime_state = app.state::<Mutex<RuntimeSecrets>>();
                if let Ok(state) = lock_store(app, &runtime_state) {
                    let _ = app.emit("clipmind://state-changed", state);
                }
                return;
            }

            if id == "tray-quit" {
                app.exit(0);
            }
        })
        .build(app)
        .map_err(|error| error.to_string())?;

    Ok(())
}

fn image_from_clipmind_payload(plaintext: &str) -> Result<image::DynamicImage, String> {
    let value: serde_json::Value =
        serde_json::from_str(plaintext).map_err(|error| format!("Image payload parse failed: {error}"))?;
    let kind = value
        .get("kind")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "Image payload kind missing".to_string())?;

    if kind == "image-rgba" {
        let width = value
            .get("width")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| "Image width missing".to_string())? as u32;
        let height = value
            .get("height")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| "Image height missing".to_string())? as u32;
        let bytes = value
            .get("bytes")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "Image bytes missing".to_string())?;
        let decoded = general_purpose::STANDARD
            .decode(bytes)
            .map_err(|error| error.to_string())?;
        let image = image::RgbaImage::from_raw(width, height, decoded)
            .ok_or_else(|| "Image payload dimensions did not match byte length".to_string())?;
        return Ok(image::DynamicImage::ImageRgba8(image));
    }

    if kind == "file-bytes" {
        let bytes = value
            .get("bytes")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "Imported file bytes missing".to_string())?;
        let decoded = general_purpose::STANDARD
            .decode(bytes)
            .map_err(|error| error.to_string())?;
        return image::load_from_memory(&decoded).map_err(|error| format!("Image decode failed: {error}"));
    }

    Err("Unsupported image payload kind".to_string())
}

fn resize_image_payload(plaintext: &str) -> Result<String, String> {
    let image = image_from_clipmind_payload(plaintext)?;
    let width = image.width();
    let height = image.height();
    if width == 0 || height == 0 {
        return Err("Image payload had invalid dimensions".to_string());
    }

    let max_edge = 1024u32;
    let scale = (max_edge as f32 / width.max(height) as f32).min(1.0);
    let target_width = ((width as f32 * scale).round() as u32).max(1);
    let target_height = ((height as f32 * scale).round() as u32).max(1);
    let resized = image.resize(target_width, target_height, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8();

    Ok(image_payload(
        target_width as usize,
        target_height as usize,
        rgba.as_raw(),
    ))
}

fn run_tesseract_stdout(image_path: &Path) -> Result<std::process::Output, String> {
    let mut child = Command::new("tesseract")
        .arg(image_path)
        .arg("stdout")
        .spawn()
        .map_err(|error| format!("OCR execution failed: {error}"))?;
    let started = Instant::now();
    let timeout = Duration::from_secs(20);

    loop {
        if child
            .try_wait()
            .map_err(|error| format!("OCR status check failed: {error}"))?
            .is_some()
        {
            return child
                .wait_with_output()
                .map_err(|error| format!("OCR output capture failed: {error}"));
        }

        if started.elapsed() >= timeout {
            let _ = child.kill();
            let _ = child.wait();
            return Err("OCR timed out after 20 seconds".to_string());
        }

        thread::sleep(Duration::from_millis(50));
    }
}

fn ocr_image_payload(app: &AppHandle, plaintext: &str) -> Result<String, String> {
    let version = Command::new("tesseract")
        .arg("--version")
        .output()
        .map_err(|_| "OCR requires the local `tesseract` binary to be installed".to_string())?;
    if !version.status.success() {
        return Err("OCR requires a working local `tesseract` binary".to_string());
    }

    let image = image_from_clipmind_payload(plaintext)?;
    let temp_dir = data_dir(app)?.join("tmp");
    fs::create_dir_all(&temp_dir).map_err(|error| error.to_string())?;
    let image_path = temp_dir.join(format!("ocr-{}.png", unique_id("image")));
    image
        .save(&image_path)
        .map_err(|error| format!("OCR image preparation failed: {error}"))?;

    let output = run_tesseract_stdout(&image_path);
    let _ = fs::remove_file(&image_path);
    let output = output?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        return Err("OCR completed but found no text".to_string());
    }

    Ok(text)
}

fn tray_icon_image() -> tauri::image::Image<'static> {
    let mut rgba = Vec::with_capacity(16 * 16 * 4);
    for y in 0..16 {
        for x in 0..16 {
            let border = x == 0 || y == 0 || x == 15 || y == 15;
            let (r, g, b) = if border { (35, 50, 47) } else { (71, 132, 111) };
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    tauri::image::Image::new_owned(rgba, 16, 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session(id: &str, capture_state: CaptureState) -> WorkSession {
        WorkSession {
            id: id.to_string(),
            title: id.to_string(),
            created_at: "2026-06-12T00:00:00.000Z".to_string(),
            updated_at: "2026-06-12T00:00:00.000Z".to_string(),
            capture_state,
            default_privacy: ClipPrivacyDefaults {
                masked: true,
                local_only: true,
                burn_after_use: false,
            },
            clip_count: 0,
            last_clip_at: None,
        }
    }

    #[test]
    fn activating_one_session_pauses_existing_active_sessions() {
        let mut store = AppStore::default();
        store.sessions = vec![
            test_session("first", CaptureState::Active),
            test_session("second", CaptureState::Paused),
            test_session("third", CaptureState::Active),
        ];

        let (event_type, _) =
            set_capture_state_in_store(&mut store, "second", CaptureState::Active).unwrap();

        assert_eq!(event_type, "capture-started");
        assert!(matches!(store.sessions[0].capture_state, CaptureState::Paused));
        assert!(matches!(store.sessions[1].capture_state, CaptureState::Active));
        assert!(matches!(store.sessions[2].capture_state, CaptureState::Paused));
    }

    #[test]
    fn pausing_one_session_does_not_change_other_sessions() {
        let mut store = AppStore::default();
        store.sessions = vec![
            test_session("first", CaptureState::Active),
            test_session("second", CaptureState::Active),
        ];

        let (event_type, _) =
            set_capture_state_in_store(&mut store, "first", CaptureState::Paused).unwrap();

        assert_eq!(event_type, "capture-paused");
        assert!(matches!(store.sessions[0].capture_state, CaptureState::Paused));
        assert!(matches!(store.sessions[1].capture_state, CaptureState::Active));
    }

    #[test]
    fn current_kdf_metadata_is_recorded_and_usable() {
        let mut store = AppStore::default();

        set_current_kdf_metadata(&mut store);
        let params = kdf_params_from_store(&store).unwrap();

        assert_eq!(store.kdf_algorithm.as_deref(), Some(KDF_ALGORITHM));
        assert_eq!(store.kdf_version, Some(KDF_VERSION));
        assert_eq!(params.output_len(), Some(32));
    }

    #[test]
    fn imported_file_payload_keeps_filename_without_local_path() {
        let file_name = safe_file_name("/Users/example/private/notes.txt");
        let payload = imported_file_payload(&file_name, "text/plain", "SGVsbG8=");
        let value: serde_json::Value = serde_json::from_str(&payload).unwrap();

        assert_eq!(file_name, "notes.txt");
        assert_eq!(value["kind"], "file-bytes");
        assert_eq!(value["fileName"], "notes.txt");
        assert_eq!(value["mimeType"], "text/plain");
        assert_eq!(value["bytes"], "SGVsbG8=");
    }

    #[test]
    fn resize_image_payload_preserves_valid_rgba_payload() {
        let rgba = vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];
        let payload = image_payload(2, 2, &rgba);
        let resized = resize_image_payload(&payload).unwrap();
        let value: serde_json::Value = serde_json::from_str(&resized).unwrap();

        assert_eq!(value["kind"], "image-rgba");
        assert_eq!(value["width"], 2);
        assert_eq!(value["height"], 2);
        assert!(value["bytes"].as_str().unwrap().len() > 10);
    }

    #[test]
    fn semantic_vectors_rank_related_text_higher() {
        let query = semantic_vector("clipboard encrypted memory");
        let related = semantic_vector("encrypted clipboard memory capture");
        let unrelated = semantic_vector("banana sunset bicycle");

        assert!(cosine_similarity(&query, &related) > cosine_similarity(&query, &unrelated));
    }
}

pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(RuntimeSecrets::default()))
        .setup(|app| {
            spawn_clipboard_watcher(app.handle().clone());
            setup_tray(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_status,
            load_state,
            refresh_state,
            lock_app,
            unlock_app,
            reset_store,
            create_session,
            set_capture_state,
            capture_clipboard_text,
            import_file_clip,
            reveal_clip,
            transform_clip,
            copy_clip_to_clipboard,
            copy_transform_to_clipboard,
            export_clip,
            export_session,
            rebuild_semantic_index,
            search_clips,
            panic_wipe_clip
        ])
        .run(tauri::generate_context!())
        .expect("error while running ClipMind");
}
