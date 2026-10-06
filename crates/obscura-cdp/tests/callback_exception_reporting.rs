// Errors thrown by callbacks the page hands the platform (timers, event
// listeners) are "reported": an ErrorEvent at the global and, unless it is
// canceled, Runtime.exceptionThrown. They used to be swallowed into
// console.error, invisible to window.onerror and to automation.

use obscura_cdp::dispatch::{dispatch, CdpContext};
use obscura_cdp::types::CdpRequest;
use serde_json::{json, Value};

async fn cdp(ctx: &mut CdpContext, method: &str, params: Value, session: &str) -> Value {
    let response = dispatch(
        &CdpRequest {
            id: 1,
            method: method.to_string(),
            params,
            session_id: Some(session.to_string()),
        },
        ctx,
    )
    .await;
    assert!(response.error.is_none(), "{method}: {:?}", response.error);
    response.result.unwrap_or_else(|| json!({}))
}

async fn evaluate(ctx: &mut CdpContext, session: &str, expression: &str) -> Value {
    let result = cdp(
        ctx,
        "Runtime.evaluate",
        json!({"expression": expression, "returnByValue": true, "awaitPromise": true}),
        session,
    )
    .await;
    assert!(result.get("exceptionDetails").is_none(), "{result}");
    result["result"]["value"].clone()
}

fn reported(ctx: &CdpContext, needle: &str) -> usize {
    ctx.pending_events
        .iter()
        .filter(|event| event.method == "Runtime.exceptionThrown")
        .filter(|event| event.params.to_string().contains(needle))
        .count()
}

async fn setup() -> (CdpContext, String) {
    let mut ctx = CdpContext::new();
    let page_id = ctx.create_page();
    let session = "callback-errors".to_string();
    ctx.sessions.insert(session.clone(), page_id);
    cdp(&mut ctx, "Runtime.enable", json!({}), &session).await;
    let url = concat!(
        "data:text/html,<button id=b>b</button><script>",
        "window.__errors = [];",
        "addEventListener('error', function (e) {",
        "  __errors.push(e.message);",
        "  if (e.message === 'quiet') e.preventDefault();",
        "  if (e.message === 'recursive') throw new Error('from-error-listener');",
        "});",
        "b.addEventListener('poke', function () { throw new Error('from-listener'); });",
        "</script>"
    );
    cdp(&mut ctx, "Page.navigate", json!({"url": url, "waitUntil": "load"}), &session).await;
    ctx.pending_events.clear();
    (ctx, session)
}

const SETTLE: &str = "new Promise(function (resolve) { setTimeout(function () { resolve(__errors); }, 50); })";

#[tokio::test(flavor = "current_thread")]
async fn timer_errors_reach_window_onerror_and_runtime_exception_thrown() {
    let (mut ctx, session) = setup().await;
    evaluate(&mut ctx, &session, "setTimeout(function () { throw new Error('from-timer'); }, 0); 0").await;
    let errors = evaluate(&mut ctx, &session, SETTLE).await;
    assert!(errors.to_string().contains("from-timer"), "window error events: {errors}");
    assert_eq!(reported(&ctx, "from-timer"), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn interval_errors_are_reported_and_the_interval_keeps_running() {
    let (mut ctx, session) = setup().await;
    evaluate(
        &mut ctx,
        &session,
        "window.__ticks = 0; window.__interval = setInterval(function () { __ticks++; if (__ticks >= 2) clearInterval(__interval); throw new Error('from-interval'); }, 0); 0",
    )
    .await;
    evaluate(&mut ctx, &session, SETTLE).await;
    assert_eq!(evaluate(&mut ctx, &session, "__ticks").await.as_f64(), Some(2.0));
    assert_eq!(reported(&ctx, "from-interval"), 2);
}

#[tokio::test(flavor = "current_thread")]
async fn event_listener_errors_are_reported_and_dispatch_continues() {
    let (mut ctx, session) = setup().await;
    let returned = evaluate(&mut ctx, &session, "b.dispatchEvent(new Event('poke'))").await;
    assert_eq!(returned, json!(true), "dispatchEvent must still return");
    let errors = evaluate(&mut ctx, &session, "__errors").await;
    assert!(errors.to_string().contains("from-listener"), "window error events: {errors}");
    assert_eq!(reported(&ctx, "from-listener"), 1);
}

#[tokio::test(flavor = "current_thread")]
async fn a_canceled_error_event_is_not_reported_to_cdp() {
    let (mut ctx, session) = setup().await;
    evaluate(&mut ctx, &session, "setTimeout(function () { throw new Error('quiet'); }, 0); 0").await;
    let errors = evaluate(&mut ctx, &session, SETTLE).await;
    assert!(errors.to_string().contains("quiet"), "window error events: {errors}");
    assert_eq!(reported(&ctx, "quiet"), 0);
}

#[tokio::test(flavor = "current_thread")]
async fn a_throwing_error_listener_does_not_recurse() {
    let (mut ctx, session) = setup().await;
    evaluate(&mut ctx, &session, "setTimeout(function () { throw new Error('recursive'); }, 0); 0").await;
    let errors = evaluate(&mut ctx, &session, SETTLE).await;
    let seen: Vec<&str> = errors.as_array().unwrap().iter().filter_map(Value::as_str).collect();
    assert_eq!(seen.iter().filter(|m| **m == "recursive").count(), 1, "{seen:?}");
    assert!(!seen.contains(&"from-error-listener"), "the listener's own error must not be re-dispatched: {seen:?}");
    assert_eq!(reported(&ctx, "recursive"), 1);
}
