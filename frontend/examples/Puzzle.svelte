<!-- Svelte 5 component recipe. Use inside an existing Svelte app. -->
<script>
  let { apiOrigin = 'http://127.0.0.1:8080', apiKey = '' } = $props();
  let loading = $state(false), puzzle = $state(null), error = $state('');
  async function load() {
    loading = true; puzzle = null; error = '';
    try {
      const response = await fetch(`${apiOrigin}/api/v1/puzzles/arithmetic?seed=42&difficulty=easy`, { headers: apiKey ? { Authorization: `Bearer ${apiKey}` } : {}, signal: AbortSignal.timeout(20000) });
      const data = await response.json();
      if (!response.ok) throw new Error(data.error?.message || `HTTP ${response.status}`);
      puzzle = data;
    } catch (cause) { error = cause.message; }
    finally { loading = false; }
  }
</script>
<section><button disabled={loading} onclick={load}>Load a puzzle</button>
<p aria-live="polite">{loading ? 'Loading…' : error || puzzle?.data.expression || 'Ready to build.'}</p></section>
