# Move completed CDP child arrays instead of serializing them again

VPM: vm-t3fo. MiniMe sampled a live ASF WebPilot Obscura process handling
Decker CPS v214. In a 160-sample connection-thread capture, 63 samples passed
through serialize_node and recursive serde_json::to_value calls. The capture
is /Users/hannes/anvil/work/evidence/asf-webpilot-obscura-hot-sample-20261023.txt.

The iterative DOM builder already owns Vec<Value> children. json!(frame.built)
serializes and copies each completed descendant subtree again when folding
it into every ancestor. Value::Array(frame.built) moves the same values into
the response. Preserve child order, null entries, protocol fields and depth
limits. No timeout or watchdog change.

Ten focused release DOM tests pass. Full release nextest, release executable,
33-stage obstacle course and actual installed timing remain delivery gates.
Do not claim this fixes every watchdog termination or hosted journey failure.
