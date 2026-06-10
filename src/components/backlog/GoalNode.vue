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

function editGoal() {
  const title = window.prompt("Goal title", props.goal.title)?.trim();
  if (title) {
    store.updateGoal(
      props.goal.id,
      title,
      props.goal.description,
      props.goal.deadline,
      props.goal.priority,
    );
  }
}

function confirmDeleteGoal() {
  if (window.confirm(`Delete goal "${props.goal.title}" and all of its tasks?`)) {
    store.deleteGoal(props.goal.id);
  }
}

defineExpose({ localTasks, onTaskDrop });
</script>

<template>
  <details class="goal" open>
    <summary>
      <span class="drag-handle">⋮⋮</span>
      <strong :class="{ done: goal.status === 'completed' }">{{ goal.title }}</strong>
      <span class="actions">
        <button class="ghost" aria-label="Edit goal" title="Edit" @click.prevent.stop="editGoal">Edit</button>
        <button class="ghost" aria-label="Archive goal" title="Archive" @click.prevent.stop="store.archiveGoal(goal.id)">Archive</button>
        <button class="ghost danger" aria-label="Delete goal" title="Delete" @click.prevent.stop="confirmDeleteGoal">Delete</button>
      </span>
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
.actions { display: flex; gap: 8px; margin-left: auto; }
.ghost { background: none; border: none; color: #7d8796; cursor: pointer; padding: 0; font-size: 12px; }
.ghost:hover { color: #d7dde7; }
.danger:hover { color: #e06c75; }
</style>
