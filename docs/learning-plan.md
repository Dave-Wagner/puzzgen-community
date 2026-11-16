# Puzzgen's mission and learning plan

Help a beginner make a small game without first making a backend. Personally funded, free to learn from, approachable, and useful as an asset to the wider developer community. The homepage leads with the learner's goal; implementation technologies live in the footer and contributor documentation.

## Editorial and implementation rules

- Working code first. Start from one small playable interaction, then explain it.
- Keep the site itself Rust; teach clients in plain browser code and popular web/native frameworks.
- Every lesson should have an objective, input/output shape, runnable example or clearly labeled recipe, exercise, and one concrete expected result.
- Distinguish a browser-tested starter, a source-only component recipe, an outline, and an idea. Never label an outline as a completed tutorial.
- Teach loading/errors/cancellation, keyboard access, seed precision, data/model separation, and honest puzzle guarantees alongside rendering.
- No AI dependency or token cost for generation, lessons, or games. Learning pages and offline examples require no account; member-enabled live APIs use a free account with a bounded allocation.
- Be kind to the host: fetch on user actions, honor Retry-After, use fixtures during UI work, avoid unbounded custom puzzle sizes.
- A free resource still has finite capacity. Explain shared limits and service availability without promising an SLA or literal infinite uniqueness.

## Available now

| Asset | Status | Location |
| --- | --- | --- |
| Ten-generator playground | Browser tested | `frontend/src/main.rs` |
| Learning hub, principles and exercises | Implemented | `frontend/src/learn.rs` |
| Plain HTML/JS arithmetic game | Runnable starter | `frontend/examples/index.html`, `plain.js` |
| React / Vue / Svelte components | Source recipes; framework build verification pending | `frontend/examples/Puzzle.*` |
| Arithmetic replay fixture | Static data for offline iteration | `frontend/examples/fixtures/arithmetic.json` |
| API contract / fair use | Implemented | `/docs`, `/api/v1/openapi.json` |
| Native and advanced workshop tracks | Clearly labeled outlines | Learning hub |

## Next small slices

Each item is intentionally bounded so a later session can implement code without regenerating this plan or expanding all prose.

1. **Plain Sudoku workshop:** editable 4×4 grid, preserve givens, keyboard focus, submit, reset; use seed 42 fixture. Expected result: the same puzzle accepts the same completed grid.
2. **React workshop:** promote the recipe into a runnable example with checking, cancellation/unmount handling, separate API client, and component tests for loading/failure/success.
3. **Vue workshop:** composable client and grid component; compare refs/computed with React state.
4. **Svelte workshop:** derived board state, keyboard navigation, and a fixture-driven component test.
5. **React Native workshop:** request state, safe-area layout, FlatList board, HTTPS origin configuration, app background cancellation. An emulator's localhost is not the development laptop.
6. **Flutter workshop:** typed response, async service, GridView, semantics, lifecycle cancellation, HTTPS host configuration.
7. **SwiftUI / Compose workshops:** equivalent state machine and accessible cell controls, then compare platform conventions.
8. **Puzzle principles labs:** seeded PRNG visualizer; maze DFS stack; Sudoku constraint propagation; nonogram run lengths; sliding parity; Lights Out toggles. Pair a tiny interactive lab with each explanation.
9. **Testing and resilience:** offline fixtures, saved progress/reset, stale-response guards, replay version changes, 429 backoff, touch/keyboard/screen-reader verification.
10. **More topics:** graph/word/logic puzzles only after a bounded generator and sound verifier exist. Larger explanations can be filled in after their examples work.

## Development cost discipline

Use targeted file reads, a single content source per recipe (`include_str!` displays the downloadable file), focused checks, Cargo's incremental cache, and fixtures. Avoid installing every teaching framework merely to write a placeholder. Expand and validate one workshop at a time.

The detected development CPU is Intel Core Ultra 5 225U, which Intel lists with a 12-TOPS AI Boost NPU. No NPU driver/runtime or local model is configured by this project. An optional future local inference tool could draft repetitive text or small edits; evaluate actual driver support, memory, model compatibility and task quality before adopting it. It will not accelerate Cargo, puzzle solvers, or hosted-model inference merely by being present. Keep any such tool outside the site's runtime and outside build prerequisites.

References: [Intel processor specifications](https://www.intel.com/content/www/us/en/products/sku/241861/intel-core-ultra-5-processor-225u-12m-cache-up-to-4-80-ghz/specifications.html), [OpenVINO NPU inference](https://docs.openvino.ai/2026/openvino-workflow-generative/inference-with-genai/inference-with-genai-on-npu.html).
