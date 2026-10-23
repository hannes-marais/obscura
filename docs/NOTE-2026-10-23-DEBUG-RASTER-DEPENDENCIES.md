2026-10-23

# Keep Debug browser screenshot rasterization optimized

VPM vm-iocb; owner codex-minime-asf. Live ASF Decker v214 run
01a16a2c-c006-77a3-b4a1-7f7b7eb63b19 spends CPU in screenshot capture.
The 19:12:10 UTC sample puts all 87 active connection-thread samples under
screenshot_region/paint and Tiny-Skia. This is not evidence of JS busy-wait.
The native SQLite run is progressing (30 passing steps by 192 seconds).

Debug currently compiles dependency SIMD raster loops with opt-level 0.
Optimize only tiny-skia and tiny-skia-path in the dev profile. Keep the
application in Debug, with symbols, assertions and incremental compilation;
retain every screenshot and existing rendering/task/security guard. The
unchanged Release profile remains the daily optimized distribution.

A deterministic 1280x900 SVG card fixture (24 cards, gradients and alpha)
produced identical PNG bytes across three installed baseline runs: median
0.8298 seconds, SHA256 850c93b5e17d65cbb5d3da604d990a48886701e0480476550afeca7353b4c242.
This synthetic timing is not a hosted-journey speedup claim. Evidence lives
under /Users/hannes/anvil/work/evidence/obscura-debug-raster-20261023*.

Validate focused/full release nextest, exact release build and 33/33 obstacle
course per AGENTS, build Debug through Anvil, compare fixture PNG bytes and
timing, then publish and verify live ASF runs. No screenshot skipping or
assertion weakening. Existing browser/factory ownership and acceptance apply.

Focused paint-enabled release coverage passed: 677 tests, 1 skipped. An
initial package-only invocation omitted its paint feature and exercised the
fallback layout configuration (29 failures); logs are retained, assertions
unchanged, and the intended paint-enabled configuration was rerun above.
