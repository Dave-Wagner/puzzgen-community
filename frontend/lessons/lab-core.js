// Original CC0 teaching examples; no API, dictionary download or model required.
export const presets = {
  spoonerism: [
    { name: 'Light rain → right lane', values: ['l', 'aɪt', 'r', 'eɪn'], spelling: ['light rain', 'right lane'] },
    { name: 'Cold goat → gold coat', values: ['k', 'oʊld', 'ɡ', 'oʊt'], spelling: ['cold goat', 'gold coat'] },
    { name: 'Blushing crow → crushing blow', values: ['bl', 'ʌʃɪŋ', 'kr', 'oʊ'], spelling: ['blushing crow', 'crushing blow'] },
  ],
  anagram: [
    { name: 'Listen / silent', values: ['listen', 'silent'] },
    { name: 'Repeated letters: letter / teller', values: ['letter', 'teller'] },
    { name: 'Garden / danger', values: ['garden', 'danger'] },
  ],
  'word-ladder': [
    { name: 'Cat to dog: two shortest routes', values: ['cat', 'dog', 'cat cot cog dog dot log lot', 'cat cot dot dog'] },
    { name: 'Cold to warm', values: ['cold', 'warm', 'cold cord card ward warm worm word wood', 'cold cord card ward warm'] },
    { name: 'Disconnected graph', values: ['cat', 'sun', 'cat cot cog dog sun', 'cat sun'] },
  ],
  'compound-bridge': [
    { name: 'Sun / flower / pot', values: ['sun', 'flower', 'pot'] },
    { name: 'Rain / bow / tie', values: ['rain', 'bow', 'tie'] },
    { name: 'Tea / cup / cake', values: ['tea', 'cup', 'cake'] },
    { name: 'Ice / cream / puff', values: ['ice', 'cream', 'puff'] },
  ],
};
const word = value => value.trim().toLowerCase();
const validWord = value => /^[a-z]{1,24}$/.test(value);
const problem = text => [{ title: 'Check your experiment', text }];
export const adjacent = (a, b) => a.length === b.length && [...a].filter((c, i) => c !== b[i]).length === 1;

export function spoonLesson(values, preset) {
  const [a, restA, b, restB] = values.map(s => s.trim());
  if (values.some(s => s.length > 24) || !restA || !restB || !a || !b) return problem('Fill all four sound blocks (up to 24 characters each). These are sounds, not automatically inferred spellings.');
  const exact = values.every((s, i) => s.trim() === preset.values[i]);
  return [
    { title: '1. Hear the starting sounds', text: exact ? `Our reviewed US-English example is “${preset.spelling[0]}”. Slashes below mark sounds, not spelling.` : 'This is your custom sound experiment. We cannot infer its spelling or confirm that it makes English words.', tokens: [`/${a}${restA}/`, `/${b}${restB}/`] },
    { title: '2. Separate onset from remainder', text: 'The onset is the opening consonant sound or cluster. Keep each remainder in its original position.', rows: [['Word', 'Onset', 'Remainder'], ['First', a, restA], ['Second', b, restB]] },
    { title: '3. Exchange only the onsets', text: a === b ? 'These onsets are identical, so exchanging them changes nothing. Try two different sounds.' : 'Move the second onset to the first word and the first onset to the second. A cluster such as /bl/ moves as one block.', tokens: [`${b} + ${restA}`, `${a} + ${restB}`] },
    { title: '4. Recombine and listen', text: exact ? `/${b}${restA}/ /${a}${restB}/ gives “${preset.spelling[1]}”. Notice that spelling may change even though the remaining sounds do not.` : `Your result is /${b}${restA}/ /${a}${restB}/. This demonstrates the sound rule; it is not a dictionary-validated spoonerism.`, tokens: [`/${b}${restA}/`, `/${a}${restB}/`] },
  ];
}

export function anagramLesson(values) {
  const [source, candidate] = values.map(word);
  if (![source, candidate].every(validWord)) return problem('Enter two single words using A–Z, up to 24 letters each. Case is ignored. This lab counts letters; it does not look up words.');
  const letters = [...new Set(source + candidate)].sort();
  const count = (s, c) => [...s].filter(x => x === c).length;
  const rows = [['Letter', 'Source count', 'Candidate count', 'Comparison'], ...letters.map(c => [c, String(count(source, c)), String(count(candidate, c)), count(source, c) === count(candidate, c) ? 'Match' : 'Different'])];
  const equal = letters.every(c => count(source, c) === count(candidate, c));
  return [
    { title: '1. Preserve every letter', text: `“${source}” has ${source.length} letters. Your candidate “${candidate}” has ${candidate.length}. Equal length is necessary, but not enough.`, tokens: [...source] },
    { title: '2. Count the multiset', text: 'A multiset preserves duplicates. A simple set would hide the difference between one L and two Ls.', rows },
    { title: '3. Compare sorted signatures', text: 'Sorting produces a reproducible signature. Compare the signatures character by character.', tokens: [[...source].sort().join(''), [...candidate].sort().join('')] },
    { title: '4. Decide what was proved', text: equal ? (source === candidate ? 'The counts match, but this is the same word. Try a different arrangement. Matching counts alone does not establish a different English word.' : 'All letter counts match: this is a valid letter rearrangement. The lab has not proved that either entry is a dictionary word or fits a clue. The API uses its curated vocabulary and answer policy.') : 'The counts differ: this candidate is not an anagram of the source. Find a “Different” row, edit a word, and step through again.' },
  ];
}

