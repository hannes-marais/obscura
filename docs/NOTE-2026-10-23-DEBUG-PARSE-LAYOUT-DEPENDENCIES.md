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

## Measured result — rejected

Anvil built eadd84a in Debug in 22.6 seconds (41.6 seconds including source
synchronization and workspace wait). Binary SHA256:
61e84a2c5b33f7c8fa324a7d1e8e9c1efe13525ffc3145b38b0c2b20d7885060.
A paired old/new/new/old probe returned parsing times 1955 / 2397 / 2349 /
1952 ms; invalidated layout was 15 ms in all four. The parser override is
about 21% slower for this actual DOM workload, and no layout benefit is
established. Remove both new overrides; preserve the independently measured
raster tuning. Do not publish this candidate as a speed fix.

Full comparison: ~/anvil/work/evidence/decker214-debug-dependency-paired.jsonl.
The targeted deployment reproduction remains diagnostic evidence only and
must never satisfy an application receipt gate.
