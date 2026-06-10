import { beforeEach, describe, expect, it } from "vitest";
import { createPinia, setActivePinia } from "pinia";
import { mockIPC } from "@tauri-apps/api/mocks";
import { usePomodoroTypeStore } from "./pomodoroTypeStore";

const standard = {
  id: "pt-std", name: "Standard", workMinutes: 20, restMinutes: 5,
  longBreakMinutes: null, longBreakEvery: null, isDefault: true,
};

describe("usePomodoroTypeStore", () => {
  beforeEach(() => setActivePinia(createPinia()));

  it("loadTypes fills state and exposes the default type", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_pomodoro_types") return [standard];
    });
    const store = usePomodoroTypeStore();
    await store.loadTypes();
    expect(store.types).toHaveLength(1);
    expect(store.defaultType?.id).toBe("pt-std");
  });

  it("createType sends a generated id and re-queries (CQS)", async () => {
    const calls: string[] = [];
    mockIPC((cmd, args) => {
      calls.push(cmd);
      if (cmd === "create_pomodoro_type") {
        const a = args as Record<string, unknown>;
        expect(typeof a.id).toBe("string");
        expect((a.id as string).length).toBeGreaterThan(10);
        expect(a.name).toBe("Deep");
        expect(a.workMinutes).toBe(50);
        return null;
      }
      if (cmd === "list_pomodoro_types") return [standard];
    });
    const store = usePomodoroTypeStore();
    await store.createType({ name: "Deep", workMinutes: 50, restMinutes: 10, longBreakMinutes: null, longBreakEvery: null });
    expect(calls).toContain("create_pomodoro_type");
    expect(calls.filter((c) => c === "list_pomodoro_types")).toHaveLength(1);
  });

  it("records the error message when a mutation is rejected", async () => {
    mockIPC((cmd) => {
      if (cmd === "set_default_pomodoro_type") throw { code: "not_found", message: "pomodoro_type not found: ghost" };
      if (cmd === "list_pomodoro_types") return [standard];
    });
    const store = usePomodoroTypeStore();
    await store.setDefault("ghost");
    expect(store.error).toContain("not found");
  });
});
