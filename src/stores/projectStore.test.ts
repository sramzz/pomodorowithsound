import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockIPC } from "@tauri-apps/api/mocks";
import { useProjectStore } from "./projectStore";

describe("useProjectStore", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loadProjects fills state from the list_projects command", async () => {
    mockIPC((cmd, args) => {
      if (cmd === "list_projects") {
        expect((args as Record<string, unknown>).includeArchived).toBe(false);
        return [
          {
            id: "p1", name: "My project", description: null, status: "open",
            isArchived: false, completedAt: null,
            createdAt: "2026-06-09T08:00:00Z", updatedAt: "2026-06-09T08:00:00Z",
          },
        ];
      }
    });

    const store = useProjectStore();
    await store.loadProjects();
    expect(store.projects).toHaveLength(1);
    expect(store.projects[0].name).toBe("My project");
    expect(store.error).toBeNull();
  });

  it("loadProjects records the error message on failure", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_projects") throw { code: "db", message: "boom" };
    });

    const store = useProjectStore();
    await store.loadProjects();
    expect(store.projects).toHaveLength(0);
    expect(store.error).toBe("boom");
  });
});
