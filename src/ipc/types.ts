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
