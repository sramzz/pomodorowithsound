import { invoke } from "@tauri-apps/api/core";

export async function ipc<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (e) {
    const err = e as { code?: string; message?: string };
    console.error(`[ipc] ${command} failed:`, err);
    // Forward to the Rust log file so the failure is visible in logs/ (spec §7).
    void invoke("log_frontend", {
      level: "error",
      message: `${command} failed: ${err.message ?? String(e)}`,
      context: args ? JSON.stringify(args) : null,
    }).catch(() => {});
    throw e;
  }
}
