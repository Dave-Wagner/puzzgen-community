# Language content v1

The original clue text, curated word/phrase selections, sound annotations and
ladder lists in en-us-v1.json and en-us-v2.json are dedicated to the public domain under
CC0 1.0 Universal: https://creativecommons.org/publicdomain/zero/1.0/
No external dictionary, quotation collection, model API or proprietary content
is bundled. Engine code retains the repository's MIT license.

Locale: general US English. IPA-like onset/rime annotations explicitly describe
these reviewed swaps; this is not an arbitrary-text pronunciation engine.
Some phrases are deliberately whimsical. Accent-dependent alternatives are not
automatically inferred. Human editorial review remains useful before app release.

To extend: add original or verified CC0 entries, retain provenance here, run
language corpus integrity tests, and change content version and generator identity
when modifying existing seeded output. The finite corpus repeats across seeds;
this API does not promise infinitely many distinct language puzzles.

## Expanded collection (version 2)

The default collection has 10 spoonerism pairs (20 directions), 131 clued words,
68 compound bridges and 36 connected ladder chains. These additions and their
clues are original CC0 content, not imported quotations or scraped dictionaries.
Some spoonerism phrases are deliberately comic (such as belly jeans); their clues
say so. General US pronunciation is the declared convention, not a promise that
every accent uses the same sounds. Complete-compound and pronunciation quality
still benefit from human editorial review.

Version 1 remains unchanged and replayable. Set content_version to "1" in a
language generate/verify request or GET query. Version 2 is the default and has
IDs ending in -content2. New rounds can repeat; a seed does not guarantee novelty
relative to previous rounds. The same seed plus kind, difficulty and content
version is reproducible. The version-2 ladder dictionary is a seeded subset of
at most two chains, keeping the visible vocabulary small for learners.

Do not edit either published corpus in place. Create a new content version and
retain the older generation branch. Bump the global generator version only when
changing existing algorithms outside their content-version branch.