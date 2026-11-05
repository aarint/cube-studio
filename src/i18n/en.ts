/**
 * English catalogue — the source of truth for every key.
 *
 * Rules for this file:
 *  - Add keys here first; `Messages` below is derived from it, so every other
 *    catalogue that misses a key fails type checking.
 *  - Write full sentences, not fragments, so translators have context.
 *  - Use `{name}` placeholders for interpolation and never concatenate strings
 *    in components: word order differs between languages.
 */

export const en = {
  // ---- app shell ------------------------------------------------------
  "app.name": "Cube Studio",
  "app.tagline": "Everything you cache and stream",

  // ---- language picker -------------------------------------------------
  "language.label": "Language",
  "language.choose": "Choose interface language",

  // ---- connections rail ------------------------------------------------
  "rail.open": "Open",
  "rail.noSessions": "No open connections.",
  "rail.newConnection": "New connection",
  "rail.closeConnection": "Close {name}",
  "rail.refresh": "Refresh",
  "rail.sessions": "{count} session",
  "rail.sessions_plural": "{count} sessions",

  // ---- status bar ------------------------------------------------------
  "status.connected": "connected",
  "status.idle": "idle",

  // ---- welcome ---------------------------------------------------------
  "welcome.title": "Everything you cache and stream",
  "welcome.lede":
    "Cube Studio inspects caches, queues and event logs from one place. Pick a middleware to get started, or reopen a saved connection.",
  "welcome.saved": "Saved connections",
  "welcome.empty.title": "No saved connections yet",
  "welcome.empty.body":
    "Connections you save are encrypted with a key bound to this machine and user account. Nothing is sent anywhere else.",
  "welcome.empty.cta": "Add your first connection",
  "welcome.open": "Open",
  "welcome.opening": "Opening…",
  "welcome.edit": "Edit {name}",
  "welcome.remove": "Remove {name}",
  "welcome.loadFailed": "Could not load saved connections",
  "welcome.connectFailed": "Could not connect",
  "welcome.removeFailed": "Could not remove",
  "welcome.removed": "Connection removed",

  // ---- connection dialog -----------------------------------------------
  "dialog.new": "New connection",
  "dialog.edit": "Edit connection",
  "dialog.close": "Close dialog",
  "dialog.middleware": "Middleware",
  "dialog.endpoint": "Endpoint",
  "dialog.auth": "Authentication",
  "dialog.redis": "Redis",
  "dialog.mqtt": "MQTT",
  "dialog.tls": "Transport security",
  "dialog.name": "Name",
  "dialog.host": "Host",
  "dialog.port": "Port",
  "dialog.username": "Username",
  "dialog.password": "Password",
  "dialog.saslUsername": "SASL username",
  "dialog.saslPassword": "SASL password",
  "dialog.token": "Token",
  "dialog.tokenHint":
    "Used instead of username and password when the server is configured for token auth.",
  "dialog.vhost": "Virtual host",
  "dialog.saslMechanism": "SASL mechanism",
  "dialog.deployment": "Deployment",
  "dialog.deployment.standalone": "Standalone",
  "dialog.deployment.cluster": "Cluster",
  "dialog.deployment.sentinel": "Sentinel",
  "dialog.masterName": "Master name",
  "dialog.databaseIndex": "Database index",
  "dialog.extraNodes": "Additional nodes",
  "dialog.clientId": "Client id",
  "dialog.clientIdHint": "generated if empty",
  "dialog.keepAlive": "Keep-alive (s)",
  "dialog.tlsMode": "TLS",
  "dialog.tls.disabled": "Disabled",
  "dialog.tls.enabled": "Enabled",
  "dialog.tls.insecure": "Enabled, no verification",
  "dialog.caBundle": "CA bundle",
  "dialog.caBundleHint": "path to PEM (optional)",
  "dialog.tlsWarn":
    "Certificates will not be verified. Any host on the network can impersonate this server, so use this only against a throwaway instance.",
  "dialog.advanced": "Show advanced",
  "dialog.advancedHide": "Hide advanced",
  "dialog.timeout": "Timeout (s)",
  "dialog.payloadLimit": "Payload limit (KB)",
  "dialog.payloadLimitHint":
    "Larger values are truncated before being sent to the window.",
  "dialog.remember": "Remember this connection",
  "dialog.rememberHint": "Stored encrypted in your app data directory",
  "dialog.test": "Test",
  "dialog.testing": "Testing…",
  "dialog.connect": "Connect",
  "dialog.connecting": "Connecting…",
  "dialog.reconnect": "Reconnect",
  "dialog.verified": "Connection verified",
  "dialog.verifiedServer": "server: {version}",
  "dialog.verifiedReachable": "server reachable",
  "dialog.failed": "Connection failed",
  "dialog.checkSettings": "Check the connection settings",
  "dialog.error.host": "Host is required.",
  "dialog.error.port": "Port must be between 1 and 65535.",
  "dialog.error.sentinel": "Sentinel mode needs the master name to resolve.",
  "dialog.error.sasl": "SASL {mechanism} needs a username.",
  "dialog.connected": "Connected",

  // ---- cloud -----------------------------------------------------------
  "cloud.tab.local": "Self-hosted",
  "cloud.tab.cloud": "Cloud",
  "cloud.provider": "Provider",
  "cloud.service": "Managed service",
  "cloud.credentials": "Cloud credentials",
  "cloud.credentialsHint":
    "Sent to the provider's API only. Never stored on disk.",
  "cloud.getCredentials": "Where do I find these?",
  "cloud.discover": "Discover instances",
  "cloud.discovering": "Discovering…",
  "cloud.results": "Instances",
  "cloud.noResults": "No instances found in this account.",
  "cloud.noResultsHint":
    "Check the region or project, and that the credentials have list permission.",
  "cloud.discovered": "Found {count} instance",
  "cloud.discovered_plural": "Found {count} instances",
  "cloud.discoverFailed": "Could not list cloud instances",
  "cloud.accessKeyId": "Access key id",
  "cloud.secretAccessKey": "Secret access key",
  "cloud.sessionToken": "Session token (optional)",
  "cloud.region": "Region",
  "cloud.tenantId": "Tenant id",
  "cloud.clientId": "Client id",
  "cloud.clientSecret": "Client secret",
  "cloud.subscriptionId": "Subscription id",
  "cloud.accessToken": "Access token",
  "cloud.accessTokenHint":
    "Run `gcloud auth print-access-token` and paste the result.",
  "cloud.projectId": "Project id",
  "cloud.apiKey": "API key",
  "cloud.apiSecret": "API secret / account email",
  "cloud.connect": "Connect",
  "cloud.connecting": "Connecting…",
  "cloud.dataPassword": "Data-plane password",
  "cloud.dataPasswordHint":
    "The password or AUTH token for the instance itself, not the cloud API.",
  "cloud.requiresPassword": "needs a password",
  "cloud.connectFailed": "Could not connect to the cloud instance",
  "cloud.region.placeholder": "us-east-1",
  "cloud.endpoint": "endpoint",
  "cloud.tls": "TLS",
  "cloud.on": "on",
  "cloud.off": "off",

  // ---- workspace -------------------------------------------------------
  "workspace.panes.databases": "Databases",
  "workspace.panes.topics": "Topics",
  "workspace.panes.queues": "Queues",
  "workspace.panes.streams": "Streams",
  "workspace.panes.filters": "Filters",
  "workspace.panes.server": "Server",
  "workspace.entries": "Entries",
  "workspace.live": "Live",
  "workspace.console": "Console",
  "workspace.stats": "Stats",
  "workspace.browse": "Browse",
  "workspace.filter": "Filter…",
  "workspace.filterEntries": "Filter entries",
  "workspace.refreshEntries": "Refresh entries",
  "workspace.refreshContainers": "Refresh containers",
  "workspace.loadMore": "Load more",
  "workspace.loading": "Loading…",
  "workspace.noEntries": "No entries match.",
  "workspace.selectContainer": "Select a container first.",
  "workspace.cannotEnumerate":
    "This backend cannot enumerate entries. Use the live view.",
  "workspace.flush": "Flush {name}",
  "workspace.flushFallback": "container",
  "workspace.flushed": "Flushed",
  "workspace.flushFailed": "Could not flush",
  "workspace.listContainersFailed": "Could not list containers",
  "workspace.listEntriesFailed": "Could not list entries",
  "workspace.readFailed": "Could not read entry",

  // ---- detail ----------------------------------------------------------
  "detail.nothing": "Nothing selected",
  "detail.nothingHint": "Pick an entry to inspect its payload.",
  "detail.copy": "Copy payload",
  "detail.copied": "Copied to clipboard",
  "detail.clipboardUnavailable": "Clipboard unavailable",
  "detail.edit": "Edit payload",
  "detail.delete": "Delete entry",
  "detail.save": "Save",
  "detail.saved": "Saved",
  "detail.cancel": "Cancel",
  "detail.saveFailed": "Could not save",
  "detail.deleted": "Deleted",
  "detail.deleteFailed": "Could not delete",
  "detail.invalidJson": "Invalid JSON",
  "detail.invalidJsonHint": "Fix the brackets before saving.",
  "detail.editorHint": "⌘S to save · Esc to cancel",
  "detail.truncated":
    "Showing {size} of a truncated value. Raise the payload limit in the connection settings to see all of it.",
  "detail.size": "size",
  "detail.ttl": "ttl",
  "detail.json": "JSON",
  "detail.raw": "Raw",

  // ---- publish ---------------------------------------------------------
  "publish.action": "Publish",
  "publish.title": "Publish message",
  "publish.partition": "Partition",
  "publish.subject": "Subject",
  "publish.topic": "Topic",
  "publish.routingKey": "Routing key",
  "publish.qos": "QoS",
  "publish.qos.atMostOnce": "0 — at most once",
  "publish.qos.atLeastOnce": "1 — at least once",
  "publish.qos.exactlyOnce": "2 — exactly once",
  "publish.retain": "Retain on the broker",
  "publish.retainHint": "New subscribers receive this message immediately",
  "publish.payload": "Payload",
  "publish.sent": "Published",
  "publish.failed": "Publish failed",

  // ---- live tail -------------------------------------------------------
  "tail.target": "Subscription target",
  "tail.subscribe": "Subscribe",
  "tail.stop": "Stop",
  "tail.pause": "Pause",
  "tail.resume": "Resume ({count})",
  "tail.clear": "Clear messages",
  "tail.live": "live · {count} received",
  "tail.waiting": "Waiting for messages on this subscription…",
  "tail.idle": "Subscribe to watch messages arrive in real time.",
  "tail.ended": "Stream ended",
  "tail.endedDetail": "{count} message received",
  "tail.endedDetail_plural": "{count} messages received",
  "tail.subscribeFailed": "Could not subscribe",
  "tail.binary": "(binary)",

  // ---- console ---------------------------------------------------------
  "console.placeholder.redis": "GET some:key",
  "console.placeholder.generic": "subject  payload",
  "console.placeholder.rabbit": "queue_declare orders",
  "console.placeholder.memcached": "get some:key",
  "console.empty": "Run a command against this session.",
  "console.run": "Run",
  "console.running": "Running…",
  "console.command": "Command",

  // ---- stats -----------------------------------------------------------
  "stats.values": "{count} values",
  "stats.refresh": "Refresh",
  "stats.failed": "Could not read stats",

  // ---- toast -----------------------------------------------------------
  "toast.dismiss": "Dismiss notification",

  // ---- backend notes (shown in the UI) ---------------------------------
  "note.memcached.noEnum":
    "Memcached has no key enumeration command. Keys are collected from server statistics, so only resident keys appear here.",
  "note.kafka.appendOnly":
    "Kafka topics are append-only: messages cannot be edited or deleted individually.",
  "note.rabbit.publish":
    "Publishing goes through an exchange with a routing key; the queue view shows messages as the broker hands them out.",
  "note.nats.core":
    "Core NATS is fire-and-forget: there is no history to list. Use the tail panel to watch a subject live.",
  "note.nats.jetstreamOff":
    "This broker has JetStream disabled, so there is no message history.",

  // ---- errors ----------------------------------------------------------
  "error.network": "Cannot reach the server",
  "error.unauthorized": "Authentication failed",
  "error.timeout": "The request timed out",
  "error.unsupported": "Not supported by this server",
  "error.unavailable": "Temporarily unavailable — try again",
  "error.invalid": "Check the input",
  "error.internal": "Something went wrong",
} as const;

/** Every key the app knows about. Derived from the English catalogue. */
export type MessageKey = keyof typeof en;

/** A complete catalogue. `Record` makes a missing key a type error. */
export type Catalogue = Record<MessageKey, string>;
