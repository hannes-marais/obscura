# Expose autonomous task failures through Runtime.exceptionThrown

VPM: vm-t3fo.

ASF's deployed Decker v214 load-files journey stalled with zero reported
runtime errors while engine.log showed an autonomous task watchdog termination.
Stderr alone did not reach WebPilot's SQLite runtime-error records.

Queue autonomous task failures through the same bounded runtime exception
queue as uncaught JavaScript errors. Preserve the original reason and page
URL. Native watchdog errors have no invented JavaScript stack. Keep the
watchdog budget, page isolation, event subscription and recovery unchanged.
A runaway timer regression must observe the actual watchdog, receive the
runtime exception, and prove that the recovered isolate can evaluate again.
