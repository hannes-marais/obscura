//! Privacy-safe command timing. Parameters, selectors and input values never enter this API.
use std::time::Instant;

pub(crate) struct CommandStage<'a> {
    id: u64,
    method: &'a str,
    stage: &'static str,
    event_type: &'static str,
    started: Instant,
}
impl<'a> CommandStage<'a> {
    pub(crate) fn begin(
        id: u64,
        method: &'a str,
        stage: &'static str,
        event_type: &'static str,
    ) -> Self {
        tracing::debug!(
            command_id = id,
            method,
            stage,
            event_type,
            elapsed_ms = 0_u64,
            "cdp.command.stage.begin"
        );
        Self {
            id,
            method,
            stage,
            event_type,
            started: Instant::now(),
        }
    }
}
impl Drop for CommandStage<'_> {
    fn drop(&mut self) {
        tracing::debug!(
            command_id = self.id,
            method = self.method,
            stage = self.stage,
            event_type = self.event_type,
            elapsed_ms = self.started.elapsed().as_millis() as u64,
            "cdp.command.stage.end"
        );
    }
}
// Only protocol enum values are admitted: an arbitrary client string cannot leak input.
pub(crate) fn key_event_type(value: &str) -> &'static str {
    match value {
        "keyDown" => "keyDown",
        "rawKeyDown" => "rawKeyDown",
        "keyUp" => "keyUp",
        "char" => "char",
        _ => "unknown",
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn input_event_metadata_rejects_arbitrary_client_values() {
        for allowed in ["keyDown", "rawKeyDown", "keyUp", "char"] {
            assert_eq!(super::key_event_type(allowed), allowed);
        }
        for private in ["secret-value", "keyDown\nsecret", "", "Bearer secret"] {
            assert_eq!(super::key_event_type(private), "unknown");
        }
    }
}
