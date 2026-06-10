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

const emptyTree = {
  id: "p1", name: "My project", description: null, status: "open", goals: [],
};

it("loadProjectTree fills activeProjectTree", async () => {
  mockIPC((cmd, args) => {
    if (cmd === "get_project_tree") {
      expect((args as Record<string, unknown>).projectId).toBe("p1");
      return emptyTree;
    }
  });
  const store = useProjectStore();
  await store.loadProjectTree("p1");
  expect(store.activeProjectTree?.id).toBe("p1");
});

it("createGoal generates an id, then refreshes the active tree (CQS)", async () => {
  const calls: string[] = [];
  mockIPC((cmd, args) => {
    calls.push(cmd);
    if (cmd === "create_goal") {
      const a = args as Record<string, unknown>;
      expect(typeof a.id).toBe("string");
      expect(a.projectId).toBe("p1");
      expect(a.title).toBe("Ship it");
      return null;
    }
    if (cmd === "get_project_tree") return emptyTree;
    if (cmd === "list_projects") return [];
  });
  const store = useProjectStore();
  await store.loadProjectTree("p1");
  await store.createGoal("p1", "Ship it");
  expect(calls).toEqual(
    expect.arrayContaining(["create_goal", "get_project_tree"]),
  );
});

it("completeMicrotask refreshes both the tree and the project stats", async () => {
  const calls: string[] = [];
  mockIPC((cmd) => {
    calls.push(cmd);
    if (cmd === "complete_microtask") return null;
    if (cmd === "get_project_tree") return emptyTree;
    if (cmd === "list_projects") return [];
  });
  const store = useProjectStore();
  await store.loadProjectTree("p1");
  await store.completeMicrotask("m1");
  expect(calls).toContain("complete_microtask");
  expect(calls.filter((c) => c === "get_project_tree").length).toBeGreaterThanOrEqual(2);
  expect(calls).toContain("list_projects");
});
