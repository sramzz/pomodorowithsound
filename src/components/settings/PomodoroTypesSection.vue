<script setup lang="ts">
import { onMounted, reactive } from "vue";
import type { PomodoroType } from "../../ipc/types";
import { usePomodoroTypeStore } from "../../stores/pomodoroTypeStore";

const store = usePomodoroTypeStore();
onMounted(() => store.loadTypes());

const draft = reactive({
  name: "",
  workMinutes: 25,
  restMinutes: 5,
  longBreakMinutes: null as number | null,
  longBreakEvery: null as number | null,
});

async function create() {
  if (!draft.name.trim()) return;
  await store.createType({ ...draft, name: draft.name.trim() });
  if (!store.error) draft.name = "";
}

function confirmDelete(id: string, name: string) {
  if (window.confirm(`Delete the "${name}" preset? Microtasks using it fall back to the default type.`)) {
    store.deleteType(id);
  }
}

function editType(type: PomodoroType) {
  const name = window.prompt("Preset name", type.name)?.trim();
  if (!name) return;
  const workMinutes = Number(window.prompt("Work minutes", String(type.workMinutes)));
  const restMinutes = Number(window.prompt("Rest minutes", String(type.restMinutes)));
  if (!Number.isInteger(workMinutes) || workMinutes < 1) return;
  if (!Number.isInteger(restMinutes) || restMinutes < 1) return;

  const longBreakMinutesInput = window.prompt(
    "Long break minutes (blank for none)",
    type.longBreakMinutes?.toString() ?? "",
  );
  if (longBreakMinutesInput === null) return;
  const longBreakEveryInput = window.prompt(
    "Long break every N pomodoros (blank for none)",
    type.longBreakEvery?.toString() ?? "",
  );
  if (longBreakEveryInput === null) return;

  store.updateType(type.id, {
    name,
    workMinutes,
    restMinutes,
    longBreakMinutes: longBreakMinutesInput === "" ? null : Number(longBreakMinutesInput),
    longBreakEvery: longBreakEveryInput === "" ? null : Number(longBreakEveryInput),
  });
}
</script>

<template>
  <section class="presets">
    <h2>Pomodoro types</h2>
    <p v-if="store.error" class="error">{{ store.error }}</p>

    <table>
      <thead>
        <tr><th>Default</th><th>Name</th><th>Work</th><th>Rest</th><th>Long break</th><th></th></tr>
      </thead>
      <tbody>
        <tr v-for="t in store.types" :key="t.id">
          <td>
            <input
              type="radio"
              name="default-type"
              :checked="t.isDefault"
              @change="store.setDefault(t.id)"
            />
          </td>
          <td>{{ t.name }}</td>
          <td>{{ t.workMinutes }}m</td>
          <td>{{ t.restMinutes }}m</td>
          <td>
            <template v-if="t.longBreakMinutes">{{ t.longBreakMinutes }}m every {{ t.longBreakEvery }}</template>
            <template v-else>—</template>
          </td>
          <td class="actions">
            <button class="ghost" :aria-label="`Edit ${t.name}`" @click="editType(t)">Edit</button>
            <button class="ghost danger" :aria-label="`Delete ${t.name}`" @click="confirmDelete(t.id, t.name)">Delete</button>
          </td>
        </tr>
      </tbody>
    </table>

    <div class="create-row">
      <input v-model="draft.name" placeholder="Name (e.g. Deep)" />
      <label>Work <input v-model.number="draft.workMinutes" type="number" min="1" /></label>
      <label>Rest <input v-model.number="draft.restMinutes" type="number" min="1" /></label>
      <label>Long break <input v-model.number="draft.longBreakMinutes" type="number" min="1" placeholder="—" /></label>
      <label>every <input v-model.number="draft.longBreakEvery" type="number" min="1" placeholder="—" /></label>
      <button @click="create">Add preset</button>
    </div>
    <p class="hint">Long-break fields go together: set both or neither.</p>
  </section>
</template>

<style scoped>
.presets table { width: 100%; border-collapse: collapse; margin: 12px 0; }
.presets th, .presets td { text-align: left; padding: 6px 10px; border-bottom: 1px solid #20242b; }
.create-row { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
.create-row input[type="number"] { width: 60px; }
.hint { color: #6b7484; font-size: 12px; }
.error { color: #e06c75; }
.actions { white-space: nowrap; }
.ghost { background: none; border: none; color: #7d8796; cursor: pointer; padding: 0 4px; }
.ghost:hover { color: #d7dde7; }
.danger:hover { color: #e06c75; }
</style>
