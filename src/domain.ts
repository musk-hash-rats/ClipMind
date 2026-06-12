export type ClipType = "text" | "link" | "image" | "screenshot" | "file";

export type CaptureState = "active" | "paused" | "stopped";

export type SourceConfidence = "high" | "medium" | "fallback";

export type CapturedVia = "clipboard-listener" | "manual-import" | "drag-drop" | "agent-import";

export type TransformKind =
  | "clean-formatting"
  | "markdown"
  | "summarize"
  | "translate"
  | "fix-json"
  | "strip-tracking"
  | "ocr"
  | "extract-links"
  | "resize-image"
  | "note-to-task"
  | "note-to-message"
  | "note-to-email";

export type AgentExportFormat = "markdown" | "json";

export type AuditEventType =
  | "app-unlocked"
  | "app-locked"
  | "session-created"
  | "capture-started"
  | "capture-paused"
  | "clip-masked"
  | "clip-revealed"
  | "clip-expired"
  | "clip-copied"
  | "burn-after-use-consumed"
  | "panic-wipe-requested"
  | "panic-wipe-completed"
  | "local-store-reset"
  | "transform-created"
  | "agent-export-created"
  | "clip-transformed";

export interface ClipSource {
  appName: string;
  windowTitle?: string;
  url?: string;
  filePath?: string;
  sender?: string;
  deviceId: string;
  capturedVia: CapturedVia;
  confidence: SourceConfidence;
  fallbackReason?: string;
}

export interface ClipPrivacy {
  sensitive: boolean;
  masked: boolean;
  localOnly: boolean;
  burnAfterUse: boolean;
  expiresAt?: string;
}

export interface ClipContentRef {
  encryptedPayloadId: string;
  safePreview: string;
  revealedPayload?: string;
  byteSize?: number;
  mimeType?: string;
}

export interface ClipTransform {
  id: string;
  kind: TransformKind;
  createdAt: string;
  outputPayloadId?: string;
  safePreview?: string;
}

export interface ClipRecord {
  id: string;
  type: ClipType;
  title: string;
  createdAt: string;
  updatedAt: string;
  sessionId: string;
  source: ClipSource;
  privacy: ClipPrivacy;
  content: ClipContentRef;
  transforms: ClipTransform[];
}

export interface WorkSession {
  id: string;
  title: string;
  createdAt: string;
  updatedAt: string;
  captureState: CaptureState;
  defaultPrivacy: Pick<ClipPrivacy, "masked" | "localOnly" | "burnAfterUse">;
  clipCount: number;
  lastClipAt?: string;
}

export interface AgentExport {
  id: string;
  sessionId: string;
  clipIds: string[];
  format: AgentExportFormat;
  createdAt: string;
  destination: string;
  includedFields: string[];
  redactions: string[];
  userConfirmed: boolean;
}

export interface AuditEvent {
  id: string;
  type: AuditEventType;
  createdAt: string;
  actor: "local-user" | "system";
  targetId?: string;
  summary: string;
  metadata?: Record<string, string | number | boolean>;
}
