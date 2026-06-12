import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AuditEvent, CaptureState, ClipRecord, ClipType, SourceConfidence, TransformKind, WorkSession } from "./domain";

type ClipFilter = "all" | ClipType;
type SearchMode = "exact" | "semantic";

type NativeState = {
  sessions: WorkSession[];
  clips: ClipRecord[];
  auditEvents: AuditEvent[];
  locked: boolean;
  lastExportPath?: string;
  authConfigured: boolean;
};

type ImagePayload = {
  kind: "image-rgba";
  width: number;
  height: number;
  bytes: string;
};

type FilePayload = {
  kind: "file-bytes";
  fileName: string;
  mimeType: string;
  bytes: string;
};

type ModalState =
  | { kind: "unlock"; authConfigured: boolean }
  | { kind: "reset-store" }
  | { kind: "new-session" }
  | { kind: "export-clip"; clipId: string; clipTitle: string; burnAfterUse: boolean; localOnly: boolean }
  | { kind: "export-session"; sessionId: string; sessionTitle: string }
  | { kind: "panic"; clipId: string; clipTitle: string }
  | null;

type AppState = {
  sessions: WorkSession[];
  clips: ClipRecord[];
  activeSessionId: string;
  selectedClipId: string;
  filter: ClipFilter;
  searchQuery: string;
  searchMode: SearchMode;
  nativeSearchActive: boolean;
  captureState: CaptureState;
  locked: boolean;
  authConfigured: boolean;
  settingsOpen: boolean;
  modal: ModalState;
  pendingAction: string;
  previewRevealed: boolean;
  statusNote: string;
  auditEvents: AuditEvent[];
  nativeReady: boolean;
  settings: {
    captureOnLaunch: boolean;
    autoLaunch: boolean;
    maskByDefault: boolean;
    exportAuditLog: boolean;
    lockTimeoutMinutes: number;
  };
};

const typeIcon: Record<ClipType, string> = {
  text: "T",
  link: "↗",
  image: "▧",
  screenshot: "▧",
  file: "□"
};

const confidenceLabel: Record<SourceConfidence, string> = {
  high: "High",
  medium: "Medium",
  fallback: "Fallback"
};

const filterLabels: Array<{ id: ClipFilter; label: string }> = [
  { id: "all", label: "All" },
  { id: "text", label: "Text" },
  { id: "link", label: "Links" },
  { id: "image", label: "Images" },
  { id: "screenshot", label: "Screenshots" },
  { id: "file", label: "Files" }
];

const defaultSettings: AppState["settings"] = {
  captureOnLaunch: false,
  autoLaunch: false,
  maskByDefault: true,
  exportAuditLog: true,
  lockTimeoutMinutes: 15
};

const transformLabels: Record<TransformKind, string> = {
  "clean-formatting": "Clean",
  markdown: "Markdown",
  summarize: "Summarize",
  translate: "Translate",
  "fix-json": "Fix JSON",
  "strip-tracking": "Strip Tracking",
  ocr: "OCR",
  "extract-links": "Extract Links",
  "resize-image": "Resize",
  "note-to-task": "Task",
  "note-to-message": "Message",
  "note-to-email": "Email"
};

const escapeHtml = (value: string | number | boolean | undefined) =>
  String(value ?? "").replace(/[&<>"']/g, (char) => {
    const entities: Record<string, string> = {
      "&": "&amp;",
      "<": "&lt;",
      ">": "&gt;",
      '"': "&quot;",
      "'": "&#39;"
    };

    return entities[char] ?? char;
  });

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Missing app root");
}

const loadSettings = () => {
  try {
    return {
      ...defaultSettings,
      ...JSON.parse(window.localStorage.getItem("clipmind-settings") ?? "{}")
    };
  } catch {
    return defaultSettings;
  }
};

const saveSettings = (settings: AppState["settings"]) => {
  window.localStorage.setItem("clipmind-settings", JSON.stringify(settings));
};

let state: AppState = {
  sessions: [],
  clips: [],
  activeSessionId: "",
  selectedClipId: "",
  filter: "all",
  searchQuery: "",
  searchMode: "exact",
  nativeSearchActive: false,
  captureState: "paused",
  locked: true,
  authConfigured: false,
  settingsOpen: false,
  modal: null,
  pendingAction: "",
  previewRevealed: false,
  statusNote: "Loading native store",
  auditEvents: [],
  nativeReady: false,
  settings: loadSettings()
};

let searchTimer: number | undefined;

const formatDateTime = (iso: string) =>
  new Date(iso).toLocaleString([], {
    dateStyle: "medium",
    timeStyle: "short"
  });

const getStatusNote = (nativeState: NativeState, statusNote: string) =>
  nativeState.lastExportPath ? `${statusNote}: ${nativeState.lastExportPath}` : statusNote;

const applyNativeState = (nativeState: NativeState, statusNote: string) => {
  const activeSession = nativeState.sessions.find((session) => session.id === state.activeSessionId) ?? nativeState.sessions[0];
  const selectedClip = nativeState.clips.find((clip) => clip.id === state.selectedClipId) ?? nativeState.clips[0];

  setState({
    sessions: nativeState.sessions,
    clips: nativeState.clips,
    auditEvents: nativeState.auditEvents,
    activeSessionId: activeSession?.id ?? "",
    selectedClipId: selectedClip?.id ?? "",
    captureState: activeSession?.captureState ?? "paused",
    locked: nativeState.locked,
    authConfigured: nativeState.authConfigured,
    nativeReady: true,
    statusNote: getStatusNote(nativeState, statusNote)
  });
};

const invokeNativeState = async (command: string, args?: Record<string, unknown>) => {
  const nativeState = await invoke<NativeState>(command, args);
  return nativeState;
};

const disabledAttr = (disabled: boolean) => (disabled ? "disabled" : "");

const isBusy = () => Boolean(state.pendingAction);

const isProtectedDisabled = () => state.locked || isBusy();

const protectedTitle = () => {
  if (state.locked) return "Unlock ClipMind first";
  if (state.pendingAction) return `Working: ${state.pendingAction}`;
  return "";
};

