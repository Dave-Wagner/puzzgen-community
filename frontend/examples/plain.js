// A complete browser starter. Personal keys stay in memory; never publish yours.
const origin = document.querySelector('#api');
origin.value = location.origin === 'null' ? 'http://127.0.0.1:8080' : location.origin;
const expression = document.querySelector('#expression');
const answer = document.querySelector('#answer');
const status = document.querySelector('#status');
const newButton = document.querySelector('#new');
const checkButton = document.querySelector('#check');
const offline = document.querySelector('#offline');
const apiKey = document.querySelector('#api-key');
offline.checked = new URLSearchParams(location.search).has('offline');
let puzzle;
let fixturePuzzle = false;
let busy = false;

async function api(path, options = {}) {
  const base = new URL(origin.value.trim());
  if (!['https:', 'http:'].includes(base.protocol)) throw new Error('Use an HTTP or HTTPS API origin.');
  const headers = new Headers(options.headers);
  if (apiKey.value.trim()) headers.set('Authorization', `Bearer ${apiKey.value.trim()}`);
  const response = await fetch(new URL(`/api/v1/${path}`, base), {
    ...options,
    headers,
    signal: AbortSignal.timeout(20000),
  });
  const body = await response.json();
  if (!response.ok) {
    const wait = response.headers.get('Retry-After');
    throw new Error((body.error?.message || `HTTP ${response.status}`) + (wait ? ` Retry after ${wait} seconds.` : ''));
  }
  return body;
}

async function act(work) {
  if (busy) return;
  busy = true;
  newButton.disabled = checkButton.disabled = true;
  origin.disabled = answer.disabled = true;
  offline.disabled = true;
  apiKey.disabled = true;
  status.textContent = 'Working…';
  try { await work(); }
  catch (error) { status.textContent = error.message; }
  finally {
    busy = false;
    newButton.disabled = origin.disabled = answer.disabled = false;
    offline.disabled = false;
    apiKey.disabled = false;
    checkButton.disabled = !puzzle;
  }
}

document.querySelector('#connection').addEventListener('submit', event => {
  event.preventDefault();
  act(async () => {
    // A 32-bit seed survives JSON number serialization without losing precision.
    const seed = crypto.getRandomValues(new Uint32Array(1))[0];
    puzzle = undefined;
    fixturePuzzle = offline.checked;
    expression.textContent = 'Loading your next challenge…';
    if (fixturePuzzle) {
      const response = await fetch('./fixtures/arithmetic.json');
      if (!response.ok) throw new Error('The practice fixture could not be loaded.');
      puzzle = await response.json();
    } else {
      puzzle = await api(`puzzles/arithmetic?seed=${seed}&difficulty=easy`);
    }
    expression.textContent = puzzle.data.expression;
    answer.value = '';
    status.textContent = `${fixturePuzzle ? 'Offline practice fixture. ' : ''}Seed ${puzzle.seed}. Ready to solve.`;
  });
});

document.querySelector('#game').addEventListener('submit', event => {
  event.preventDefault();
  const number = Number(answer.value);
  if (!puzzle || !answer.value.trim() || !Number.isSafeInteger(number)) return;
  act(async () => {
    if (fixturePuzzle) {
      // Known fixture answer, not a general-purpose or competitive validator.
      status.textContent = number === 650 ? 'You solved the practice fixture!' : 'Not quite. Try another answer.';
      return;
    }
    const result = await api('verify', {
      method: 'POST', headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ kind: puzzle.kind, seed: Number(puzzle.seed), difficulty: puzzle.difficulty, answer: number }),
    });
    status.textContent = result.correct ? 'You solved it!' : 'Not quite. Try another answer.';
  });
});
