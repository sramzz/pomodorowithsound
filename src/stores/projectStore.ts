import { defineStore } from "pinia";
import { ipc } from "../ipc/client";
import type { IpcError, ProjectSummary, ProjectTree } from "../ipc/types";

export const useProjectStore = defineStore("project", {
  state: () => ({
    projects: [] as ProjectSummary[],
    activeProjectTree: null as ProjectTree | null,
    loading: false,
    error: null as string | null,
  }),
  actions: {
    async loadProjects(includeArchived = false, preserveError = false) {
      this.loading = true;
      if (!preserveError) this.error = null;
      try {
        this.projects = await ipc<ProjectSummary[]>("list_projects", { includeArchived });
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      } finally {
        this.loading = false;
      }
    },
    async loadProjectTree(projectId: string) {
      this.error = null;
      try {
        this.activeProjectTree = await ipc<ProjectTree>("get_project_tree", { projectId });
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      }
    },
    // CQS: mutate, then re-query the tree (and the stats list — counts change).
    async mutate(cmd: string, args: Record<string, unknown>) {
      this.error = null;
      try {
        await ipc<void>(cmd, args);
        if (this.activeProjectTree) await this.loadProjectTree(this.activeProjectTree.id);
        await this.loadProjects(false, true);
      } catch (e) {
        this.error = (e as IpcError).message ?? String(e);
      }
    },

    // ---- projects ----
    async createProject(name: string, description: string | null = null) {
      await this.mutate("create_project", { id: crypto.randomUUID(), name, description });
    },
    async updateProject(id: string, name: string, description: string | null) {
      await this.mutate("update_project", { id, name, description });
    },
    async archiveProject(id: string) {
      await this.mutate("archive_project", { id });
    },
    async deleteProject(id: string) {
      if (this.activeProjectTree?.id === id) this.activeProjectTree = null;
      await this.mutate("delete_project", { id });
    },

    // ---- goals ----
    async createGoal(projectId: string, title: string) {
      await this.mutate("create_goal", {
        id: crypto.randomUUID(), projectId, title,
        description: null, deadline: null, priority: 0,
      });
    },
    async updateGoal(id: string, title: string, description: string | null, deadline: string | null, priority: number) {
      await this.mutate("update_goal", { id, title, description, deadline, priority });
    },
    async archiveGoal(id: string) {
      await this.mutate("archive_goal", { id });
    },
    async deleteGoal(id: string) {
      await this.mutate("delete_goal", { id });
    },
    async reorderGoals(projectId: string, orderedIds: string[]) {
      await this.mutate("reorder_goals", { projectId, orderedIds });
    },

    // ---- tasks ----
    async createTask(goalId: string, title: string) {
      await this.mutate("create_task", {
        id: crypto.randomUUID(), goalId, title,
        description: null, deadline: null, priority: 0,
      });
    },
    async updateTask(id: string, title: string, description: string | null, deadline: string | null, priority: number) {
      await this.mutate("update_task", { id, title, description, deadline, priority });
    },
    async archiveTask(id: string) {
      await this.mutate("archive_task", { id });
    },
    async deleteTask(id: string) {
      await this.mutate("delete_task", { id });
    },
    async reorderTasks(goalId: string, orderedIds: string[]) {
      await this.mutate("reorder_tasks", { goalId, orderedIds });
    },

    // ---- microtasks ----
    async createMicrotask(taskId: string, title: string, estimatedMinutes: number, pomodoroCount: number, pomodoroTypeId: string | null) {
      await this.mutate("create_microtask", {
        id: crypto.randomUUID(), taskId, title,
        estimatedMinutes, pomodoroCount, pomodoroTypeId,
        deadline: null, priority: 0,
      });
    },
    async updateMicrotask(id: string, title: string, estimatedMinutes: number, pomodoroCount: number, pomodoroTypeId: string | null, deadline: string | null, priority: number) {
      await this.mutate("update_microtask", { id, title, estimatedMinutes, pomodoroCount, pomodoroTypeId, deadline, priority });
    },
    async completeMicrotask(id: string) {
      await this.mutate("complete_microtask", { id });
    },
    async uncompleteMicrotask(id: string) {
      await this.mutate("uncomplete_microtask", { id });
    },
    async archiveMicrotask(id: string) {
      await this.mutate("archive_microtask", { id });
    },
    async deleteMicrotask(id: string) {
      await this.mutate("delete_microtask", { id });
    },
    async reorderMicrotasks(taskId: string, orderedIds: string[]) {
      await this.mutate("reorder_microtasks", { taskId, orderedIds });
    },
  },
});