const runNativeAction = async (
  label: string,
  action: () => Promise<void>,
  failurePrefix: string
) => {
  if (state.pendingAction) return;

  setState({ pendingAction: label, statusNote: label });
  try {
    await action();
  } catch (error) {
    setState({ statusNote: `${failurePrefix}: ${String(error)}` });
  } finally {
    setState({ pendingAction: "" });
  }
};

const getAvailableClips = () => state.clips;

const getSessionClips = (sessionId: string) => getAvailableClips().filter((clip) => clip.sessionId === sessionId);

const getVisibleClips = () =>
  getSessionClips(state.activeSessionId).filter((clip) => {
    const matchesType = state.filter === "all" || clip.type === state.filter;
    const query = state.searchQuery.trim().toLowerCase();

    if (!matchesType) return false;
    if (state.nativeSearchActive) return true;
    if (!query) return true;

    const searchText = [
      clip.title,
      clip.content.safePreview,
      clip.source.appName,
      clip.source.windowTitle,
      clip.source.url,
      clip.source.filePath,
      clip.source.sender,
      clip.source.fallbackReason,
      ...clip.transforms.map((transform) => `${transform.kind} ${transform.safePreview ?? ""}`)
    ]
      .filter(Boolean)
      .join(" ")
      .toLowerCase();

    return searchText.includes(query);
  });

const getActiveSession = () =>
  state.sessions.find((session) => session.id === state.activeSessionId) ?? state.sessions[0];

const getSelectedClip = () =>
  getAvailableClips().find((clip) => clip.id === state.selectedClipId) ??
  getVisibleClips()[0] ??
  getAvailableClips()[0];

const isPrivateMetadataHidden = (clip: ClipRecord) =>
  state.locked || (clip.privacy.sensitive && clip.privacy.masked && !(clip.id === state.selectedClipId && state.previewRevealed));

const getOrigin = (clip: ClipRecord) => {
  if (isPrivateMetadataHidden(clip)) {
    return "Sensitive origin hidden";
  }

  return (
    clip.source.url ??
    clip.source.filePath ??
    clip.source.sender ??
    clip.source.fallbackReason ??
    clip.source.windowTitle ??
    "Unknown origin"
  );
};

const getClipBadges = (clip: ClipRecord) => [
  clip.source.capturedVia,
  clip.privacy.localOnly ? "local only" : "syncable",
  ...(clip.privacy.burnAfterUse ? ["burn after use"] : []),
  ...clip.transforms.map((transform) => transform.kind)
];

const getSessionIcon = (session: WorkSession) => {
  if (session.title.toLowerCase().includes("bug")) return "⌁";
  if (session.title.toLowerCase().includes("shopping")) return "▤";
  if (session.title.toLowerCase().includes("legal")) return "⚖";
  return "▣";
};

const getPreviewText = (clip: ClipRecord) => {
  if (state.locked) {
    return "Unlock ClipMind to reveal local working memory";
  }

  if (clip.privacy.masked && !state.previewRevealed) {
    if (clip.type === "file") return "File preview hidden";
    if (clip.type === "image" || clip.type === "screenshot") return "Image preview hidden";
    return "Sensitive preview hidden";
  }

  return clip.content.safePreview;
};

const parseImagePayload = (clip: ClipRecord): ImagePayload | null => {
  if (!(clip.type === "image" || clip.type === "screenshot") || !clip.content.revealedPayload) {
    return null;
  }

  try {
    const parsed = JSON.parse(clip.content.revealedPayload) as Partial<ImagePayload>;
    if (
      parsed.kind === "image-rgba" &&
      typeof parsed.width === "number" &&
      typeof parsed.height === "number" &&
      typeof parsed.bytes === "string"
    ) {
      return parsed as ImagePayload;
    }
  } catch {
    return null;
  }

  return null;
};

const parseFilePayload = (clip: ClipRecord): FilePayload | null => {
  if (!clip.content.revealedPayload) return null;

  try {
    const parsed = JSON.parse(clip.content.revealedPayload) as Partial<FilePayload>;
    if (
      parsed.kind === "file-bytes" &&
      typeof parsed.fileName === "string" &&
      typeof parsed.mimeType === "string" &&
      typeof parsed.bytes === "string"
    ) {
      return parsed as FilePayload;
    }
  } catch {
    return null;
  }

  return null;
};

const hydrateImagePreviews = () => {
  app.querySelectorAll<HTMLCanvasElement>("canvas[data-image-bytes]").forEach((canvas) => {
    const width = Number(canvas.dataset.imageWidth);
    const height = Number(canvas.dataset.imageHeight);
    const bytes = canvas.dataset.imageBytes;
    if (!width || !height || !bytes) return;

    try {
      const binary = atob(bytes);
      const rgba = new Uint8ClampedArray(binary.length);
      for (let index = 0; index < binary.length; index += 1) {
        rgba[index] = binary.charCodeAt(index);
      }

      canvas.width = width;
      canvas.height = height;
      canvas.getContext("2d")?.putImageData(new ImageData(rgba, width, height), 0, 0);
    } catch {
      canvas.replaceWith("Image preview could not be rendered");
    }
  });
};

const fileToBase64 = async (file: File) => {
  const bytes = new Uint8Array(await file.arrayBuffer());
  let binary = "";
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }
  return btoa(binary);
};

const importFile = (file: File, importKind: "file" | "screenshot") => {
  if (isProtectedDisabled()) {
    setState({ statusNote: protectedTitle() });
    return;
  }

  if (!state.activeSessionId) {
    setState({ statusNote: "native session unavailable" });
    return;
  }

  void runNativeAction(
    importKind === "screenshot" ? "importing screenshot" : "importing file",
    async () => {
      const nativeState = await invokeNativeState("import_file_clip", {
        sessionId: state.activeSessionId,
        fileName: file.name,
        mimeType: file.type || undefined,
        bytesBase64: await fileToBase64(file),
        masked: state.settings.maskByDefault,
        screenshot: importKind === "screenshot"
      });
      applyNativeState(nativeState, importKind === "screenshot" ? "screenshot imported" : "file imported");
    },
    importKind === "screenshot" ? "screenshot import failed" : "file import failed"
  );
};

const setState = (patch: Partial<AppState>) => {
  state = { ...state, ...patch };

  const visibleClips = getVisibleClips();
  if (!visibleClips.some((clip) => clip.id === state.selectedClipId) && visibleClips[0]) {
    state.selectedClipId = visibleClips[0].id;
    state.previewRevealed = false;
  }

  render();
  hydrateImagePreviews();
};

