2026-10-31

# Decker text-fit layout cost

The real DevYard Build reaches Obscura's 5.5-second autonomous-task limit; the live trace measures 2.502 seconds inside one AppGen text-fit pass. A static captured Build DOM using its compiled CSS and assets, with no failed asset responses, has two text frames requiring the ordinary 12 fitting rounds. The same JS takes 1,018 and 1,013 ms in the Anvil release Obscura, almost entirely in scroll/client measurements after font-size writes, versus 4.7 and 2.9 ms in Chromium. Evidence is `textfit-static-release-assets` and `textfit-static-chromium` under `/home/oumy/anvil/work/evidence/decker-devyard-20261030`; PCG vm-nb4x and AppGen vm-x54b track the issue. Static replay is diagnostic, not acceptance.

Identify which phases of resource-aware retained layout cost time before choosing a repair. Temporarily profile image collection, layout and derived geometry with opt-in aggregate timings only, never page content. Preserve exact font fitting, CSS layout and watchdog semantics. Prefer a general retained-layout fix with strict eligibility and fallback over application-specific assumptions. Validate meaningful layout and mutation regressions and both browsers on the same static fixture, then test full DevYard server Build and saved-deck reopen. No claim of success until the real target passes. Roll back temporary profiling when no longer needed; preserve other agents' work.

Updated: 2026-10-31
The asset-complete static replay naturally runs all 12 rounds for both frames: Obscura 1,018 and 1,013 ms, Chromium 4.7 and 2.9 ms. Native phase instrumentation on 26 preparations measured median 0.11 ms image collection, 77.56 ms layout, and 78.28 ms through derived geometry. Existing telemetry reports a stylesheet cache hit but retained-style fallback and ~4 ms cascade. Evidence `textfit-static-release-assets`, `textfit-static-chromium`, and `obscura-layout-detailed.log`. Instrumentation was removed after the measurement. The dominant remaining cost is layout/shaping; no performance fix or real Build pass is claimed.

Updated: 2026-10-31

Further isolate the 78 ms preparation by temporarily measuring TextEngine construction and recursive layout-tree construction, plus total layout pass, using an explicit opt-in environment flag and aggregate durations/counts only. The test fixture is the previously captured, asset-complete synthetic Build DOM; do not log document content. Preserve behavior and apply no change to fitting or watchdog. Validate the measured build against the same fixture and remove the probe after measurement. The likely optimization remains unproven until phase timing and equivalence tests support it.
