import { defineStore } from "pinia";
import { ipc } from "../ipc/client";
import type { IpcError, Project } from "../ipc/types";

export const useProjectStore = defineStore("project", {
  state: () => ({
    projects: [] as Project[],
    loading: false,
    error: null as string | null,
  }),
  actions: {
    async loadProjects(includeArchived = false) {
      this.loading = true;
      this.error = null;
      try {
        this.projects = await ipc<Project[]>("list_projects", { includeArchived });
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      } finally {
        this.loading = false;
      }
    },
  },
});