const renderSessions = () =>
  state.sessions.length === 0
    ? `<div class="empty-state">Native store unavailable</div>`
    : state.sessions
    .map((session) => {
      const count = getSessionClips(session.id).length || session.clipCount;

      return `
        <button class="session ${session.id === state.activeSessionId ? "active" : ""}" type="button" data-session-id="${escapeHtml(session.id)}">
          <span class="session-icon">${getSessionIcon(session)}</span>
          <span class="session-copy">
            <strong>${escapeHtml(session.title)}</strong>
            <small>${escapeHtml(session.captureState)} · ${session.defaultPrivacy.masked ? "masked" : "visible"} default</small>
          </span>
          <b>${escapeHtml(count)}</b>
        </button>
      `;
    })
    .join("");

const renderFilters = () =>
  filterLabels
    .map(
      (filter) => `
        <button class="mode ${filter.id === state.filter ? "active" : ""}" type="button" data-filter="${filter.id}">
          ${filter.label}
        </button>
      `
    )
    .join("");

const renderClips = () => {
  const visibleClips = getVisibleClips();
  const protectedDisabled = isProtectedDisabled();
  const actionTitle = protectedTitle();

  if (visibleClips.length === 0) {
    return `<div class="empty-state">${state.searchQuery.trim() ? "No clips match this search" : "No clips in this view"}</div>`;
  }

  return visibleClips
    .map((clip) => {
      const badges = getClipBadges(clip);

      return `
        <article class="clip ${clip.id === state.selectedClipId ? "selected" : ""}" data-clip-id="${escapeHtml(clip.id)}">
          <div class="clip-type ${clip.type}">${typeIcon[clip.type]}</div>
          <div class="clip-body">
            <h3>${escapeHtml(clip.title)}</h3>
            <p>${escapeHtml(getPreviewText(clip))}</p>
            <div class="clip-meta">
              <span>${escapeHtml(clip.source.appName)}</span>
              <span>${escapeHtml(getOrigin(clip))}</span>
              <span>${escapeHtml(state.locked ? "Locked" : formatDateTime(clip.createdAt))}</span>
              ${badges.map((badge) => `<span class="tag">${escapeHtml(badge)}</span>`).join("")}
            </div>
          </div>
          <div class="clip-actions">
            <button class="icon-btn" type="button" title="${escapeHtml(actionTitle || "Transform clip")}" aria-label="Transform clip" data-tool="transform" ${disabledAttr(protectedDisabled)}>✦</button>
            <button class="icon-btn" type="button" title="${escapeHtml(actionTitle || "Copy clip to clipboard")}" aria-label="Copy clip to clipboard" data-tool="paste" ${disabledAttr(protectedDisabled)}>↩</button>
          </div>
        </article>
      `;
    })
    .join("");
};

const renderPreview = (clip: ClipRecord | undefined) => {
  if (!clip) {
    return `
      <section class="preview-panel">
        <div class="preview-head">
          <div>
            <h2>Selected Clip</h2>
            <strong>No clip selected</strong>
            <small>Capture clipboard text to begin</small>
          </div>
          <span class="badge">Empty</span>
        </div>
        <div class="masked-preview">No local ClipMind data is loaded yet.</div>
      </section>
    `;
  }

  const isMasked = state.locked || (clip.privacy.masked && !state.previewRevealed);
  const previewClass = isMasked ? "masked-preview" : "revealed-preview";
  const revealLabel = state.previewRevealed ? "Mask Preview" : "Reveal Preview";
  const protectedDisabled = isProtectedDisabled();
  const imagePayload = isMasked ? null : parseImagePayload(clip);
  const filePayload = isMasked ? null : parseFilePayload(clip);
  const previewBody = imagePayload
    ? `
      <figure class="image-preview-frame">
        <canvas
          class="image-preview-canvas"
          data-image-width="${escapeHtml(imagePayload.width)}"
          data-image-height="${escapeHtml(imagePayload.height)}"
          data-image-bytes="${escapeHtml(imagePayload.bytes)}"
          aria-label="Revealed image preview"
        ></canvas>
        <figcaption>${escapeHtml(imagePayload.width)}x${escapeHtml(imagePayload.height)} · ${escapeHtml(clip.content.mimeType ?? "image payload")}</figcaption>
      </figure>
    `
    : filePayload?.mimeType.startsWith("image/")
      ? `
      <figure class="image-preview-frame">
        <img class="image-preview-canvas" src="data:${escapeHtml(filePayload.mimeType)};base64,${escapeHtml(filePayload.bytes)}" alt="Revealed imported image" />
        <figcaption>${escapeHtml(filePayload.fileName)} · ${escapeHtml(filePayload.mimeType)}</figcaption>
      </figure>
    `
      : filePayload
        ? `
      <div class="file-preview-detail">
        <strong>${escapeHtml(filePayload.fileName)}</strong>
        <span>${escapeHtml(filePayload.mimeType)} · ${escapeHtml(clip.content.byteSize ?? 0)} bytes</span>
      </div>
    `
    : escapeHtml(getPreviewText(clip));

  return `
    <section class="preview-panel">
      <div class="preview-head">
        <div>
          <h2>Selected Clip</h2>
          <strong>${escapeHtml(clip.title)}</strong>
          <small>${escapeHtml(clip.type)} clip · ${escapeHtml(confidenceLabel[clip.source.confidence])} source confidence</small>
        </div>
        <span class="badge">${isMasked ? "Masked" : "Visible"}</span>
      </div>

      <div class="${previewClass}">${previewBody}</div>
      ${
        clip.privacy.masked && !state.locked
          ? `<button class="quiet-action full" type="button" data-action="toggle-preview" ${disabledAttr(protectedDisabled)}>${revealLabel}</button>`
          : ""
      }
    </section>
  `;
};

