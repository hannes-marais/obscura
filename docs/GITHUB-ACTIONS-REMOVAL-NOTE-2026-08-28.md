# GitHub Actions removal note

## Scope

Remove Obscura's three tracked GitHub Actions workflows and the three tracked
`scripts/ci` helpers that are referenced only by those workflows. Preserve the
local build, test, benchmark, packaging, policy documentation, issue templates,
and release source.

## Failure modes and blast radius

- Pull-request checks, GitHub-hosted release builds, and Docker Hub publication
  will no longer run from this repository.
- The removed comparison and policy helpers will no longer be available as
  standalone scripts; no tracked non-workflow caller currently references them.
- Local Cargo commands and the external obstacle-course project are unchanged.

## Rollback

The deletions remain recoverable from Git history until committed, and from the
parent commit after commit. Restore only the exact deleted paths if rollback is
requested.

## Acceptance criteria

- No tracked path matching `.github/workflows/**` or `scripts/ci/**` remains.
- Non-CI GitHub metadata remains present.
- `git diff --check` passes.
