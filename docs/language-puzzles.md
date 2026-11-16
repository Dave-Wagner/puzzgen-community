# Language puzzle API (content v2)

Four seeded kinds are available through the existing endpoints:
`spoonerism`, `anagram`, `word-ladder`, `compound-bridge`.
All share current authentication, account quotas, batch limits and CPU bounds.
No AI service, download, database migration or external dictionary is needed.

```bash
curl -X POST https://puzzlegen.davewagner.org/api/v1/generate \
  -H "Authorization: Bearer $PUZZGEN_API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"kind":"spoonerism","seed":42,"difficulty":"medium"}'
```

This command becomes available on the VPS after deploying this change. Local
implementation alone does not update the live server.

Generate via `GET /api/v1/puzzles/{kind}?seed=42&difficulty=medium`,
`POST /api/v1/generate`, `POST /api/v1/batch`, or
`GET /api/v1/daily/{kind}`. Consult `/api/v1/catalog` and `/api/v1/openapi.json`.
Set `include_solution: true` only for development or a deliberate reveal.
Seeds are replay handles, not an anti-cheat boundary. Decimal response seed
strings preserve u64 precision; POST seeds must be exact JSON integers.

Submit the same kind, seed and difficulty to `POST /api/v1/verify`:

```json
{"kind":"spoonerism","seed":42,"difficulty":"medium","answer":"your two-word phrase"}
```

| Kind | Public data | Answer |
|---|---|---|
| spoonerism | source_phrase, clue, word_lengths; optional choices/hint/source_sounds | Target two-word phrase string |
| anagram | letters, length; clue/first_letter or allowed_words | Single word string |
| word-ladder | start, end, allowed_words, minimum_steps, max_steps | Array including both endpoints |
| compound-bridge | left, right, length; optional clue/first_letter | Middle word string |

ASCII case and repeated/outer whitespace are normalized. Non-string answers are
rejected; punctuation is not silently stripped. Word ladders accept alternate
paths if each step changes exactly one letter, all words are in allowed_words,
there are no repeats, and the path respects max_steps. minimum_steps is computed
by BFS in that vocabulary, rather than assumed from the source chain.

Difficulty presets change assistance, not a calibrated reading-age rating:
- Spoonerisms: Easy gives four choices and sound guidance; Medium gives guidance;
  Hard keeps the clue and source phrase without sound hints.
- Anagrams: Easy gives a clue and initial letter; Medium a clue; Hard supplies the
  full curated vocabulary instead of a clue and accepts any vocabulary anagram.
- Bridges: Easy gives clue and initial letter; Medium clue; Hard word length only.
- Ladders: presets choose endpoints farther apart along a source chain and allow
  bounded detours. Graph shortcuts mean optimal distance need not increase.

Clued puzzles accept their curated answer, not every imaginable synonym or accent
variant. Catalog uniqueness fields make this explicit. Spoonerisms swap consonant
**sounds**, including clusters, rather than first letters. Their reviewed sound
annotations target general US English; arbitrary English-to-phoneme conversion
and accent selection are deliberately not implemented yet.

Content lives in `data/language/en-us-v2.json`: ten spoonerism pairs usable in
both directions, 131 clued anagrams, 68 bridges, and 36 ladder chains.
It is original CC0 content; see that directory's README for provenance. The Rust
engine uses existing MIT/Apache-compatible rand, rand_chacha, serde and serde_json
dependencies; no new library is required. Corpus size is finite and seeds repeat.
Keep corpus ordering and algorithms stable for replay. Future language changes must introduce a new content version and preserve the
older generation branch so saved challenges remain replayable.

The web playground supports all four kinds with plain-text answers, optional choices, and a word-ladder path builder. Teaching workshops live at /lessons/spoonerism.html, /lessons/anagram.html, /lessons/word-ladder.html and /lessons/compound-bridge.html. Android integration remains a separate follow-up.

For a local stateless playground preview, run `bun dev` from the repository root
and open http://127.0.0.1:8081/. Bun starts the Rust API and Trunk's frontend watch
server; it adds no JavaScript backend. Ctrl+C stops both process trees. This
command deliberately ignores DATABASE_URL and PUZZGEN_API_KEYS to avoid connecting
a local preview to the VPS database. Use the existing documented Rust/account
workflow when testing authentication or account quotas. The first build may fetch
missing Rust/Trunk tool dependencies.

## Learning and replay

Version 2 adds learning.objective, learning.principle and learning.experiment.
The playground presents these before play and shows a worked explanation after
success or reveal. Learning notes do not contain the hidden target answer.

Omitted content_version selects the latest collection (currently "2"). To replay
or verify a saved challenge, send its data.content_version. The original corpus
remains available as "1" on generate, batch, daily and verify routes. Version 1
keeps original IDs and output; version 2 IDs end with -content2. The global
mathematical generators and their IDs are unchanged. Passing a content version
for a mathematical puzzle, or an unsupported language version, is an error.

`frontend/examples/fixtures/language.json` supplies offline teaching fixtures for
both versions, including explicitly revealed solutions. Regenerate with
`cargo run --offline -p puzzgen --example export_language_fixtures`.
The fixtures are construction tools, not secret answers for competitive play.
## Interactive teaching labs

Each /lessons/ page includes an editable browser-only walkthrough. labs.js owns
labeled controls and step navigation; lab-core.js contains the pure learning
models. These use small original CC0 examples and make no API requests. They do
not replace backend validation, infer arbitrary pronunciation, or supply a full
English dictionary. Word-graph experiments are limited to 40 vocabulary entries.
Run `bun test tests/language-labs.test.js` for deterministic lab-model checks.