const renderTransformHistory = (clip: ClipRecord | undefined) => {
  if (!clip) {
    return `<p class="audit-empty">Select a clip to run paste transformations</p>`;
  }

  if (clip.transforms.length === 0) {
    return `<p class="audit-empty">No transforms created for this clip yet</p>`;
  }

  return `
    <ol class="transform-list">
      ${clip.transforms
        .map(
          (transform) => `
            <li>
              <strong>${escapeHtml(transformLabels[transform.kind] ?? transform.kind)}</strong>
              <small>${escapeHtml(formatDateTime(transform.createdAt))}</small>
              <p>${escapeHtml(transform.safePreview ?? "Encrypted transform output stored locally")}</p>
              <button class="quiet-action mini" type="button" data-transform-copy="${escapeHtml(transform.id)}" ${disabledAttr(isProtectedDisabled())}>Copy Output</button>
            </li>
          `
        )
        .join("")}
    </ol>
  `;
};

const renderPasteTools = () => {
  const clip = getSelectedClip();
  const protectedDisabled = isProtectedDisabled();
  const actionTitle = protectedTitle();
  const textCompatible = Boolean(clip && (clip.type === "text" || clip.type === "link"));
  const imageCompatible = Boolean(clip && (clip.type === "image" || clip.type === "screenshot"));
  const textDisabled = disabledAttr(protectedDisabled || !textCompatible);
  const imageDisabled = disabledAttr(protectedDisabled || !imageCompatible);
  const title = escapeHtml(actionTitle);
  const textTitle = escapeHtml(actionTitle || (textCompatible ? "Run text transform" : "Select a text or link clip"));
  const imageTitle = escapeHtml(actionTitle || (imageCompatible ? "Run image transform" : "Select an image or screenshot clip"));
  const ocrTitle = escapeHtml(actionTitle || (imageCompatible ? "Run local OCR with Tesseract" : "Select an image or screenshot clip"));

  return `
    <div class="tool-grid">
      <button type="button" title="${textTitle}" data-tool="clean-formatting" ${textDisabled}><strong>⌁</strong><span>Clean</span></button>
      <button type="button" title="${textTitle}" data-tool="markdown" ${textDisabled}><strong>Md</strong><span>Markdown</span></button>
      <button type="button" title="${textTitle}" data-tool="summarize" ${textDisabled}><strong>Σ</strong><span>Summarize</span></button>
      <button type="button" title="${textTitle}" data-tool="fix-json" ${textDisabled}><strong>{ }</strong><span>Fix JSON</span></button>
      <button type="button" title="${textTitle}" data-tool="strip-tracking" ${textDisabled}><strong>⌫</strong><span>Clean URL</span></button>
      <button type="button" title="${textTitle}" data-tool="extract-links" ${textDisabled}><strong>↗</strong><span>Links</span></button>
      <button type="button" title="${textTitle}" data-tool="note-to-task" ${textDisabled}><strong>☑</strong><span>Task</span></button>
      <button type="button" title="${textTitle}" data-tool="note-to-message" ${textDisabled}><strong>@</strong><span>Message</span></button>
      <button type="button" title="${textTitle}" data-tool="note-to-email" ${textDisabled}><strong>✉</strong><span>Email</span></button>
      <button type="button" title="${title}" disabled><strong>文</strong><span>Translate</span></button>
      <button type="button" title="${ocrTitle}" data-tool="ocr" ${imageDisabled}><strong>Aa</strong><span>OCR</span></button>
      <button type="button" title="${imageTitle}" data-tool="resize-image" ${imageDisabled}><strong>↘</strong><span>Resize</span></button>
    </div>
  `;
};

const renderSettings = () => {
  if (!state.settingsOpen) return "";

  return `
    <section class="settings-panel" aria-label="ClipMind settings">
      <h2>Local Preferences</h2>
      <label><input type="checkbox" disabled /> Capture on launch</label>
      <label><input type="checkbox" disabled /> Auto-launch desktop app</label>
      <label><input type="checkbox" data-setting="maskByDefault" ${state.settings.maskByDefault ? "checked" : ""} /> Mask new clips by default</label>
      <label><input type="checkbox" checked disabled /> Keep export audit trail</label>
      <label class="number-setting">
        <span>Lock timeout</span>
        <input type="number" min="1" max="240" step="1" value="${escapeHtml(state.settings.lockTimeoutMinutes)}" disabled />
      </label>
    </section>
  `;
};

const renderAuditTrail = () => {
  if (state.auditEvents.length === 0) {
    return `<p class="audit-empty">No security events in this session yet</p>`;
  }

  return `
    <ol class="audit-list">
      ${state.auditEvents
        .slice(0, 4)
        .map(
          (event) => `
            <li>
              <strong>${escapeHtml(event.summary)}</strong>
              <small>${escapeHtml(formatDateTime(event.createdAt))}</small>
            </li>
          `
        )
        .join("")}
    </ol>
  `;
};

