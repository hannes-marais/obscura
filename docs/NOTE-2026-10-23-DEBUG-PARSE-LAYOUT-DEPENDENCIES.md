# 2026-10-23 — Debug browser parse/layout dependencies

VPM: vm-t3fo. Continuation of the Debug raster dependency policy.

Fresh ASF Decker v214 journeys still reach the 5500 ms autonomous task
watchdog after disk contention was removed. A native sample contains
html5ever fragment parsing and Taffy deferred flex layout under ResizeObserver.
Instrumented live preparation parses the same 566595-character document three
times (about 600 ms total); this does not alone explain the whole timeout.
A separate offline probe parses a 54191-character detached template 100 times
in 1891 ms; width-invalidated layout stays 15 ms after those parses, rejecting
the hypothesis that detached node accumulation alone explains the slowdown.

Optimize only the measured html5ever and Taffy dependencies in Cargo's dev
profile, alongside the existing tiny-skia overrides. App code, assertions,
symbols, watchdog budgets, receipt coverage, and Debug publication stay Debug.
This is a measured candidate, not proof that the journey is repaired. Compare
the same offline probe and retained-source hosted journey after Anvil builds
it. Retain all failures. Do not introduce a routine Release compilation to
make Debug testing pass.

Evidence: ~/anvil/work/evidence/decker214-dom-cost/,
obscura-fresh-drive-picker-sample.txt, and
decker214-detached-layout-invalidation-probe.log. Native WebPilot SQLite runs
01a16bb0-ad8c-7601-841f-ee300792b45a and
01a16bb2-db6b-7911-9dee-def265d4d20e retain the actual hosted failures.
