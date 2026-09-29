// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The suffix is computed once per module instance from location.pathname, so
// every case resets modules and re-imports against the pathname it installs.
async function importSuffix(): Promise<() => string> {
  const module = await import("@/lib/backend/workspaceStorageScope");
  return module.browserStorageScopeSuffix;
}

function setPathname(pathname: string) {
  window.history.pushState({}, "", pathname);
}

beforeEach(() => {
  vi.resetModules();
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("browserStorageScopeSuffix", () => {
  it("is empty for a root deployment", async () => {
    setPathname("/");
    expect((await importSuffix())()).toBe("");
  });

  it("namespaces a reverse-proxy mount path, tolerating trailing slashes", async () => {
    setPathname("/api/v1/userapp/proxy/dbx/dev/1/197/");
    expect((await importSuffix())()).toBe(":api-v1-userapp-proxy-dbx-dev-1-197");

    vi.resetModules();
    setPathname("/api/v1/userapp/proxy/dbx/dev/1/197");
    expect((await importSuffix())()).toBe(":api-v1-userapp-proxy-dbx-dev-1-197");
  });

  it("keeps the workspace stable across the /login auth redirect", async () => {
    setPathname("/api/v1/userapp/proxy/dbx/prod/1/197/login");
    expect((await importSuffix())()).toBe(":api-v1-userapp-proxy-dbx-prod-1-197");
  });

  it("distinguishes environments, users and apps on the same origin", async () => {
    const suffixes: string[] = [];
    for (const pathname of ["/api/v1/userapp/proxy/dbx/dev/1/197", "/api/v1/userapp/proxy/dbx/prod/1/197", "/api/v1/userapp/proxy/dbx/dev/1/198"]) {
      vi.resetModules();
      setPathname(pathname);
      suffixes.push((await importSuffix())());
    }
    expect(new Set(suffixes).size).toBe(3);
  });

  it("sanitizes separators and preserves safe characters", async () => {
    setPathname("/ws/a//b");
    expect((await importSuffix())()).toBe(":ws-a-b");

    vi.resetModules();
    setPathname("/x/y_1.z-2");
    expect((await importSuffix())()).toBe(":x-y_1.z-2");
  });

  it("caches the suffix for the document lifetime", async () => {
    setPathname("/ws/dev");
    const suffix = await importSuffix();
    expect(suffix()).toBe(":ws-dev");
    setPathname("/ws/prod");
    expect(suffix()).toBe(":ws-dev");
  });
});

describe("browser app state storage under a mount path", () => {
  function installFailingIndexedDb(openedNames: string[]) {
    vi.stubGlobal("indexedDB", {
      open(name: string) {
        openedNames.push(name);
        const request: Record<string, any> = { onsuccess: null, onerror: null, onblocked: null, onupgradeneeded: null };
        queueMicrotask(() => request.onerror?.(new Event("error")));
        return request as IDBOpenDBRequest;
      },
    });
  }

  function installLocalStorageCapture(captured: Map<string, string>) {
    vi.stubGlobal("localStorage", {
      getItem: vi.fn((key: string) => captured.get(key) ?? null),
      setItem: vi.fn((key: string, value: string) => captured.set(key, value)),
      removeItem: vi.fn((key: string) => captured.delete(key)),
    });
  }

  it("opens a suffixed database and writes the suffixed localStorage fallback", async () => {
    setPathname("/api/v1/userapp/proxy/dbx/dev/1/197/");
    const openedNames: string[] = [];
    const captured = new Map<string, string>();
    installFailingIndexedDb(openedNames);
    installLocalStorageCapture(captured);

    const { saveBrowserAppState } = await import("@/lib/backend/browserAppStateStorage");
    await saveBrowserAppState("open_tabs", { tabs: [], activeTabId: null });

    expect(openedNames).toEqual(["dbx-app-state:api-v1-userapp-proxy-dbx-dev-1-197"]);
    expect([...captured.keys()]).toEqual(["dbx-app-state:api-v1-userapp-proxy-dbx-dev-1-197:open_tabs"]);
  });

  it("keeps official storage names byte-identical for a root deployment", async () => {
    setPathname("/");
    const openedNames: string[] = [];
    const captured = new Map<string, string>();
    installFailingIndexedDb(openedNames);
    installLocalStorageCapture(captured);

    const { saveBrowserAppState } = await import("@/lib/backend/browserAppStateStorage");
    await saveBrowserAppState("open_tabs", { tabs: [], activeTabId: null });

    expect(openedNames).toEqual(["dbx-app-state"]);
    expect([...captured.keys()]).toEqual(["dbx-app-state:open_tabs"]);
  });
});