const renderModal = () => {
  if (!state.modal) return "";

  const busy = isBusy();
  const cancelButton = `<button class="quiet-action" type="button" data-modal-action="cancel" ${disabledAttr(busy)}>Cancel</button>`;

  if (state.modal.kind === "unlock") {
    return `
      <div class="modal-backdrop" role="presentation">
        <section class="modal" role="dialog" aria-modal="true" aria-labelledby="modal-title">
          <h2 id="modal-title">${state.modal.authConfigured ? "Unlock ClipMind" : "Set Unlock Passphrase"}</h2>
          <p>${state.modal.authConfigured ? "Enter your ClipMind passphrase to unlock local working memory." : "Create a local passphrase for this ClipMind store. Use at least 12 characters; forgotten passphrases require wiping this local store."}</p>
          <label class="modal-field">
            <span>Passphrase</span>
            <input data-modal-input="unlock-passphrase" type="password" minlength="12" autocomplete="current-password" />
          </label>
          <div class="modal-actions">
            <button class="danger-action" type="button" data-modal-action="open-reset-store" ${disabledAttr(busy)}>Reset Store</button>
            ${cancelButton}
            <button class="primary-action" type="button" data-modal-action="confirm-unlock" ${disabledAttr(busy)}>Unlock</button>
          </div>
        </section>
      </div>
    `;
  }

  if (state.modal.kind === "reset-store") {
    return `
      <div class="modal-backdrop" role="presentation">
        <section class="modal danger-modal" role="dialog" aria-modal="true" aria-labelledby="modal-title">
          <h2 id="modal-title">Reset Local Store</h2>
          <p>This wipes all ClipMind sessions, clips, encrypted payloads, exports, and passphrase metadata on this device. Forgotten passphrases cannot be recovered.</p>
          <label class="modal-field">
            <span>Type RESET to confirm</span>
            <input data-modal-input="reset-confirmation" autocomplete="off" />
          </label>
          <div class="modal-actions">
            ${cancelButton}
            <button class="danger-action" type="button" data-modal-action="confirm-reset-store" ${disabledAttr(busy)}>Reset Store</button>
          </div>
        </section>
      </div>
    `;
  }

  if (state.modal.kind === "new-session") {
    return `
      <div class="modal-backdrop" role="presentation">
        <section class="modal" role="dialog" aria-modal="true" aria-labelledby="modal-title">
          <h2 id="modal-title">New Session</h2>
          <label class="modal-field">
            <span>Session name</span>
            <input data-modal-input="session-title" maxlength="80" placeholder="Research, bug, client reply" />
          </label>
          <div class="modal-actions">
            ${cancelButton}
            <button class="primary-action" type="button" data-modal-action="confirm-new-session" ${disabledAttr(busy)}>Create</button>
          </div>
        </section>
      </div>
    `;
  }

  if (state.modal.kind === "export-clip") {
    return `
      <div class="modal-backdrop" role="presentation">
        <section class="modal" role="dialog" aria-modal="true" aria-labelledby="modal-title">
          <h2 id="modal-title">Export Selected Clip</h2>
          <p>Export full plaintext content for "${escapeHtml(state.modal.clipTitle)}" with source metadata for agent handoff.</p>
          ${state.modal.localOnly ? `<p class="warning-copy">This local-only clip will still export plaintext because you are explicitly exporting this selected clip.</p>` : ""}
          ${state.modal.burnAfterUse ? `<p class="warning-copy">This clip is burn-after-use and will be removed after export.</p>` : ""}
          <div class="modal-actions">
            ${cancelButton}
            <button class="primary-action" type="button" data-modal-action="confirm-export-clip" ${disabledAttr(busy)}>Export</button>
          </div>
        </section>
      </div>
    `;
  }

  if (state.modal.kind === "export-session") {
    return `
      <div class="modal-backdrop" role="presentation">
        <section class="modal" role="dialog" aria-modal="true" aria-labelledby="modal-title">
          <h2 id="modal-title">Export Session</h2>
          <p>Export all unmasked clips in "${escapeHtml(state.modal.sessionTitle)}". Current search and type filters do not change export scope.</p>
          <div class="modal-actions">
            ${cancelButton}
            <button class="primary-action" type="button" data-modal-action="confirm-export-session" ${disabledAttr(busy)}>Export Session</button>
          </div>
        </section>
      </div>
    `;
  }

  return `
    <div class="modal-backdrop" role="presentation">
      <section class="modal danger-modal" role="dialog" aria-modal="true" aria-labelledby="modal-title">
        <h2 id="modal-title">Panic Wipe Clip</h2>
        <p>Remove "${escapeHtml(state.modal.clipTitle)}" and its encrypted payload from this local ClipMind store.</p>
        <label class="modal-field">
          <span>Type WIPE to confirm</span>
          <input data-modal-input="wipe-confirmation" autocomplete="off" />
        </label>
        <div class="modal-actions">
          ${cancelButton}
          <button class="danger-action" type="button" data-modal-action="confirm-panic" ${disabledAttr(busy)}>Wipe Clip</button>
        </div>
      </section>
    </div>
  `;
};

const focusModal = () => {
  if (!state.modal) return;

  const input =
    app.querySelector<HTMLInputElement>("[data-modal-input]") ??
    app.querySelector<HTMLButtonElement>("[data-modal-action]:not([disabled])");
  input?.focus();
};

