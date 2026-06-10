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
    <button class="ghost" title="Archive" @click="store.archiveMicrotask(microtask.id)">⌫</button>
  </div>
</template>

<style scoped>
.microtask { display: flex; align-items: center; gap: 8px; padding: 4px 0; }
.microtask.done .title { text-decoration: line-through; color: #6b7484; }
.meta { color: #9aa3b2; font-size: 12px; margin-left: auto; }
.drag-handle { cursor: grab; color: #4a5260; user-select: none; }
.ghost { background: none; border: none; color: #6b7484; cursor: pointer; }
</style>
