<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import draggable from "vuedraggable";
import { useProjectStore } from "../stores/projectStore";
import { usePomodoroTypeStore } from "../stores/pomodoroTypeStore";
import GoalNode from "../components/backlog/GoalNode.vue";
import InlineCreate from "../components/backlog/InlineCreate.vue";

const store = useProjectStore();
const typeStore = usePomodoroTypeStore();

onMounted(() => {
  store.loadProjects();
  typeStore.loadTypes(); // quick estimation needs the default type's work length
});

const localGoals = ref(store.activeProjectTree?.goals ?? []);
watch(
  () => store.activeProjectTree?.goals,
  (g) => (localGoals.value = g ? [...g] : []),
);

async function onGoalDrop() {
  if (!store.activeProjectTree) return;
  await store.reorderGoals(
    store.activeProjectTree.id,
    localGoals.value.map((g) => g.id),
  );
}
</script>

<template>
  <section class="backlog">
    <aside class="projects">
      <h1>Backlog</h1>
      <p v-if="store.error" class="error">{{ store.error }}</p>
      <ul>
        <li
          v-for="p in store.projects"
          :key="p.id"
          :class="{ active: store.activeProjectTree?.id === p.id }"
          @click="store.loadProjectTree(p.id)"
        >
          <span>{{ p.name }}</span>
          <span class="stats">{{ p.completedMicrotasks }}/{{ p.totalMicrotasks }}</span>
        </li>
      </ul>
      <InlineCreate placeholder="New project" @create="(name) => store.createProject(name)" />
    </aside>

    <div class="tree">
      <p v-if="!store.activeProjectTree" class="placeholder">
        Select or create a project to manage its goals, tasks, and microtasks.
      </p>
      <template v-else>
        <h2>{{ store.activeProjectTree.name }}</h2>
        <draggable v-model="localGoals" item-key="id" handle=".drag-handle" @end="onGoalDrop">
          <template #item="{ element }">
            <GoalNode :goal="element" />
          </template>
        </draggable>
        <InlineCreate
          placeholder="New goal"
          @create="(t) => store.createGoal(store.activeProjectTree!.id, t)"
        />
      </template>
    </div>
  </section>
</template>

<style scoped>
.backlog { display: flex; gap: 24px; height: 100%; }
.projects { width: 240px; border-right: 1px solid #20242b; padding-right: 16px; }
.projects ul { list-style: none; padding: 0; margin: 12px 0; }
.projects li {
  display: flex; justify-content: space-between; padding: 8px 10px;
  border-radius: 8px; cursor: pointer; color: #c6cdd8;
}
.projects li:hover { background: #181d24; }
.projects li.active { background: #1f2630; color: #fff; }
.stats { color: #6b7484; font-size: 12px; }
.tree { flex: 1; }
.placeholder { color: #6b7484; }
.error { color: #e06c75; }
</style>
