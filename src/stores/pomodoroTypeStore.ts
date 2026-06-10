import { defineStore } from "pinia";
import { ipc } from "../ipc/client";
import type { IpcError, PomodoroType } from "../ipc/types";

export interface PomodoroTypeDraft {
  name: string;
  workMinutes: number;
  restMinutes: number;
  longBreakMinutes: number | null;
  longBreakEvery: number | null;
}

export const usePomodoroTypeStore = defineStore("pomodoroType", {
  state: () => ({
    types: [] as PomodoroType[],
    loading: false,
    error: null as string | null,
  }),
  getters: {
    defaultType: (s) => s.types.find((t) => t.isDefault) ?? null,
  },
  actions: {
    async loadTypes() {
      this.loading = true;
      this.error = null;
      try {
        this.types = await ipc<PomodoroType[]>("list_pomodoro_types");
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      } finally {
        this.loading = false;
      }
    },
    // CQS: every mutation returns nothing; state refreshes by re-querying.
    async mutate(cmd: string, args: Record<string, unknown>) {
      this.error = null;
      try {
        await ipc<void>(cmd, args);
        await this.loadTypes();
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      }
    },
    async createType(draft: PomodoroTypeDraft) {
      await this.mutate("create_pomodoro_type", { id: crypto.randomUUID(), ...draft });
    },
    async updateType(id: string, draft: PomodoroTypeDraft) {
      await this.mutate("update_pomodoro_type", { id, ...draft });
    },
    async deleteType(id: string) {
      await this.mutate("delete_pomodoro_type", { id });
    },
    async setDefault(id: string) {
      await this.mutate("set_default_pomodoro_type", { id });
    },
  },
});
