<script setup lang="ts">
import { ref } from "vue";

defineProps<{ placeholder: string }>();
const emit = defineEmits<{ create: [title: string] }>();
const draft = ref("");

function submit() {
  const title = draft.value.trim();
  if (!title) return;
  emit("create", title);
  draft.value = "";
}
</script>

<template>
  <input
    v-model="draft"
    class="inline-create"
    :placeholder="placeholder"
    @keydown.enter="submit"
    @keydown.esc="draft = ''"
  />
</template>

<style scoped>
.inline-create {
  width: 100%;
  background: transparent;
  border: 1px dashed #2a313c;
  border-radius: 6px;
  color: #e6e9ef;
  padding: 6px 10px;
  font-size: 13px;
}
.inline-create:focus { border-style: solid; outline: none; border-color: #3d4756; }
</style>
