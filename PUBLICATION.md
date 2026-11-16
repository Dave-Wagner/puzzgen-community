# Public source boundary

Beacon & Bough and Puzzgen are free, open source projects. Generic build and
development tools may be public. The maintainer's personal convenience tools
must remain private: signing/backup/recovery, key provisioning, attended-device
automation, private-server deployment/recovery, release activation and personal
compute/offload orchestration.

Public generic scripts currently approved for review: Build-AndroidPublic.ps1,
Build-Frontend.ps1, Build-Linux.ps1, Setup-Toolchain.ps1, Check.ps1, dev.ts and
export-weather-audio.py. This list is not permission to publish new scripts by
default. Review each script's purpose and content before adding it.

The public source candidate must omit private scripts, operational reports,
account/verification documents, signing metadata, .env, generated outputs and
personal workflow files. Existing private Git history contains such helpers;
ignoring/removing them from the latest tree does not remove them from history.
Publish only the separately reviewed clean-history candidate, after explicit
owner approval. Do not change the existing repository's visibility as a shortcut.

Build and sideload documentation is docs/BUILD-AND-SIDELOAD.md. All builds use
the builder's own local signing identity; no maintainer secret is required.
