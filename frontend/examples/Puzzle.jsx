// React component recipe: place in an existing React app. No key in public mode.
// Loading, error, success; user-initiated fetch. Extend with /verify as an exercise.
import { useState } from 'react';
export default function Puzzle({ apiOrigin = 'http://127.0.0.1:8080', apiKey = '' }) {
  const [state, setState] = useState({ loading: false, puzzle: null, error: '' });
  async function load() {
    setState({ loading: true, puzzle: null, error: '' });
    try {
      const response = await fetch(`${apiOrigin}/api/v1/puzzles/arithmetic?seed=42&difficulty=easy`, { headers: apiKey ? { Authorization: `Bearer ${apiKey}` } : {}, signal: AbortSignal.timeout(20000) });
      const data = await response.json();
      if (!response.ok) throw new Error(data.error?.message || `HTTP ${response.status}`);
      setState({ loading: false, puzzle: data, error: '' });
    } catch (error) { setState({ loading: false, puzzle: null, error: error.message }); }
  }
  return <section><button disabled={state.loading} onClick={load}>Load a puzzle</button>
    <p aria-live="polite">{state.loading ? 'Loading…' : state.error || state.puzzle?.data.expression || 'Ready to build.'}</p></section>;
}
