# Licensing and attribution

Original project code is MIT licensed; see LICENSE. This does not relicense
third-party code, fonts, music, or data. Public source and free hosting do not
grant unlimited access to the hosted API. Independent deployments are welcome.

| Material | Terms and retained notices |
| --- | --- |
| Original language/discovery corpus | CC0 dedication in data/language/README.md and data/discovery/README.md |
| Vendored Bevy render/PBR and wgpu-hal | Original MIT/Apache-2.0 texts in each third_party directory; local patches retain upstream notices |
| Kenney art, particles, prompts, audio, skies | CC0; original pack notices in apps/android/game/public/licenses |
| Quaternius nature | CC0; source/provenance in art/garden3d/sources.json and garden-weather.txt; see review caveat below |
| ambientCG Snow010A | CC0; credited source and modifications in garden-weather.txt and credits.json |
| Carefree and Enchanted Journey | Kevin MacLeod, incompetech.com, CC BY 4.0; separate original track/artist/license/change notices in public/licenses |
| Atkinson Hyperlegible, Bree Serif, Fira Mono | SIL Open Font License; retain each font's OFL notice |
| Open-Meteo weather and GeoNames location data | CC BY 4.0 attribution in garden-weather.txt; hosted service usage terms apply separately |
| Original synthesized music/weather audio | MIT; generator paths and hashes in credits.json |
| Android NDK runtime | Retain Android-NDK-NOTICE.txt when distributing bundled native runtime |

Paths under `public` above are relative to `apps/android/game/public`.
`credits.json` and `art/garden3d/sources.json` record selected files and changes.
The in-app Credits reader must remain available in redistributed builds.
Dependencies resolved from Cargo/Gradle retain their own licenses; this table is
not a complete transitive dependency notice bundle.

## Publication review caveats

- The supplied Quaternius notice names Ultimate Platformer Pack whereas the
  imported models are attributed to Ultimate Stylized Nature. The publisher's
  [Nature pack page](https://quaternius.com/packs/ultimatestylizednature.html)
  independently identifies CC0 (checked 2026-11-16). Keep the original template
  notice unchanged and retain this source clarification.
- Review provenance for original/generated home artwork and design concept
  images; do not assume third-party assets are covered by the project MIT file.
- Confirm all runtime files are represented in the attribution inventory and
  produce a complete resolved Cargo/Gradle notice inventory before distribution.
- Preserve CC-BY creator/source/license/change information. Do not describe the
  entire asset library as CC0 or apply non-commercial restrictions to MIT code.
- The resolved Cargo graph includes Symphonia packages under MPL-2.0. Preserve
  their notices and provide the corresponding covered source when distributing
  binaries; MIT project licensing does not waive dependency obligations. The
  graph also contains multiple permissive-license alternatives. Check the actual
  distributed target rather than treating all 713 resolved packages as shipped.

References: [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/),
[Firebase public client configuration](https://firebase.google.com/docs/projects/api-keys).
