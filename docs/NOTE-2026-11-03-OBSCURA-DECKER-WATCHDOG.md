2026-11-03

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
