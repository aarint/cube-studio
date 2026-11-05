import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { App } from "@/components/App";
import { ToastProvider } from "@/components/Toast";

import "@/styles/tokens.css";
import "@/styles/shell.css";
import "@/styles/panes.css";
import "@/styles/forms.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("root element missing from index.html");
}

/**
 * In a browser (no Tauri runtime) install the in-memory backend so the UI is
 * still fully explorable. `import.meta.env.DEV` keeps this out of production
 * bundles entirely.
 */
async function bootstrap() {
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const preview = await import("@/lib/preview");
    preview.installPreviewBackend();
  }

  createRoot(container!).render(
    <StrictMode>
      <ToastProvider>
        <App />
      </ToastProvider>
    </StrictMode>,
  );
}

void bootstrap();
