2026-11-03
Updated: 2026-11-03

# Decker Plan is interrupted by Obscura's autonomous watchdog

The merged IA fork at `e77ed40` still fails the real Decker PPTX Read-to-Plan
journey. Its 73-slide, 188,953-byte input reaches Read; during Plan, Obscura
raises `autonomous browser task exceeded its task budget (5500ms)`. Captured
DOSVR responses are successful immediately before the exception, after which
only Dossier polling remains. Evidence is under
`/home/oumy/anvil/work/evidence/obscura-upstream-e77ed40-decker-pptx-20261103/`.
The backend job's terminal state and the interrupted JavaScript stack were not
captured. The watchdog is armed around `poll_event_loop`, so the evidence points
to a long synchronous V8 task, but does not identify its function.

Tracked in PCG VPM `vm-r1wv`. Governing architecture principles: 4 (contain a
page failure to its worker), 7 (make failure observable and bound work), 18
(bound execution against realistic input), 35 (restore the actual user
journey), and 36 (set budgets from workload evidence). The 5.5-second limit
is a measured default that currently aborts this real journey. Preserve a hard
upper bound; do not mask exceptions or infer a backend defect. ToQ: the pinned
73-slide Plan completes, or fails with a specific attributable cause, with the
engine remaining usable. ToT: one isolated real journey and a focused runtime
regression; no app-specific behavior in the generic engine.

Validation: focused autonomous-watchdog nextest; complete release nextest and
release CLI build; 33/33 obstacle course; then a managed Anvil build and the
same isolated real Decker journey. Preserve the existing shared QA pool. Any
remaining watchdog interruption keeps `vm-r1wv` open.

Focused nextest run `205ab025-108a-4adb-b869-f50525db91c2` passes both the
six-second task regression and the infinite-task termination/recovery case.
This validates the explicit bound locally; it does not establish the Decker
journey until the managed build is exercised.

Full release nextest `81597d6a-6ea0-43e4-a5fc-80ce7ff9e5b9`: 2,551 passed,
5 skipped. The required release CLI build succeeded, and the obstacle course
passed 33/33. Anvil operation `b04221c6-d35e-4456-8f11-88eaf59eaf8b` built
commit `2feb446` as `obscura 0.1.0-dev+2feb446`; managed binary SHA-256 is
`a90baf285c08940f4c7314775773ed4701af26a3893a79d1d0caec5673af4272`.

The same isolated real Decker journey then passed 79/79 steps on that binary:
Upload 30.4s, Read 28.9s, Plan 170.6s; Plan exposed all 73 cards and enabled
Build. Runtime and console error counts are zero; Dossier health has no errors,
failures, pending items or incomplete controls. The 296-request Plan window
contains one HTTP 404 `GetObject`, followed by successful reads of that object;
the complete journey and its acceptance checks passed. Evidence is
`/home/oumy/anvil/work/evidence/obscura-decker-watchdog-2feb446-20261103/`.
Only the isolated server on port 19702 was stopped. A second real journey using
the retained 32-slide Check fixture found the 20-second bound still too short:
the watchdog fired during `GetTurnJobEvents` at `23:39:33.753Z`; only Dossier
polling continued afterward. The persisted deck still showed an enabled
"Check the deck" action, without results. The server job's final state remains
unknown. The owner stopped only the browser runner after five settled
Dossier-only poll pairs and stopped only its isolated server. The VPM task was
returned to bug status; no Check retry has been launched. The runtime bound is
kept at 20 seconds for now. A 60-second candidate was committed as `b7ac6c7`
but not built or deployed; review identified that it would only prolong a
non-yielding page task without showing the operation that blocks. Its full
release test build was stopped before tests ran. A V8 `request_interrupt` stack
sampler was prototyped against the existing 20-second watchdog, but the focused
infinite-task regression did not receive the callback before termination, even
with a five-second sampling window; that diagnostic was removed. The Check
repro's `Check the deck` click is followed by a 116.382-second WebPilot
`waitFor`; it ended with `browser stopped answering: Interrupted system call`
when the owner stopped the runner after polling had gone silent. That error is
not independent evidence of a WebPilot propagation defect. The reliable
finding is that the 20-second Obscura watchdog fires and DOSVR event polling
ceases, with no Check results visible. Next use an isolate CPU profiler or
app-level bounded phase timing to identify the synchronous Check function.
Preserve the 20-second worker bound and do not retry while the prior backend
job state is unknown. The
WebPilot-to-Obscura CDP link is WebSocket; Decker's DOSVR calls are HTTP POST
fetches and job polling.

After rejecting the unverified 60-second candidate, the source is back at the
previously validated 20-second watchdog and six-second allowance regression.
Focused release nextest `ad04a7a9-72be-462e-ae3e-04fa7c48a6be` passes both the
six-second task and watchdog termination/recovery cases (2 passed). The actual
Check remains unverified on Obscura; the 20-second failure remains open.
