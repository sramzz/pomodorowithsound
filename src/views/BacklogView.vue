<script setup lang="ts">
import { onMounted } from "vue";
import { useProjectStore } from "../stores/projectStore";

const store = useProjectStore();
onMounted(() => store.loadProjects());
</script>

<template>
  <section>
    <h1>Backlog</h1>
    <p v-if="store.loading">Loading…</p>
    <p v-else-if="store.error" class="error">{{ store.error }}</p>
    <p v-else-if="store.projects.length === 0" class="placeholder">
      No projects yet. Project creation arrives in Phase 2.
    </p>
    <ul v-else>
      <li v-for="p in store.projects" :key="p.id">{{ p.name }}</li>
    </ul>
  </section>
</template>
