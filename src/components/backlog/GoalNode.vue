<script setup lang="ts">
import { ref, watch } from "vue";
import draggable from "vuedraggable";
import type { TreeGoal } from "../../ipc/types";
import { useProjectStore } from "../../stores/projectStore";
import TaskNode from "./TaskNode.vue";
import InlineCreate from "./InlineCreate.vue";

const props = defineProps<{ goal: TreeGoal }>();
const store = useProjectStore();

const localTasks = ref([...props.goal.tasks]);
watch(() => props.goal.tasks, (t) => (localTasks.value = [...t]));

async function onTaskDrop() {
  await store.reorderTasks(props.goal.id, localTasks.value.map((t) => t.id));
}

defineExpose({ localTasks, onTaskDrop });
</script>

<template>
  <details class="goal" open>
    <summary>
      <span class="drag-handle">⋮⋮</span>
      <strong :class="{ done: goal.status === 'completed' }">{{ goal.title }}</strong>
      <button class="ghost" title="Archive" @click.prevent="store.archiveGoal(goal.id)">⌫</button>
    </summary>
    <draggable v-model="localTasks" item-key="id" handle=".drag-handle" @end="onTaskDrop">
      <template #item="{ element }">
        <TaskNode :task="element" />
      </template>
    </draggable>
    <InlineCreate placeholder="New task" @create="(t) => store.createTask(goal.id, t)" />
  </details>
</template>

<style scoped>
.goal { margin-left: 8px; padding: 4px 0; }
summary { display: flex; align-items: center; gap: 8px; cursor: pointer; list-style: none; }
.done { text-decoration: line-through; color: #6b7484; }
.drag-handle { cursor: grab; color: #4a5260; user-select: none; }
.ghost { background: none; border: none; color: #6b7484; cursor: pointer; margin-left: auto; }
</style>
