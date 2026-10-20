//! Connection-local dialog rendezvous. Only protocol data crosses threads;
//! the original JavaScript call stays suspended on its isolate's thread.
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use obscura_js::dialog::{DialogChoice, DialogHandler, DialogRequest};
use serde_json::{json, Value};
use tokio::sync::mpsc::UnboundedSender;

struct Pending {
    generation: u64,
    deadline: Instant,
    session: String,
    page: String,
    context: String,
    kind: String,
    reply: mpsc::SyncSender<Result<DialogChoice, String>>,
}

#[derive(Default)]
struct State {
    pending: Option<Pending>,
    generation: u64,
    disconnected: bool,
}

#[derive(Clone, Default)]
pub(crate) struct DialogBroker(Arc<Mutex<State>>);

impl DialogBroker {
    pub(crate) fn handler(
        &self,
        session: String,
        page: String,
        context: String,
        output: UnboundedSender<String>,
        timeout: Duration,
    ) -> DialogHandler {
        let broker = self.clone();
        Arc::new(move |request| broker.open(&session, &page, &context, &output, timeout, request))
    }

    fn open(&self, session: &str, page: &str, context: &str, output: &UnboundedSender<String>, timeout: Duration, request: DialogRequest)
        -> Result<DialogChoice, String>
    {
        let (tx, rx) = mpsc::sync_channel(1);
        let generation = {
            let mut state = self.0.lock().unwrap();
            if state.disconnected { return Err("dialog connection closed".into()); }
            if state.pending.is_some() { return Err("another dialog is pending".into()); }
            state.generation += 1;
            let generation = state.generation;
            state.pending = Some(Pending { generation, deadline: Instant::now() + timeout, session: session.into(), page: page.into(), context: context.into(), kind: request.kind.clone(), reply: tx });
            // Publish while holding the lock: a handle cannot race ahead of opening.
            let event = json!({"method":"Page.javascriptDialogOpening", "sessionId":session,
                "params":{"url":request.url,"message":request.message,"type":request.kind,
                    "hasBrowserHandler":true,"defaultPrompt":request.default_prompt}});
            if output.send(event.to_string()).is_err() {
                state.pending = None;
                return Err("dialog opening could not be delivered".into());
            }
            generation
        };
        let result = rx.recv_timeout(timeout)
            .map_err(|_| "dialog choice was not received before cancellation deadline".to_string())
            .and_then(|choice| choice);
        let mut state = self.0.lock().unwrap();
        if state.pending.as_ref().is_some_and(|pending| pending.generation == generation) {
            state.pending = None;
        }
        result
    }

    /// Called on the socket I/O thread; never touches V8 or a DOM tree.
    pub(crate) fn handle(&self, id: u64, session: Option<&str>, params: &Value, output: &UnboundedSender<String>) {
        let mut state = self.0.lock().unwrap();
        let error = match state.pending.as_ref() {
            None => Some("No dialog is showing"),
            Some(pending) if pending.deadline <= Instant::now() => Some("Dialog choice deadline expired"),
            Some(pending) if Some(pending.session.as_str()) != session => Some("Dialog belongs to another page session"),
            _ if params.get("accept").and_then(Value::as_bool).is_none() => Some("accept must be a boolean"),
            _ if params.get("promptText").is_some_and(|text| !text.is_string()) => Some("promptText must be a string"),
            _ => None,
        };
        if let Some(message) = error {
            let mut reply = json!({"id":id,"error":{"code":-32602,"message":message}});
            if let Some(session) = session { reply["sessionId"] = json!(session); }
            let _ = output.send(reply.to_string());
            return;
        }
        let pending = state.pending.take().unwrap();
        let choice = DialogChoice {
            accept: params["accept"].as_bool().unwrap(),
            prompt_text: if pending.kind == "prompt" { params.get("promptText").and_then(Value::as_str).unwrap_or("").into() } else { String::new() },
        };
        // Queue close and handle acknowledgement before resuming the original call.
        let closed = json!({"method":"Page.javascriptDialogClosed","sessionId":pending.session,
            "params":{"result":choice.accept,"userInput":if choice.accept { &choice.prompt_text } else { "" }}});
        let ack = json!({"id":id,"sessionId":pending.session,"result":{}});
        let delivered = output.send(closed.to_string()).is_ok() && output.send(ack.to_string()).is_ok();
        let result = if delivered { Ok(choice) } else { Err("dialog completion could not be delivered".into()) };
        let _ = pending.reply.send(result);
    }

    /// Cancel only the pending page's lifecycle operation; normal renderer
    /// dispatch still performs the actual close/navigation after JS unwinds.
    pub(crate) fn cancel_lifecycle(&self, method: &str, session: Option<&str>, params: &Value) {
        let mut state = self.0.lock().unwrap();
        let matches = state.pending.as_ref().is_some_and(|pending| match method {
            "Target.closeTarget" => params.get("targetId").and_then(Value::as_str) == Some(pending.page.as_str()),
            "Target.disposeBrowserContext" => params.get("browserContextId").and_then(Value::as_str) == Some(pending.context.as_str()),
            "Target.detachFromTarget" => params.get("sessionId").and_then(Value::as_str) == Some(pending.session.as_str()),
            "Page.navigate" | "Page.reload" | "Page.close" => session == Some(pending.session.as_str()),
            _ => false,
        });
        if matches {
            let pending = state.pending.take().unwrap();
            let _ = pending.reply.send(Err("dialog cancelled by page lifecycle change".into()));
        }
    }

