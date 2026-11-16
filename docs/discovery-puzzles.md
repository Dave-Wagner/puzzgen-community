# Seven discovery puzzle APIs

The API now has 21 kinds. These additions work through the existing GET puzzle,
POST generate, batch, daily and verify routes, with the same authentication,
quotas and CPU limits. They require no migration or external service.

| Kind | Answer | Learning focus |
|---|---|---|
| word-sorting | Object mapping every item ID to category ID | Classification |
| sequence-detective | Integer | Explicit rule families |
| memory-pairs | Array of two-card ID arrays covering every card | Identity and state |
| path-connections | Oriented port-mask grid | Rotation and connectivity |
| futoshiki | Completed 4×4 integer grid | Latin squares and inequalities |
| sound-families | Array of all three matching candidate IDs | Rhymes beyond spelling |
| sentence-assembly | Array of all five tile IDs in order | Controlled grammar |

Generate with `GET /api/v1/puzzles/futoshiki?seed=42&difficulty=medium`.
Reveal using `include_solution=true`. Verify with the original kind, seed,
difficulty and your answer. Preserve generator_version for replay. These seven
kinds use content_revision "1", not the content_version selector belonging to
the four earlier language puzzles. Seeds need exact integer handling in JSON;
response seeds are decimal strings. See /api/v1/openapi.json and /api/v1/catalog.

Presets change category count, numerical rule families, deck/network size,
inequality/clue targets, distractor count or textual assistance. They are not
calibrated human difficulty ratings. Sentence assembly always has five tiles.

Validation checks the intended rule family, exact category coverage, complete
card pairing, reciprocal ports and global connectivity, preserved Futoshiki givens
and constraints, curated rhymes, or three supported grammar orders, respectively.
Alternative valid pipe solutions, pair orderings, rhyme-selection orders and
supported adverb placements are accepted. Futoshiki uniqueness is checked by a
bounded solver before removing each clue; exhausting a budget never proves
uniqueness. Partial and malformed submissions cannot bypass completion rules.

All seven have playable web controls and interactive workshops:
`/lessons/{kind}.html`. Workshops use editable original examples locally, with no
API requests or paid tools. They explicitly distinguish unknown words from
invalid ones, local checks from full validation, and controlled rules from all
possible English or mathematical interpretations. Content provenance is in
[data/discovery/README.md](../data/discovery/README.md).

Offline teaching fixtures (solutions deliberately included) are available at
`/examples/fixtures/discovery.json`. Regenerate with:

```powershell
cargo run --offline -p puzzgen --example export_discovery_fixtures
```

Use `bun dev`, then open http://127.0.0.1:8081/. Restart bun dev after backend
source changes; Trunk watches frontend changes but Cargo run is not a backend
watcher. This is a local preview; nothing is deployed to the VPS by that command.
