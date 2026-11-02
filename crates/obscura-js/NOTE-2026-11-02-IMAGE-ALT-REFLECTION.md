2026-11-02

# Image alt reflection

Decker ASF pass 215 reported missing alt on radio thumbnails. Current deployed AppUI assigns `image.alt = ''`, but a disposable CDP control shows Obscura leaves the attribute absent while Chromium creates `alt=""`; direct setAttribute works. HTMLImageElement lacks the reflected alt accessor. Add string attribute reflection, including absent and empty values, without changing image fetching.

Architecture principles 13, 17, 24, 27, 30 and 32 govern the existing DOM contract, minimal fix, evidence and target verification. ToQ: property and content attribute remain synchronized; empty alt identifies decorative images. ToT: do not alter rendered imagery. Risk: inherited generic reflection must not be shadowed incorrectly. Files: bootstrap.js and image_alt_reflection.rs. Validate missing, empty, nonempty, attribute mutation and removal with the existing isolated runtime test; rerun the hosted accessibility journey on the rebuilt engine.