const render = () => {
  const activeSession = getActiveSession();
  const selectedClip = getSelectedClip();
  const captureActive = state.captureState === "active";
  const activeSessionTitle = activeSession?.title ?? "No Session";
  const protectedDisabled = isProtectedDisabled();
  const actionTitle = protectedTitle();
  const captureLabel = captureActive ? `Auto capture armed for ${activeSessionTitle}` : "Auto Capture Paused";
  const lockLabel = state.locked ? "Locked" : "Unlocked";
  const sourceSummary = selectedClip
    ? `${selectedClip.source.deviceId} · ${selectedClip.source.appName}`
    : `local-desktop · ${state.nativeReady ? "waiting for clipboard" : "native store offline"}`;

  app.innerHTML = `
    <main class="shell" aria-label="ClipMind desktop app">
      <header class="topbar">
        <div class="brand" aria-label="ClipMind">
          <span class="brand-mark">⌘</span>
          <span>ClipMind</span>
        </div>
        <label class="search" aria-label="Search working memory">
          <span aria-hidden="true">⌕</span>
          <input data-action="search" value="${escapeHtml(state.searchQuery)}" placeholder="Search clips, sources, transforms" />
        </label>
        <button class="status-lock ${state.locked ? "" : "unlocked"}" type="button" title="${escapeHtml(lockLabel)}" data-action="toggle-lock" ${disabledAttr(isBusy())}>
          <span aria-hidden="true">●</span>
          ${escapeHtml(lockLabel)}
        </button>
        <button class="icon-btn" type="button" title="Settings" aria-label="Settings" data-action="toggle-settings" ${disabledAttr(isBusy())}>⚙</button>
      </header>

      <aside class="sidebar" aria-label="Work sessions">
        <button class="primary-action full" type="button" data-action="new-session" title="${escapeHtml(actionTitle)}" ${disabledAttr(protectedDisabled)}>New Session</button>
        <h2>Work Sessions</h2>
        <nav class="session-list">${renderSessions()}</nav>
      </aside>

      <section class="workspace" aria-label="Active session clips">
        <div class="capture-bar ${captureActive ? "" : "paused"}">
          <div class="capture-state">
            <span class="capture-dot"></span>
            <div>
              <strong>${escapeHtml(captureLabel)}</strong>
              <small>${escapeHtml(sourceSummary)} · ${escapeHtml(state.statusNote)}</small>
            </div>
          </div>
          <button class="capture-toggle" type="button" data-action="toggle-capture" title="${escapeHtml(actionTitle)}" ${disabledAttr(protectedDisabled)}>
            <span>Auto</span>
            <span class="switch" aria-hidden="true"></span>
          </button>
          <button class="quiet-action" type="button" data-action="capture-now" title="${escapeHtml(actionTitle)}" ${disabledAttr(protectedDisabled)}>Capture Clipboard</button>
          <button class="quiet-action" type="button" data-action="import-file" title="${escapeHtml(actionTitle)}" ${disabledAttr(protectedDisabled)}>Import File</button>
          <button class="quiet-action" type="button" data-action="import-screenshot" title="${escapeHtml(actionTitle)}" ${disabledAttr(protectedDisabled)}>Import Screenshot</button>
          <input class="hidden-file-input" type="file" data-file-import="file" aria-hidden="true" tabindex="-1" />
          <input class="hidden-file-input" type="file" accept="image/*" data-file-import="screenshot" aria-hidden="true" tabindex="-1" />
        </div>

        <div class="mode-row" aria-label="Clip filters">${renderFilters()}</div>
        <div class="mode-row" aria-label="Search mode">
          <button class="mode ${state.searchMode === "exact" ? "active" : ""}" type="button" data-search-mode="exact">Exact</button>
          <button class="mode ${state.searchMode === "semantic" ? "active" : ""}" type="button" data-search-mode="semantic" ${disabledAttr(protectedDisabled)}>Semantic</button>
          <button class="quiet-action" type="button" data-action="rebuild-semantic" title="${escapeHtml(actionTitle)}" ${disabledAttr(protectedDisabled)}>Rebuild Semantic Index</button>
        </div>
        <section class="clip-list" aria-label="Clips">${renderClips()}</section>
      </section>

      <aside class="inspector" aria-label="Clip details and tools">
        ${renderPreview(selectedClip)}

        <section>
          <h2>Source Memory</h2>
          <dl class="meta-list">
            <div><dt>Source</dt><dd>${escapeHtml(selectedClip?.source.appName ?? "None")}</dd></div>
            <div><dt>Origin</dt><dd>${selectedClip ? escapeHtml(getOrigin(selectedClip)) : "None"}</dd></div>
            <div><dt>Session</dt><dd>${escapeHtml(activeSessionTitle)}</dd></div>
            <div><dt>Confidence</dt><dd>${selectedClip ? confidenceLabel[selectedClip.source.confidence] : "None"}</dd></div>
          </dl>
        </section>

        <section>
          <h2>Paste Tools</h2>
          ${renderPasteTools()}
        </section>

        <section>
          <h2>Transform History</h2>
          ${renderTransformHistory(selectedClip)}
        </section>

        <section>
          <h2>Agent Handoff</h2>
          <button class="primary-action full" type="button" data-action="export" title="${escapeHtml(actionTitle)}" ${disabledAttr(protectedDisabled)}>Export Selected</button>
          <button class="quiet-action full" type="button" data-action="export-session" title="${escapeHtml(actionTitle)}" ${disabledAttr(protectedDisabled)}>Export Session</button>
          <button class="danger-action full" type="button" data-action="panic" title="${escapeHtml(actionTitle)}" ${disabledAttr(protectedDisabled)}>Panic Wipe Clip</button>
        </section>

        <section>
          <h2>Audit Trail</h2>
          ${renderAuditTrail()}
        </section>
        ${renderSettings()}
      </aside>
      ${renderModal()}
    </main>
  `;
  queueMicrotask(focusModal);
};

