export interface Project {
  id: string;
  name: string;
  description: string | null;
  status: "open" | "completed";
  isArchived: boolean;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface IpcError {
  code: "db" | "not_found" | "validation";
  message: string;
}

export interface ProjectSummary {
  id: string;
  name: string;
  description: string | null;
  status: "open" | "completed";
  isArchived: boolean;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
  totalMicrotasks: number;
  completedMicrotasks: number;
}

export interface TreeMicrotask {
  id: string;
  title: string;
  estimatedMinutes: number;
  pomodoroCount: number;
  pomodoroTypeId: string | null;
  deadline: string | null;
  priority: number;
  status: "open" | "completed";
}

export interface TreeTask {
  id: string;
  title: string;
  description: string | null;
  deadline: string | null;
  priority: number;
  status: "open" | "completed";
  microtasks: TreeMicrotask[];
}

export interface TreeGoal {
  id: string;
  title: string;
  description: string | null;
  deadline: string | null;
  priority: number;
  status: "open" | "completed";
  tasks: TreeTask[];
}

export interface ProjectTree {
  id: string;
  name: string;
  description: string | null;
  status: "open" | "completed";
  goals: TreeGoal[];
}

export interface PomodoroType {
  id: string;
  name: string;
  workMinutes: number;
  restMinutes: number;
  longBreakMinutes: number | null;
  longBreakEvery: number | null;
  isDefault: boolean;
}