    pub(crate) fn disconnect(&self) {
        let mut state = self.0.lock().unwrap();
        state.disconnected = true;
        if let Some(pending) = state.pending.take() {
            let _ = pending.reply.send(Err("dialog connection closed".into()));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request() -> DialogRequest {
        DialogRequest { kind:"confirm".into(), message:"Continue?".into(), default_prompt:"".into(), url:"http://localhost/".into() }
    }

    #[test]
    fn native_dialog_choice_preserves_order_and_refuses_foreign_session() {
        let broker = DialogBroker::default();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = broker.handler("page-1-session".into(), "page-1".into(), "context-1".into(), tx.clone(), Duration::from_secs(2));
        let worker = std::thread::spawn(move || handler(request()));
        let opening: Value = serde_json::from_str(&rx.blocking_recv().unwrap()).unwrap();
        assert_eq!(opening["method"], "Page.javascriptDialogOpening");
        broker.handle(2, Some("page-2-session"), &json!({"accept":true}), &tx);
        let rejected: Value = serde_json::from_str(&rx.blocking_recv().unwrap()).unwrap();
        assert!(rejected.get("error").is_some());
        broker.handle(3, Some("page-1-session"), &json!({"accept":false}), &tx);
        let closed: Value = serde_json::from_str(&rx.blocking_recv().unwrap()).unwrap();
        let ack: Value = serde_json::from_str(&rx.blocking_recv().unwrap()).unwrap();
        assert_eq!(closed["params"]["result"], false);
        assert_eq!(ack["id"], 3);
        assert!(!worker.join().unwrap().unwrap().accept);
        broker.handle(4, Some("page-1-session"), &json!({"accept":true}), &tx);
        let duplicate: Value = serde_json::from_str(&rx.blocking_recv().unwrap()).unwrap();
        assert!(duplicate.get("error").is_some());
    }

    #[test]
    fn native_dialog_disconnect_wakes_original_call_and_rejects_future_opening() {
        let broker = DialogBroker::default();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = broker.handler("s".into(), "page-1".into(), "context-1".into(), tx, Duration::from_secs(2));
        let next = handler.clone();
        let worker = std::thread::spawn(move || handler(request()));
        rx.blocking_recv().unwrap();
        broker.disconnect();
        assert!(worker.join().unwrap().is_err());
        assert!(next(request()).is_err());
    }

    #[test]
    fn native_dialog_lifecycle_cancels_only_the_owning_page_or_context() {
        for (method, field, matching) in [
            ("Target.closeTarget", "targetId", "page-1"),
            ("Target.disposeBrowserContext", "browserContextId", "context-1"),
            ("Target.detachFromTarget", "sessionId", "s"),
        ] {
            let broker = DialogBroker::default();
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let handler = broker.handler("s".into(), "page-1".into(), "context-1".into(), tx, Duration::from_secs(2));
            let worker = std::thread::spawn(move || handler(request()));
            rx.blocking_recv().unwrap();
            let mut params = json!({});
            params[field] = json!("foreign");
            broker.cancel_lifecycle(method, None, &params);
            assert!(broker.0.lock().unwrap().pending.is_some());
            params[field] = json!(matching);
            broker.cancel_lifecycle(method, None, &params);
            assert!(worker.join().unwrap().is_err());
            assert!(rx.try_recv().is_err(), "cancellation must not invent native close evidence");
        }
    }

    #[test]
    fn native_dialog_invalid_choices_do_not_consume_prompt_and_input_is_preserved() {
        let broker = DialogBroker::default();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = broker.handler("s".into(), "page-1".into(), "context-1".into(), tx.clone(), Duration::from_secs(2));
        let worker = std::thread::spawn(move || {
            let mut prompt = request(); prompt.kind = "prompt".into(); prompt.default_prompt = "old".into();
            handler(prompt)
        });
        let opening: Value = serde_json::from_str(&rx.blocking_recv().unwrap()).unwrap();
        assert_eq!(opening["params"]["defaultPrompt"], "old");
        for malformed in [json!({}), json!({"accept":"yes"}), json!({"accept":true,"promptText":7})] {
            broker.handle(1, Some("s"), &malformed, &tx);
            let error: Value = serde_json::from_str(&rx.blocking_recv().unwrap()).unwrap();
            assert!(error.get("error").is_some());
            assert!(broker.0.lock().unwrap().pending.is_some());
        }
        broker.handle(2, Some("s"), &json!({"accept":true,"promptText":"chosen"}), &tx);
        let closed: Value = serde_json::from_str(&rx.blocking_recv().unwrap()).unwrap();
        assert_eq!(closed["params"]["userInput"], "chosen");
        assert_eq!(worker.join().unwrap().unwrap().prompt_text, "chosen");
    }

    #[test]
    fn native_dialog_timeout_clears_pending_without_inventing_close() {
        let broker = DialogBroker::default();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let handler = broker.handler("s".into(), "page-1".into(), "context-1".into(), tx.clone(), Duration::from_millis(1));
        assert!(handler(request()).is_err());
        assert!(rx.try_recv().is_ok());
        assert!(rx.try_recv().is_err());
        broker.handle(1, Some("s"), &json!({"accept":true}), &tx);
        let reply: Value = serde_json::from_str(&rx.try_recv().unwrap()).unwrap();
        assert!(reply.get("error").is_some());
    }
}
