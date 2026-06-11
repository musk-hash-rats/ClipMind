import "./styles.css";
import { invoke } from "@tauri-apps/api/core";
import type { AuditEvent, CaptureState, ClipRecord, ClipType, SourceConfidence, WorkSession } from "./domain";

type ClipFilter = "all" | ClipType;

type NativeState = {
  sessions: WorkSession[];
  clips: ClipRecord[];
  auditEvents: AuditEvent[];
};

type AppState = {
  sessions: WorkSession[];
  clips: ClipRecord[];
  activeSessionId: string;
  selectedClipId: string;
  filter: ClipFilter;
  captureState: CaptureState;
  previewRevealed: boolean;
  statusNote: string;
  auditEvents: AuditEvent[];
  nativeReady: boolean;
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

let state: AppState = {
  sessions: [],
  clips: [],
  activeSessionId: "",
  selectedClipId: "",
  filter: "all",
  captureState: "paused",
  previewRevealed: false,
  statusNote: "Loading native store",
  auditEvents: [],
  nativeReady: false
};

const formatDateTime = (iso: string) =>
  new Date(iso).toLocaleString([], {
    dateStyle: "medium",
    timeStyle: "short"
  });

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
    nativeReady: true,
    statusNote
  });
};

const invokeNativeState = async (command: string, args?: Record<string, unknown>) => {
  const nativeState = await invoke<NativeState>(command, args);
  return nativeState;
};

const getAvailableClips = () => state.clips;

const getSessionClips = (sessionId: string) => getAvailableClips().filter((clip) => clip.sessionId === sessionId);

const getVisibleClips = () =>
  getSessionClips(state.activeSessionId).filter((clip) => state.filter === "all" || clip.type === state.filter);

const getActiveSession = () =>
  state.sessions.find((session) => session.id === state.activeSessionId) ?? state.sessions[0];

const getSelectedClip = () =>
  getAvailableClips().find((clip) => clip.id === state.selectedClipId) ??
  getVisibleClips()[0] ??
  getAvailableClips()[0];

const isPrivateMetadataHidden = (clip: ClipRecord) =>
  clip.privacy.sensitive && clip.privacy.masked && !(clip.id === state.selectedClipId && state.previewRevealed);

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
  if (clip.privacy.masked && !state.previewRevealed) {
    if (clip.type === "file") return "File preview hidden";
    if (clip.type === "image" || clip.type === "screenshot") return "Image preview hidden";
    return "Sensitive preview hidden";
  }

  return clip.content.safePreview;
};

