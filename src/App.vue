<script setup lang="ts">
import { ref } from "vue";
import DayView from "./views/DayView.vue";
import BacklogView from "./views/BacklogView.vue";
import SettingsView from "./views/SettingsView.vue";
import AnalyticsView from "./views/AnalyticsView.vue";

const views = {
  day: { label: "Day", component: DayView },
  backlog: { label: "Backlog", component: BacklogView },
  analytics: { label: "Analytics", component: AnalyticsView },
  settings: { label: "Settings", component: SettingsView },
} as const;
type ViewKey = keyof typeof views;
const currentView = ref<ViewKey>("day");
</script>

<template>
  <div class="shell">
    <nav class="sidebar">
      <button
        v-for="(view, key) in views"
        :key="key"
        :class="{ active: currentView === key }"
        @click="currentView = key"
      >
        {{ view.label }}
      </button>
    </nav>
    <main class="content">
      <component :is="views[currentView].component" />
    </main>
  </div>
</template>

<style>
:root {
  color-scheme: dark;
  font-family: "Inter", system-ui, sans-serif;
}
body { margin: 0; background: #111418; color: #e6e9ef; }
.shell { display: flex; height: 100vh; }
.sidebar {
  width: 200px; padding: 16px 8px; display: flex; flex-direction: column; gap: 4px;
  background: #0b0e11; border-right: 1px solid #20242b;
}
.sidebar button {
  background: none; border: none; color: #9aa3b2; text-align: left;
  padding: 10px 12px; border-radius: 8px; font-size: 14px; cursor: pointer;
}
.sidebar button:hover { background: #181d24; color: #e6e9ef; }
.sidebar button.active { background: #1f2630; color: #fff; }
.content { flex: 1; padding: 24px; overflow-y: auto; }
.placeholder { color: #6b7280; }
.error { color: #f87171; }
</style>