app.addEventListener("click", (event) => {
  const target = event.target as HTMLElement;
  const sessionButton = target.closest<HTMLButtonElement>("[data-session-id]");
  const filterButton = target.closest<HTMLButtonElement>("[data-filter]");
  const searchModeButton = target.closest<HTMLButtonElement>("[data-search-mode]");
  const clipCard = target.closest<HTMLElement>("[data-clip-id]");
  const actionButton = target.closest<HTMLButtonElement>("[data-action]");
  const toolButton = target.closest<HTMLButtonElement>("[data-tool]");
  const modalButton = target.closest<HTMLButtonElement>("[data-modal-action]");
  const transformCopyButton = target.closest<HTMLButtonElement>("[data-transform-copy]");

  if (modalButton) {
    const modalAction = modalButton.dataset.modalAction;

    if (modalAction === "cancel") {
      setState({ modal: null, statusNote: "action canceled" });
      return;
    }

    if (modalAction === "open-reset-store") {
      setState({ modal: { kind: "reset-store" }, statusNote: "confirm local store reset" });
      return;
    }

    if (modalAction === "confirm-reset-store" && state.modal?.kind === "reset-store") {
      const confirmation =
        app.querySelector<HTMLInputElement>("[data-modal-input='reset-confirmation']")?.value.trim() ?? "";
      if (confirmation !== "RESET") {
        setState({ statusNote: "type RESET to confirm local store reset" });
        return;
      }

      void runNativeAction(
        "resetting local store",
        async () => {
          const nativeState = await invokeNativeState("reset_store", { confirmation });
          applyNativeState(nativeState, "local store reset");
          setState({ modal: null, previewRevealed: false, searchQuery: "", nativeSearchActive: false });
        },
        "native reset failed"
      );
      return;
    }

    if (modalAction === "confirm-unlock" && state.modal?.kind === "unlock") {
      const passphrase = app.querySelector<HTMLInputElement>("[data-modal-input='unlock-passphrase']")?.value ?? "";
      if (passphrase.trim().length < 12) {
        setState({ statusNote: "passphrase must be at least 12 characters" });
        return;
      }

      void runNativeAction(
        state.authConfigured ? "unlocking app" : "setting unlock passphrase",
        async () => {
          const nativeState = await invokeNativeState("unlock_app", { passphrase });
          applyNativeState(nativeState, state.authConfigured ? "app unlocked" : "passphrase set and app unlocked");
          setState({ modal: null, previewRevealed: false });
        },
        "native unlock failed"
      );
      return;
    }

    if (modalAction === "confirm-new-session" && state.modal?.kind === "new-session") {
      const title = app.querySelector<HTMLInputElement>("[data-modal-input='session-title']")?.value.trim() ?? "";
      if (!title) {
        setState({ statusNote: "session name is required" });
        return;
      }

      void runNativeAction(
        "creating session",
        async () => {
          const nativeState = await invokeNativeState("create_session", {
            title,
            maskByDefault: state.settings.maskByDefault
          });
          applyNativeState(nativeState, "session created");
          setState({ modal: null });
        },
        "native session create failed"
      );
      return;
    }

    if (modalAction === "confirm-export-clip" && state.modal?.kind === "export-clip") {
      const modal = state.modal;
      void runNativeAction(
        "exporting selected clip",
        async () => {
          const nativeState = await invokeNativeState("export_clip", {
            clipId: modal.clipId,
            userConfirmed: true
          });
          applyNativeState(
            nativeState,
            modal.burnAfterUse ? "export written and clip burned" : "export written"
          );
          setState({ modal: null });
        },
        "native export failed"
      );
      return;
    }

    if (modalAction === "confirm-export-session" && state.modal?.kind === "export-session") {
      const modal = state.modal;
      void runNativeAction(
        "exporting session",
        async () => {
          const nativeState = await invokeNativeState("export_session", {
            sessionId: modal.sessionId,
            userConfirmed: true
          });
          applyNativeState(nativeState, "session export written");
          setState({ modal: null });
        },
        "native session export failed"
      );
      return;
    }

    if (modalAction === "confirm-panic" && state.modal?.kind === "panic") {
      const modal = state.modal;
      const confirmation =
        app.querySelector<HTMLInputElement>("[data-modal-input='wipe-confirmation']")?.value.trim() ?? "";
      if (confirmation !== "WIPE") {
        setState({ statusNote: "type WIPE to confirm panic wipe" });
        return;
      }

      void runNativeAction(
        "wiping selected clip",
        async () => {
          const nativeState = await invokeNativeState("panic_wipe_clip", {
            clipId: modal.clipId,
            confirmation
          });
          applyNativeState(nativeState, "selected clip wiped");
          setState({ modal: null, previewRevealed: false });
        },
        "native wipe failed"
      );
      return;
    }
  }

  if (actionButton?.dataset.action === "toggle-lock") {
    if (state.locked) {
      setState({
        modal: { kind: "unlock", authConfigured: state.authConfigured },
        statusNote: state.authConfigured ? "unlock required" : "set unlock passphrase"
      });
      return;
    }

    void runNativeAction(
      "locking app",
      async () => {
        const nativeState = await invokeNativeState("lock_app");
        applyNativeState(nativeState, "app locked");
        setState({ previewRevealed: false });
      },
      "native lock failed"
    );
    return;
  }

  if (transformCopyButton) {
    const selectedClip = getSelectedClip();
    const transformId = transformCopyButton.dataset.transformCopy;
    if (!selectedClip || !transformId) {
      setState({ statusNote: "no transform selected" });
      return;
    }

    void runNativeAction(
      "copying transform output",
      async () => {
        const nativeState = await invokeNativeState("copy_transform_to_clipboard", {
          clipId: selectedClip.id,
          transformId
        });
        applyNativeState(nativeState, "transform output copied to clipboard");
      },
      "native transform copy failed"
    );
    return;
  }

  if (actionButton?.dataset.action === "toggle-settings") {
    setState({
      settingsOpen: !state.settingsOpen,
      statusNote: state.settingsOpen ? "settings closed" : "settings opened"
    });
    return;
  }

  if (sessionButton) {
    const activeSessionId = sessionButton.dataset.sessionId ?? state.activeSessionId;
    const session = state.sessions.find((item) => item.id === activeSessionId);

    setState({
      activeSessionId,
      captureState: session?.captureState ?? "paused",
      filter: "all",
      nativeSearchActive: false,
      previewRevealed: false,
      statusNote: "session changed"
    });
    return;
  }

  if (filterButton) {
    setState({
      filter: (filterButton.dataset.filter as ClipFilter | undefined) ?? "all",
      nativeSearchActive: false,
      previewRevealed: false,
      statusNote: "filter applied"
    });
    return;
  }

  if (searchModeButton) {
    setState({
      searchMode: (searchModeButton.dataset.searchMode as SearchMode | undefined) ?? "exact",
      nativeSearchActive: false,
      previewRevealed: false,
      statusNote: "search mode changed"
    });
    return;
  }

  if (clipCard && !toolButton) {
    setState({
      selectedClipId: clipCard.dataset.clipId ?? state.selectedClipId,
      previewRevealed: false,
      statusNote: "clip selected"
    });
    return;
  }

  if (actionButton?.dataset.action === "toggle-capture") {
    if (!state.activeSessionId) {
      setState({ statusNote: "native session unavailable" });
      return;
    }

    const captureState = state.captureState === "active" ? "paused" : "active";
    void runNativeAction(
      captureState === "active" ? "arming auto capture" : "pausing auto capture",
      async () => {
        const nativeState = await invokeNativeState("set_capture_state", {
          sessionId: state.activeSessionId,
          captureState
        });
        applyNativeState(nativeState, captureState === "active" ? "auto capture armed" : "auto capture paused");
      },
      "native capture state failed"
    );
    return;
  }

  if (actionButton?.dataset.action === "toggle-preview") {
    const selectedClip = getSelectedClip();
    if (!selectedClip) {
      setState({ statusNote: "no clip selected" });
      return;
    }

    if (state.previewRevealed) {
      setState({ previewRevealed: false, statusNote: "preview masked" });
      return;
    }

    void runNativeAction(
      "revealing preview",
      async () => {
        const nativeState = await invokeNativeState("reveal_clip", { clipId: selectedClip.id });
        applyNativeState(
          nativeState,
          selectedClip.privacy.burnAfterUse ? "preview revealed and clip burned" : "preview revealed and audited"
        );
        setState({ previewRevealed: !selectedClip.privacy.burnAfterUse });
      },
      "native reveal failed"
    );
    return;
  }

  if (actionButton?.dataset.action === "capture-now") {
    if (!state.activeSessionId) {
      setState({ statusNote: "native session unavailable" });
      return;
    }

    void runNativeAction(
      "capturing clipboard",
      async () => {
        const nativeState = await invokeNativeState("capture_clipboard_text", {
          sessionId: state.activeSessionId,
          masked: state.settings.maskByDefault
        });
        applyNativeState(nativeState, "clipboard captured");
      },
      "native capture unavailable"
    );
    return;
  }

  if (actionButton?.dataset.action === "rebuild-semantic") {
    void runNativeAction(
      "rebuilding semantic index",
      async () => {
        const nativeState = await invokeNativeState("rebuild_semantic_index");
        applyNativeState(nativeState, "semantic index rebuilt");
      },
      "semantic rebuild failed"
    );
    return;
  }

  if (actionButton?.dataset.action === "import-file" || actionButton?.dataset.action === "import-screenshot") {
    if (isProtectedDisabled()) {
      setState({ statusNote: protectedTitle() });
      return;
    }

    const importKind = actionButton.dataset.action === "import-screenshot" ? "screenshot" : "file";
    app.querySelector<HTMLInputElement>(`[data-file-import='${importKind}']`)?.click();
    return;
  }

  if (actionButton?.dataset.action === "export") {
    const selectedClip = getSelectedClip();
    if (!selectedClip) {
      setState({ statusNote: "no clip selected" });
      return;
    }

    if (isPrivateMetadataHidden(selectedClip)) {
      setState({ statusNote: "reveal masked clip before export" });
      return;
    }

    setState({
      modal: {
        kind: "export-clip",
        clipId: selectedClip.id,
        clipTitle: selectedClip.title,
        burnAfterUse: selectedClip.privacy.burnAfterUse,
        localOnly: selectedClip.privacy.localOnly
      },
      statusNote: "confirm selected clip export"
    });
    return;
  }

  if (actionButton?.dataset.action === "export-session") {
    const activeSession = getActiveSession();
    if (!activeSession) {
      setState({ statusNote: "no session selected" });
      return;
    }

    setState({
      modal: {
        kind: "export-session",
        sessionId: activeSession.id,
        sessionTitle: activeSession.title
      },
      statusNote: "confirm session export"
    });
    return;
  }

  if (actionButton?.dataset.action === "panic") {
    const selectedClip = getSelectedClip();
    if (!selectedClip) {
      setState({ statusNote: "no clip selected" });
      return;
    }

    setState({
      modal: {
        kind: "panic",
        clipId: selectedClip.id,
        clipTitle: selectedClip.title
      },
      statusNote: "confirm panic wipe"
    });
    return;
  }

  if (actionButton?.dataset.action === "new-session") {
    setState({ modal: { kind: "new-session" }, statusNote: "new session" });
    return;
  }

  if (toolButton) {
    const selectedClip = getSelectedClip();
    const kind = toolButton.dataset.tool as TransformKind | "transform" | "paste" | undefined;
    if (kind === "paste") {
      if (!selectedClip) {
        setState({ statusNote: "no clip selected" });
        return;
      }

      void runNativeAction(
        "copying selected clip",
        async () => {
          const nativeState = await invokeNativeState("copy_clip_to_clipboard", {
            clipId: selectedClip.id
          });
          applyNativeState(
            nativeState,
            selectedClip.privacy.burnAfterUse ? "clip copied and burned" : "clip copied to clipboard"
          );
          setState({ previewRevealed: false });
        },
        "native paste failed"
      );
      return;
    }

    if (!selectedClip || !kind || kind === "transform") {
      setState({ statusNote: "select a paste tool from the inspector" });
      return;
    }

    void runNativeAction(
      `${kind} transform`,
      async () => {
        const nativeState = await invokeNativeState("transform_clip", {
          clipId: selectedClip.id,
          kind
        });
        applyNativeState(nativeState, `${kind} copied to clipboard`);
      },
      "transform failed"
    );
  }
});

