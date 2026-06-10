<script setup lang="ts">
import type { TreeMicrotask } from "../../ipc/types";
import { useProjectStore } from "../../stores/projectStore";

const props = defineProps<{ microtask: TreeMicrotask }>();
const store = useProjectStore();

function toggle() {
  if (props.microtask.status === "completed") {
    store.uncompleteMicrotask(props.microtask.id);
  } else {
    store.completeMicrotask(props.microtask.id);
  }
}

function editMicrotask() {
  const title = window.prompt("Microtask title", props.microtask.title)?.trim();
  if (!title) return;
  const estimated = Number(window.prompt("Estimated minutes", String(props.microtask.estimatedMinutes)));
  if (!Number.isInteger(estimated) || estimated < 1) return;
  const count = Number(window.prompt("Pomodoro count", String(props.microtask.pomodoroCount)));
  if (!Number.isInteger(count) || count < 1) return;
  store.updateMicrotask(
    props.microtask.id,
    title,
    estimated,
    count,
    props.microtask.pomodoroTypeId,
    props.microtask.deadline,
    props.microtask.priority,
  );
}

function confirmDeleteMicrotask() {
  if (window.confirm(`Delete microtask "${props.microtask.title}"?`)) {
    store.deleteMicrotask(props.microtask.id);
  }
}
</script>

<template>
  <div class="microtask" :class="{ done: microtask.status === 'completed' }">
    <span class="drag-handle">⋮⋮</span>
    <input
      type="checkbox"
      :checked="microtask.status === 'completed'"
      @change="toggle"
    />
    <span class="title">{{ microtask.title }}</span>
    <span class="meta">{{ microtask.estimatedMinutes }}m · {{ microtask.pomodoroCount }}🍅</span>
    <span class="actions">
      <button class="ghost" aria-label="Edit microtask" title="Edit" @click="editMicrotask">Edit</button>
      <button class="ghost" aria-label="Archive microtask" title="Archive" @click="store.archiveMicrotask(microtask.id)">Archive</button>
      <button class="ghost danger" aria-label="Delete microtask" title="Delete" @click="confirmDeleteMicrotask">Delete</button>
    </span>
  </div>
</template>

<style scoped>
.microtask { display: flex; align-items: center; gap: 8px; padding: 4px 0; }
.microtask.done .title { text-decoration: line-through; color: #6b7484; }
.meta { color: #9aa3b2; font-size: 12px; margin-left: auto; }
.drag-handle { cursor: grab; color: #4a5260; user-select: none; }
.actions { display: flex; gap: 8px; }
.ghost { background: none; border: none; color: #7d8796; cursor: pointer; padding: 0; font-size: 12px; }
.ghost:hover { color: #d7dde7; }
.danger:hover { color: #e06c75; }
</style>
