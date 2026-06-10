<script setup lang="ts">
import { ref, watch } from "vue";
import draggable from "vuedraggable";
import type { TreeTask } from "../../ipc/types";
import { useProjectStore } from "../../stores/projectStore";
import { usePomodoroTypeStore } from "../../stores/pomodoroTypeStore";
import { computePomodoroCount } from "../../lib/estimation";
import MicrotaskRow from "./MicrotaskRow.vue";
import InlineCreate from "./InlineCreate.vue";

const props = defineProps<{ task: TreeTask }>();
const store = useProjectStore();
const typeStore = usePomodoroTypeStore();

const localMicrotasks = ref([...props.task.microtasks]);
watch(() => props.task.microtasks, (m) => (localMicrotasks.value = [...m]));

async function onMicrotaskDrop() {
  await store.reorderMicrotasks(
    props.task.id,
    localMicrotasks.value.map((m) => m.id),
  );
}

// Quick estimation: "Outline the doc 45" -> 45 estimated minutes, count auto-computed
// from the default pomodoro type's work length (spec §6).
async function createMicrotask(text: string) {
  const match = text.match(/^(.*?)\s+(\d+)$/);
  const title = match ? match[1] : text;
  const estimated = match ? Number(match[2]) : 20;
  const workMinutes = typeStore.defaultType?.workMinutes ?? null;
  await store.createMicrotask(
    props.task.id,
    title,
    estimated,
    computePomodoroCount(estimated, workMinutes),
    null,
  );
}

defineExpose({ localMicrotasks, onMicrotaskDrop });
</script>

<template>
  <details class="task" open>
    <summary>
      <span class="drag-handle">⋮⋮</span>
      <span :class="{ done: task.status === 'completed' }">{{ task.title }}</span>
      <button class="ghost" title="Archive" @click.prevent="store.archiveTask(task.id)">⌫</button>
    </summary>
    <draggable
      v-model="localMicrotasks"
      item-key="id"
      handle=".drag-handle"
      @end="onMicrotaskDrop"
    >
      <template #item="{ element }">
        <MicrotaskRow :microtask="element" />
      </template>
    </draggable>
    <InlineCreate
      placeholder="New microtask — append minutes to estimate, e.g. “Outline 45”"
      @create="createMicrotask"
    />
  </details>
</template>

<style scoped>
.task { margin-left: 16px; padding: 2px 0; }
summary { display: flex; align-items: center; gap: 8px; cursor: pointer; list-style: none; }
.done { text-decoration: line-through; color: #6b7484; }
.drag-handle { cursor: grab; color: #4a5260; user-select: none; }
.ghost { background: none; border: none; color: #6b7484; cursor: pointer; margin-left: auto; }
</style>
