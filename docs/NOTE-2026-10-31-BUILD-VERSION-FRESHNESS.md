2026-10-31

# Build version freshness

Anvil admitted Obscura commit `01ef95c` and built the XML clone fix, but the resulting banner reported `0.1.0-dev+6fa054d`. The build script reads `git rev-parse HEAD` but only declares environment rerun dependencies. Cargo can reuse the build-script result after a branch advances (AppGen vm-ccpy).

Add rerun dependencies for the Git HEAD file, the active symbolic ref when present, and packed refs, using Git's own `--git-path` resolution so nested package paths and linked Git metadata work. Preserve explicit version and tag precedence and no-Git fallback. Validation must build twice with unchanged source and observe a real Git ref advance, then verify the Anvil-built executable's version against its admitted source; do not manufacture a version or clear the shared build cache. Run focused tests and the repository's release gates. This only changes build metadata; rollback is confined to the build script and its tests.

Updated: 2026-10-31
Direct build-script checks pass for HEAD and branch dependency paths, current revision, explicit and tagged overrides, and no-Git fallback. A real sequential Anvil rebuild across a ref advance remains required.
