# Security

Do not post credentials, personal documents, private saves, access tokens or
unredacted server logs in issues. Report suspected vulnerabilities privately to
apps+support@davewagner.org. Include reproduction steps and affected versions,
without live credentials or other people's information. No response-time or
bug-bounty commitment is made.

Keep `.env`, service-account keys, signing keys/passwords, local verification
documents and backup receipts out of Git and release assets. Firebase Android
client config is public metadata, not an Admin credential; server token validation
and usage limits remain necessary. Never embed a shared privileged API key in a
public app. Signing backups and Google Play publication are separate operations.

Before public visibility or distribution, scan every intended branch/tag and the
proposed source snapshot, inspect findings, and review existing release assets.
A scanner is not proof that no secret exists. If a real credential entered Git,
revoke/rotate it first and review history removal before publication.