app.addEventListener("input", (event) => {
  const target = event.target as HTMLInputElement;
  const settingKey = target.dataset.setting as keyof AppState["settings"] | undefined;

  if (settingKey) {
    const settings = {
      ...state.settings,
      [settingKey]: target.type === "checkbox" ? target.checked : Number(target.value)
    };
    saveSettings(settings);
    setState({ settings, statusNote: "local preferences saved" });
    return;
  }

  if (target.dataset.action !== "search") return;

  const query = target.value;
  setState({
    searchQuery: query,
    nativeSearchActive: false,
    previewRevealed: false,
    statusNote: query.trim() ? "search applied" : "search cleared"
  });

  window.clearTimeout(searchTimer);
  if (state.locked) return;

  searchTimer = window.setTimeout(() => {
    void (async () => {
      try {
        const trimmed = query.trim();
        const nativeState = trimmed
          ? await invokeNativeState("search_clips", {
              query: trimmed,
              sessionId: state.activeSessionId || undefined,
              clipType: state.filter === "all" ? undefined : state.filter,
              semantic: state.searchMode === "semantic"
            })
          : await invokeNativeState("refresh_state");
        applyNativeState(nativeState, trimmed ? `${state.searchMode} search applied` : "native search cleared");
        setState({ nativeSearchActive: Boolean(trimmed) });
      } catch (error) {
        setState({ statusNote: `native search failed: ${String(error)}` });
      }
    })();
  }, 250);
});

app.addEventListener("change", (event) => {
  const input = event.target as HTMLInputElement | null;
  const importKind = input?.dataset.fileImport;
  const file = input?.files?.[0];
  if (!input || !importKind || !file) return;

  input.value = "";
  importFile(file, importKind === "screenshot" ? "screenshot" : "file");
});

app.addEventListener("dragover", (event) => {
  event.preventDefault();
  if (isProtectedDisabled()) {
    event.dataTransfer!.dropEffect = "none";
    return;
  }
  event.dataTransfer!.dropEffect = "copy";
});

app.addEventListener("drop", (event) => {
  event.preventDefault();
  const file = event.dataTransfer?.files?.[0];
  if (!file) {
    setState({ statusNote: "drop a file to import" });
    return;
  }

  importFile(file, file.type.startsWith("image/") ? "screenshot" : "file");
});

app.addEventListener("keydown", (event) => {
  if (event.key !== "Escape" || !state.modal || isBusy()) return;

  event.preventDefault();
  setState({ modal: null, statusNote: "action canceled" });
});

render();

void listen<NativeState>("clipmind://state-changed", (event) => {
  applyNativeState(event.payload, "clipboard change captured");
}).catch(() => {
  setState({ statusNote: "native capture listener unavailable" });
});

void (async () => {
  try {
    const nativeState = await invokeNativeState("load_state");
    applyNativeState(nativeState, "native store loaded");
  } catch {
    setState({ statusNote: "native runtime unavailable" });
  }
})();