export function validatePath(path, start, end, allowed) {
  if (path.length < 2 || path[0] !== start || path.at(-1) !== end) return 'Include the start and end in order.';
  if (path.some(w => !allowed.includes(w))) return 'A path word is missing from your allowed vocabulary.';
  if (new Set(path).size !== path.length) return 'Do not repeat words in your path.';
  if (path.slice(1).some((w, i) => !adjacent(path[i], w))) return 'Every step must change exactly one letter.';
  return 'Valid path in this lab graph. Other paths may work too.';
}
export function ladderLesson(values) {
  const [start, end] = values.slice(0, 2).map(word);
  const raw = values[2].trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (raw.length > 40 || values[3].length > 600) return problem('Keep the vocabulary to 40 words and your path to 600 characters so this experiment stays small.');
  if (![start, end, ...raw].every(validWord) || start.length !== end.length) return problem('Use A–Z words of up to 24 letters. Start and end must have equal length.');
  if (start === end) return problem('Choose different start and end words to explore a route with at least one change.');
  const words = [...new Set(raw)].sort();
  if (!words.includes(start) || !words.includes(end)) return problem('Include both start and end in the allowed vocabulary.');
  if (words.some(w => w.length !== start.length)) return problem('Every allowed word must have the same length as the start.');
  const path = values[3].trim().toLowerCase().split(/\s+/).filter(Boolean);
  const steps = [{ title: '1. Build a graph', text: 'Each allowed word is a vertex. An edge joins words differing at exactly one position. Your supplied vocabulary defines the entire graph; we do not check dictionary meanings.', rows: [['Word', 'Neighbors'], ...words.map(w => [w, words.filter(n => adjacent(w, n)).join(', ') || '(none)'])] }];
  let frontier = [[start]];
  const visited = new Set([start]);
  let distance = 0;
  let found;
  while (frontier.length) {
    steps.push({ title: `Search layer ${distance}`, text: `These words are ${distance} change(s) from “${start}”. Breadth-first search examines a whole layer before moving farther away.`, tokens: frontier.map(p => p.at(-1)) });
    found = frontier.find(p => p.at(-1) === end);
    if (found) break;
    const next = [];
    for (const p of frontier) for (const candidate of words) if (!visited.has(candidate) && adjacent(p.at(-1), candidate)) { visited.add(candidate); next.push([...p, candidate]); }
    frontier = next; distance++;
  }
  steps.push({ title: found ? 'A shortest route' : 'No connecting route', text: found ? `${found.join(' → ')} uses ${found.length - 1} changes. This is one shortest route, not necessarily the only one.` : 'The search exhausted the reachable component. Add a bridging word to connect the islands and try again.', tokens: found ?? [] });
  steps.push({ title: 'Check your own path', text: validatePath(path, start, end, words) + ' This lab has no move budget; the API also enforces its generated max_steps.', tokens: path });
  return steps;
}

const compounds = new Set(['sunflower','flowerpot','rainbow','bowtie','teacup','cupcake','icecream','creampuff','snowball','ballroom','toothbrush','brushwood','bookcase','casework','doorbell','belltower','foot ball','ballpark','seashell','shellfish','butterfly','flypaper','starfish','fishbowl','birdhouse','houseboat','moonlight','lighthouse'].map(s => s.replaceAll(' ', '')));
export function bridgeLesson(values) {
  const [left, middle, right] = values.map(word);
  if (![left, middle, right].every(validWord)) return problem('Fill the left, middle and right blocks with A–Z words, up to 24 letters each.');
  const first = left + middle, second = middle + right;
  const describe = text => compounds.has(text) ? 'In this lab’s reviewed collection' : 'Not in this lab’s small collection; that does not prove it is invalid English';
  return [
    { title: '1. Find a shared role', text: 'The same middle word must complete two compounds. Read the blocks in order.', tokens: [left, middle, right] },
    { title: '2. Read the left compound', text: `${left} + ${middle} → ${first}. ${describe(first)}. Think about what the joined expression means.`, tokens: [left, '+', middle, '→', first] },
    { title: '3. Read the right compound', text: `${middle} + ${right} → ${second}. ${describe(second)}. Compounds may conventionally use a space or hyphen: “ice cream” is not normally written “icecream”.`, tokens: [middle, '+', right, '→', second] },
    { title: '4. Test both meanings', text: compounds.has(first) && compounds.has(second) ? 'Both combinations are in this lab’s reviewed collection. Explain each meaning, then change only the middle block to see how both sides depend on it.' : 'At least one combination is outside this lab’s collection. It may be nonsense, or a real compound we have not included. A small lookup table is not a complete English dictionary.', rows: [['Combination (joined spelling)', 'Evidence'], [first, describe(first)], [second, describe(second)]] },
  ];
}
export const builders = { spoonerism: spoonLesson, anagram: anagramLesson, 'word-ladder': ladderLesson, 'compound-bridge': bridgeLesson };
