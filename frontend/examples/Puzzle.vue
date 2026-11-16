<!-- Vue 3 component recipe. Use inside an existing Vue app. -->
<script setup>
import { ref } from 'vue';
const props = defineProps({ apiKey: { type: String, default: '' }, apiOrigin: { type: String, default: 'http://127.0.0.1:8080' } });
const loading = ref(false), puzzle = ref(null), error = ref('');
async function load() {
  loading.value = true; error.value = ''; puzzle.value = null;
  try {
    const response = await fetch(`${props.apiOrigin}/api/v1/puzzles/arithmetic?seed=42&difficulty=easy`, { headers: props.apiKey ? { Authorization: `Bearer ${props.apiKey}` } : {}, signal: AbortSignal.timeout(20000) });
    const data = await response.json();
    if (!response.ok) throw new Error(data.error?.message || `HTTP ${response.status}`);
    puzzle.value = data;
  } catch (cause) { error.value = cause.message; }
  finally { loading.value = false; }
}
</script>
<template><section><button :disabled="loading" @click="load">Load a puzzle</button>
<p aria-live="polite">{{ loading ? 'Loading…' : error || puzzle?.data.expression || 'Ready to build.' }}</p></section></template>
