import "./styles.css";
import type { CaptureState, ClipRecord, ClipType, SourceConfidence, WorkSession } from "./domain";
import { sampleClips, sampleSessions } from "./sample-data";

type ClipFilter = "all" | ClipType;

type AppState = {
  activeSessionId: string;
  selectedClipId: string;
  filter: ClipFilter;
  captureState: CaptureState;
  previewRevealed: boolean;
  statusNote: string;
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

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Missing app root");
}

let state: AppState = {
  activeSessionId: sampleSessions[0]?.id ?? "",
  selectedClipId: sampleClips[0]?.id ?? "",
  filter: "all",
  captureState: sampleSessions[0]?.captureState ?? "paused",
  previewRevealed: false,
  statusNote: "Ready"
};

const formatDateTime = (iso: string) =>
  new Date(iso).toLocaleString([], {
    dateStyle: "medium",
    timeStyle: "short"
  });

const getSessionClips = (sessionId: string) => sampleClips.filter((clip) => clip.sessionId === sessionId);

const getVisibleClips = () =>
  getSessionClips(state.activeSessionId).filter((clip) => state.filter === "all" || clip.type === state.filter);

const getActiveSession = () =>
  sampleSessions.find((session) => session.id === state.activeSessionId) ?? sampleSessions[0];

const getSelectedClip = () =>
  sampleClips.find((clip) => clip.id === state.selectedClipId) ?? getVisibleClips()[0] ?? sampleClips[0];

const getOrigin = (clip: ClipRecord) =>
  clip.source.url ??
  clip.source.filePath ??
  clip.source.sender ??
  clip.source.fallbackReason ??
  clip.source.windowTitle ??
  "Unknown origin";

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
  sampleSessions
    .map((session) => {
      const count = getSessionClips(session.id).length || session.clipCount;

      return `
        <button class="session ${session.id === state.activeSessionId ? "active" : ""}" type="button" data-session-id="${session.id}">
          <span class="session-icon">${getSessionIcon(session)}</span>
          <span class="session-copy">
            <strong>${session.title}</strong>
            <small>${session.captureState} · ${session.defaultPrivacy.masked ? "masked" : "visible"} default</small>
          </span>
          <b>${count}</b>
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
        <article class="clip ${clip.id === state.selectedClipId ? "selected" : ""}" data-clip-id="${clip.id}">
          <div class="clip-type ${clip.type}">${typeIcon[clip.type]}</div>
          <div class="clip-body">
            <h3>${clip.title}</h3>
            <p>${getPreviewText(clip)}</p>
            <div class="clip-meta">
              <span>${clip.source.appName}</span>
              <span>${getOrigin(clip)}</span>
              <span>${formatDateTime(clip.createdAt)}</span>
              ${badges.map((badge) => `<span class="tag">${badge}</span>`).join("")}
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

const renderPreview = (clip: ClipRecord) => {
  const isMasked = clip.privacy.masked && !state.previewRevealed;
  const previewClass = isMasked ? "masked-preview" : "revealed-preview";
  const revealLabel = state.previewRevealed ? "Mask Preview" : "Reveal Preview";

  return `
    <section class="preview-panel">
      <div class="preview-head">
        <div>
          <h2>Selected Clip</h2>
          <strong>${clip.title}</strong>
          <small>Meaning match · ${clip.source.confidence === "high" ? "92%" : "78%"}</small>
        </div>
        <span class="badge">${isMasked ? "Masked" : "Visible"}</span>
      </div>

      <div class="${previewClass}">${getPreviewText(clip)}</div>
      ${
        clip.privacy.masked
          ? `<button class="quiet-action full" type="button" data-action="toggle-preview">${revealLabel}</button>`
          : ""
      }
    </section>
  `;
};

const render = () => {
  const activeSession = getActiveSession();
  const selectedClip = getSelectedClip();
  const captureActive = state.captureState === "active";
  const captureLabel = captureActive ? `Capturing to ${activeSession.title}` : "Capture Paused";

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
              <strong>${captureLabel}</strong>
              <small>${selectedClip.source.deviceId} · ${selectedClip.source.appName} · ${state.statusNote}</small>
            </div>
          </div>
          <button class="capture-toggle" type="button" data-action="toggle-capture">
            <span>Capture</span>
            <span class="switch" aria-hidden="true"></span>
          </button>
        </div>

        <div class="mode-row" aria-label="Clip filters">${renderFilters()}</div>
        <section class="clip-list" aria-label="Clips">${renderClips()}</section>
      </section>

      <aside class="inspector" aria-label="Clip details and tools">
        ${renderPreview(selectedClip)}

        <section>
          <h2>Source Memory</h2>
          <dl class="meta-list">
            <div><dt>Source</dt><dd>${selectedClip.source.appName}</dd></div>
            <div><dt>Origin</dt><dd>${getOrigin(selectedClip)}</dd></div>
            <div><dt>Session</dt><dd>${activeSession.title}</dd></div>
            <div><dt>Confidence</dt><dd>${confidenceLabel[selectedClip.source.confidence]}</dd></div>
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
          <button class="danger-action full" type="button" data-action="panic">Panic Wipe</button>
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
    const session = sampleSessions.find((item) => item.id === activeSessionId);

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
    setState({
      captureState: state.captureState === "active" ? "paused" : "active",
      statusNote: state.captureState === "active" ? "capture paused" : "capture active"
    });
    return;
  }

  if (actionButton?.dataset.action === "toggle-preview") {
    setState({
      previewRevealed: !state.previewRevealed,
      statusNote: state.previewRevealed ? "preview masked" : "preview revealed"
    });
    return;
  }

  if (actionButton?.dataset.action === "panic") {
    setState({ statusNote: "panic wipe confirmation pending" });
    return;
  }

  if (actionButton?.dataset.action === "export") {
    setState({ statusNote: "agent export staged" });
    return;
  }

  if (actionButton?.dataset.action === "new-session") {
    setState({ statusNote: "new session draft opened" });
    return;
  }

  if (toolButton) {
    setState({ statusNote: `${toolButton.dataset.tool ?? "tool"} queued` });
  }
});

render();