const setState = (patch: Partial<AppState>) => {
  state = { ...state, ...patch };

  const visibleClips = getVisibleClips();
  if (!visibleClips.some((clip) => clip.id === state.selectedClipId) && visibleClips[0]) {
    state.selectedClipId = visibleClips[0].id;
    state.previewRevealed = false;
  }

  render();
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

  if (visibleClips.length === 0) {
    return `<div class="empty-state">No clips in this view</div>`;
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
              <span>${escapeHtml(formatDateTime(clip.createdAt))}</span>
              ${badges.map((badge) => `<span class="tag">${escapeHtml(badge)}</span>`).join("")}
            </div>
          </div>
          <div class="clip-actions">
            <button class="icon-btn" type="button" title="Transform clip" aria-label="Transform clip" data-tool="transform">✦</button>
            <button class="icon-btn" type="button" title="Paste clip" aria-label="Paste clip" data-tool="paste">↩</button>
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

  const isMasked = clip.privacy.masked && !state.previewRevealed;
  const previewClass = isMasked ? "masked-preview" : "revealed-preview";
  const revealLabel = state.previewRevealed ? "Mask Preview" : "Reveal Preview";

  return `
    <section class="preview-panel">
      <div class="preview-head">
        <div>
          <h2>Selected Clip</h2>
          <strong>${escapeHtml(clip.title)}</strong>
          <small>Meaning match · ${clip.source.confidence === "high" ? "92%" : "78%"}</small>
        </div>
        <span class="badge">${isMasked ? "Masked" : "Visible"}</span>
      </div>

      <div class="${previewClass}">${escapeHtml(getPreviewText(clip))}</div>
      ${
        clip.privacy.masked
          ? `<button class="quiet-action full" type="button" data-action="toggle-preview">${revealLabel}</button>`
          : ""
      }
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

const render = () => {
  const activeSession = getActiveSession();
  const selectedClip = getSelectedClip();
  const captureActive = state.captureState === "active";
  const activeSessionTitle = activeSession?.title ?? "No Session";
  const captureLabel = captureActive ? `Capturing to ${activeSessionTitle}` : "Capture Paused";
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
          <input value="Stripe webhook note from yesterday" />
        </label>
        <button class="status-lock" type="button" title="App locked">
          <span aria-hidden="true">●</span>
          Locked
        </button>
        <button class="icon-btn" type="button" title="Settings" aria-label="Settings">⚙</button>
      </header>

      <aside class="sidebar" aria-label="Work sessions">
        <button class="primary-action full" type="button" data-action="new-session">New Session</button>
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
          <button class="capture-toggle" type="button" data-action="toggle-capture">
            <span>Capture</span>
            <span class="switch" aria-hidden="true"></span>
          </button>
          <button class="quiet-action" type="button" data-action="capture-now">Capture Clipboard</button>
        </div>

        <div class="mode-row" aria-label="Clip filters">${renderFilters()}</div>
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
          <div class="tool-grid">
            <button type="button" data-tool="clean-formatting"><strong>⌁</strong><span>Clean</span></button>
            <button type="button" data-tool="markdown"><strong>Md</strong><span>Markdown</span></button>
            <button type="button" data-tool="summarize"><strong>Σ</strong><span>Summarize</span></button>
            <button type="button" data-tool="translate"><strong>文</strong><span>Translate</span></button>
            <button type="button" data-tool="fix-json"><strong>{ }</strong><span>Fix JSON</span></button>
            <button type="button" data-tool="ocr"><strong>Aa</strong><span>OCR</span></button>
          </div>
        </section>

        <section>
          <h2>Agent Handoff</h2>
          <button class="primary-action full" type="button" data-action="export">Export Selected</button>
          <button class="danger-action full" type="button" data-action="panic">Panic Wipe Clip</button>
        </section>

        <section>
          <h2>Audit Trail</h2>
          ${renderAuditTrail()}
        </section>
      </aside>
    </main>
  `;
};

app.addEventListener("click", (event) => {
  const target = event.target as HTMLElement;
  const sessionButton = target.closest<HTMLButtonElement>("[data-session-id]");
  const filterButton = target.closest<HTMLButtonElement>("[data-filter]");
  const clipCard = target.closest<HTMLElement>("[data-clip-id]");
  const actionButton = target.closest<HTMLButtonElement>("[data-action]");
  const toolButton = target.closest<HTMLButtonElement>("[data-tool]");

  if (sessionButton) {
    const activeSessionId = sessionButton.dataset.sessionId ?? state.activeSessionId;
    const session = state.sessions.find((item) => item.id === activeSessionId);

    setState({
      activeSessionId,
      captureState: session?.captureState ?? "paused",
      filter: "all",
      previewRevealed: false,
      statusNote: "session changed"
    });
    return;
  }

  if (filterButton) {
    setState({
      filter: (filterButton.dataset.filter as ClipFilter | undefined) ?? "all",
      previewRevealed: false,
      statusNote: "filter applied"
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
    void (async () => {
      try {
        const nativeState = await invokeNativeState("set_capture_state", {
          sessionId: state.activeSessionId,
          captureState
        });
        applyNativeState(nativeState, captureState === "active" ? "capture active" : "capture paused");
      } catch (error) {
        setState({ statusNote: `native capture state failed: ${String(error)}` });
      }
    })();
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

    void (async () => {
      try {
        const nativeState = await invokeNativeState("reveal_clip", { clipId: selectedClip.id });
        applyNativeState(nativeState, "preview revealed and audited");
        setState({ previewRevealed: true });
      } catch (error) {
        setState({ statusNote: `native reveal failed: ${String(error)}` });
      }
    })();
    return;
  }

  if (actionButton?.dataset.action === "capture-now") {
    if (!state.activeSessionId) {
      setState({ statusNote: "native session unavailable" });
      return;
    }

    void (async () => {
      try {
        const nativeState = await invokeNativeState("capture_clipboard_text", { sessionId: state.activeSessionId });
        applyNativeState(nativeState, "clipboard captured");
      } catch (error) {
        setState({ statusNote: `native capture unavailable: ${String(error)}` });
      }
    })();
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

    const confirmed = window.confirm(
      `Export selected clip to agent handoff?\n\nClip: ${selectedClip.title}\nScope: selected clip only\nIncluded: safe preview, source label, session, timestamp\nAudit: export event will be recorded`
    );

    if (!confirmed) {
      setState({ statusNote: "export canceled" });
      return;
    }

    void (async () => {
      try {
        const nativeState = await invokeNativeState("export_clip", {
          clipId: selectedClip.id,
          userConfirmed: true
        });
        applyNativeState(nativeState, "export written and audited");
      } catch (error) {
        setState({ statusNote: `native export failed: ${String(error)}` });
      }
    })();
    return;
  }

  if (actionButton?.dataset.action === "panic") {
    const selectedClip = getSelectedClip();
    if (!selectedClip) {
      setState({ statusNote: "no clip selected" });
      return;
    }

    const confirmation = window.prompt(
      `Panic wipe is scoped to this selected ClipMind clip only:\n\n${selectedClip.title}\n\nType WIPE to remove it from this local session.`
    );

    if (confirmation !== "WIPE") {
      setState({ statusNote: "panic wipe canceled" });
      return;
    }

    void (async () => {
      try {
        const nativeState = await invokeNativeState("panic_wipe_clip", {
          clipId: selectedClip.id,
          confirmation
        });
        applyNativeState(nativeState, "selected clip wiped");
        setState({ previewRevealed: false });
      } catch (error) {
        setState({ statusNote: `native wipe failed: ${String(error)}` });
      }
    })();
    return;
  }

  if (actionButton?.dataset.action === "new-session") {
    const title = window.prompt("Session name");
    if (!title) {
      setState({ statusNote: "new session canceled" });
      return;
    }

    void (async () => {
      try {
        const nativeState = await invokeNativeState("create_session", { title });
        applyNativeState(nativeState, "session created");
      } catch (error) {
        setState({ statusNote: `native session create failed: ${String(error)}` });
      }
    })();
    return;
  }

  if (toolButton) {
    setState({ statusNote: `${toolButton.dataset.tool ?? "tool"} queued` });
  }
});

render();

void (async () => {
  try {
    const nativeState = await invokeNativeState("load_state");
    applyNativeState(nativeState, "native store loaded");
  } catch {
    setState({ statusNote: "browser preview mode" });
  }
})();
