# Fetch bytes from object URLs

2026-10-17.

An uploaded browser file can be retained as a Blob URL before its bytes are
saved to a host. createObjectURL currently retains only decoded Worker source,
while fetch sends every URL to the HTTP operation. Fetching a locally created
binary Blob therefore fails with net::ERR_FAILED. A four-byte fixture containing
NUL and non-UTF-8 bytes reproduces this independently of any application.

Keep a private, per-runtime registry of immutable Blob snapshots alongside the
existing Worker source registry. Resolve only registered object URLs in fetch;
preserve exact bytes, content type and length, normal Response consumption,
fragments, abort semantics and revocation. Unknown or revoked URLs and methods
other than GET reject. They must never turn into network requests or relax the
HTTP private-network/redirect checks. Preserve existing Worker behavior.

Validate focused runtime cases, the normal release nextest suite, the documented
render-enabled CLI build and the companion obstacle course. Re-run the original
standalone binary fixture and the browser upload journey using an owned engine.
Source proof, locally built engine proof and installed distribution remain
separate. No generated snapshots, application documents or credentials belong
in this change.
