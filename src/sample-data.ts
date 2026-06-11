import type { ClipRecord, WorkSession } from "./domain";

export const sampleSessions: WorkSession[] = [
  {
    id: "session-client-reply",
    title: "Client Reply",
    createdAt: "2026-06-11T14:20:00.000Z",
    updatedAt: "2026-06-11T15:05:00.000Z",
    captureState: "active",
    defaultPrivacy: {
      masked: true,
      localOnly: true,
      burnAfterUse: false
    },
    clipCount: 18,
    lastClipAt: "2026-06-11T15:04:00.000Z"
  },
  {
    id: "session-webhook-bug",
    title: "Webhook Bug",
    createdAt: "2026-06-11T13:10:00.000Z",
    updatedAt: "2026-06-11T15:18:00.000Z",
    captureState: "paused",
    defaultPrivacy: {
      masked: true,
      localOnly: true,
      burnAfterUse: false
    },
    clipCount: 7,
    lastClipAt: "2026-06-11T15:18:00.000Z"
  },
  {
    id: "session-shopping",
    title: "Shopping",
    createdAt: "2026-06-10T18:10:00.000Z",
    updatedAt: "2026-06-11T13:30:00.000Z",
    captureState: "stopped",
    defaultPrivacy: {
      masked: false,
      localOnly: true,
      burnAfterUse: false
    },
    clipCount: 5,
    lastClipAt: "2026-06-11T13:30:00.000Z"
  },
  {
    id: "session-legal-docs",
    title: "Legal Docs",
    createdAt: "2026-06-09T16:00:00.000Z",
    updatedAt: "2026-06-11T12:40:00.000Z",
    captureState: "stopped",
    defaultPrivacy: {
      masked: true,
      localOnly: true,
      burnAfterUse: true
    },
    clipCount: 9,
    lastClipAt: "2026-06-11T12:40:00.000Z"
  }
];

export const sampleClips: ClipRecord[] = [
  {
    id: "clip-stripe-webhook",
    type: "text",
    title: "Stripe webhook retry note",
    createdAt: "2026-06-10T20:12:00.000Z",
    updatedAt: "2026-06-10T20:12:00.000Z",
    sessionId: "session-client-reply",
    source: {
      appName: "Chrome",
      windowTitle: "Stripe Docs",
      url: "https://docs.stripe.com/webhooks",
      deviceId: "local-desktop",
      capturedVia: "clipboard-listener",
      confidence: "high"
    },
    privacy: {
      sensitive: true,
      masked: true,
      localOnly: true,
      burnAfterUse: false
    },
    content: {
      encryptedPayloadId: "payload-clip-stripe-webhook",
      safePreview: "Retry policy captured from docs."
    },
    transforms: []
  },
  {
    id: "clip-pricing-link",
    type: "link",
    title: "Pricing page with tracking removed",
    createdAt: "2026-06-11T13:30:00.000Z",
    updatedAt: "2026-06-11T13:30:00.000Z",
    sessionId: "session-client-reply",
    source: {
      appName: "Safari",
      windowTitle: "Pricing",
      url: "https://example.com/pricing",
      deviceId: "local-desktop",
      capturedVia: "clipboard-listener",
      confidence: "high"
    },
    privacy: {
      sensitive: false,
      masked: false,
      localOnly: true,
      burnAfterUse: false
    },
    content: {
      encryptedPayloadId: "payload-clip-pricing-link",
      safePreview: "https://example.com/pricing"
    },
    transforms: [
      {
        id: "transform-pricing-strip",
        kind: "strip-tracking",
        createdAt: "2026-06-11T13:30:05.000Z",
        safePreview: "Tracking parameters removed."
      }
    ]
  },
  {
    id: "clip-checkout-screenshot",
    type: "screenshot",
    title: "Screenshot: checkout error state",
    createdAt: "2026-06-11T14:44:00.000Z",
    updatedAt: "2026-06-11T14:44:00.000Z",
    sessionId: "session-client-reply",
    source: {
      appName: "Screenshot",
      windowTitle: "Checkout",
      deviceId: "local-desktop",
      capturedVia: "clipboard-listener",
      confidence: "medium"
    },
    privacy: {
      sensitive: true,
      masked: true,
      localOnly: true,
      burnAfterUse: false
    },
    content: {
      encryptedPayloadId: "payload-clip-checkout-screenshot",
      safePreview: "Screenshot captured with OCR ready."
    },
    transforms: [
      {
        id: "transform-checkout-ocr",
        kind: "ocr",
        createdAt: "2026-06-11T14:44:20.000Z",
        safePreview: "OCR queued."
      }
    ]
  },
  {
    id: "clip-invoice-pdf",
    type: "file",
    title: "Client invoice PDF",
    createdAt: "2026-06-10T18:03:00.000Z",
    updatedAt: "2026-06-10T18:03:00.000Z",
    sessionId: "session-legal-docs",
    source: {
      appName: "Files",
      filePath: "~/Downloads/client-invoice.pdf",
      deviceId: "local-desktop",
      capturedVia: "drag-drop",
      confidence: "fallback",
      fallbackReason: "No source window available for dragged file."
    },
    privacy: {
      sensitive: true,
      masked: true,
      localOnly: true,
      burnAfterUse: true
    },
    content: {
      encryptedPayloadId: "payload-clip-invoice-pdf",
      safePreview: "Encrypted file clip with burn-after-use enabled.",
      mimeType: "application/pdf"
    },
    transforms: []
  },
  {
    id: "clip-webhook-payload",
    type: "text",
    title: "Webhook payload sample",
    createdAt: "2026-06-11T15:08:00.000Z",
    updatedAt: "2026-06-11T15:08:00.000Z",
    sessionId: "session-webhook-bug",
    source: {
      appName: "VS Code",
      windowTitle: "server/events.ts",
      filePath: "server/events.ts",
      deviceId: "local-desktop",
      capturedVia: "clipboard-listener",
      confidence: "high"
    },
    privacy: {
      sensitive: false,
      masked: false,
      localOnly: true,
      burnAfterUse: false
    },
    content: {
      encryptedPayloadId: "payload-clip-webhook-payload",
      safePreview: "{ id: 'evt_123', type: 'checkout.session.completed' }"
    },
    transforms: [
      {
        id: "transform-webhook-json",
        kind: "fix-json",
        createdAt: "2026-06-11T15:08:30.000Z",
        safePreview: "JSON normalized."
      }
    ]
  }
];
