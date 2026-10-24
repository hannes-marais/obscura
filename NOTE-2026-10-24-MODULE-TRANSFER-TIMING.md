# Module transfer measurements

2026-10-24

VPM vm-t3fo: the Decker v214 static-transfer receipt has an empty resource
timeline for its entry module, while CDP reports a synthetic zero-byte200.
The module loader actually fetches a complete Response, but discards it when
returning ModuleSource. Page later invents a network event after evaluation.

Carry typed transport observations from the bounded HTTP body reader to module
completion: actual encoded bytes, header/body completion instants, cache reuse
and timing permission accumulated across redirects. Record module resource
timings and CDP completion once, from this response. Unknown transport values
stay unknown; cached/shared responses must not count another physical transfer.
Do not weaken the static-transfer assertions or change request policy.

Validate real gzip module graphs, HTTP errors, cache reuse, TAO and redirects,
and document replacement. Preserve cancellation and existing callback behavior.
Use focused release nextest, full render nextest and the33-stage obstacle
course. Product binaries are built through Anvil using the existing repository
cache. No release/readback claim follows from local test success alone.

Focused release validation:839 nextest cases passed in7.239s, including a
compressed imported child, real404 status, TAO across redirect hops, one
physical response among16 cache followers, and navigation-boundary rejection.
The first fixture run exposed a missing document URL in the test runtime;
fixtures now set the actual owning URL. Unknown encoded sizes on other
transports remain unknown rather than using decoded bytes as wire evidence.

Revision26cd690 passed the full render gate:2224 tests,5 existing skipped;
the Anvil-built optimized main/worker passed the33/33 obstacle course.
Installed Debug IA Tools0.1.677 carries this instrumentation. Live ASF run
01a16d75-2936-7612-81d8-6594264b8d09 records the hosted gzip entry module as
2426970 encoded bytes rather than zero,329818 above the existing2MiB budget.
That payload failure must remain visible. The concurrent home-page-size task
watchdog failure is separately retained in SQLite and Log Central.

Follow-up: preserve lazy-module measurements across same-document SPA routes.
The navigation clock and origin identify the document; a full-URL comparison
incorrectly rejects hash/history changes. Both focused boundary tests pass.
Full-engine gates and rollout for this follow-up remain separate from the
26cd690 proof above.
