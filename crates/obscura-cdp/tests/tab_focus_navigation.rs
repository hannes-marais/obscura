// Tab's default action is sequential focus navigation (HTML §6.6.3): positive
// tabIndex first in ascending order, then tabIndex 0 in document order,
// skipping disabled, hidden and tabindex=-1 elements; Shift reverses; a
// canceled keydown moves nothing.

use obscura_cdp::dispatch::{dispatch, CdpContext};
use obscura_cdp::types::CdpRequest;
use serde_json::{json, Value};

async fn cdp(ctx: &mut CdpContext, method: &str, params: Value) -> Value {
    let response = dispatch(
        &CdpRequest {
            id: 1,
            method: method.to_string(),
            params,
            session_id: Some("tab".to_string()),
        },
        ctx,
    )
    .await;
    assert!(response.error.is_none(), "{method}: {:?}", response.error);
    response.result.unwrap_or_else(|| json!({}))
}

async fn evaluate(ctx: &mut CdpContext, expression: &str) -> Value {
    let result = cdp(
        ctx,
        "Runtime.evaluate",
        json!({"expression": expression, "returnByValue": true}),
    )
    .await;
    assert!(result.get("exceptionDetails").is_none(), "{result}");
    result["result"]["value"].clone()
}

async fn setup() -> CdpContext {
    let mut ctx = CdpContext::new();
    let page_id = ctx.create_page();
    ctx.sessions.insert("tab".to_string(), page_id);
    let url = concat!(
        "data:text/html,",
        "<input id=a>",
        "<input id=hidden type=hidden>",
        "<button id=off disabled>off</button>",
        "<span id=skip tabindex=-1>skip</span>",
        "<div id=first tabindex=2>first</div>",
        "<button id=b>b</button>",
        "<a id=nohref>no href</a>",
        "<a id=link href=x>link</a>"
    );
    cdp(&mut ctx, "Page.navigate", json!({"url": url, "waitUntil": "load"})).await;
    ctx
}

async fn tab(ctx: &mut CdpContext, shift: bool) {
    let modifiers = if shift { 8 } else { 0 };
    for kind in ["keyDown", "keyUp"] {
        cdp(
            ctx,
            "Input.dispatchKeyEvent",
            json!({"type": kind, "key": "Tab", "code": "Tab", "windowsVirtualKeyCode": 9, "modifiers": modifiers}),
        )
        .await;
    }
}

async fn focused(ctx: &mut CdpContext) -> Value {
    evaluate(ctx, "document.activeElement && document.activeElement.id").await
}

#[tokio::test(flavor = "current_thread")]
async fn tab_walks_positive_tabindex_first_then_document_order_and_wraps() {
    let mut ctx = setup().await;
    evaluate(&mut ctx, "document.activeElement && document.activeElement.blur && document.activeElement.blur()").await;
    let mut seen = Vec::new();
    for _ in 0..5 {
        tab(&mut ctx, false).await;
        seen.push(focused(&mut ctx).await);
    }
    assert_eq!(seen, vec![json!("first"), json!("a"), json!("b"), json!("link"), json!("first")]);
}

#[tokio::test(flavor = "current_thread")]
async fn shift_tab_walks_backwards() {
    let mut ctx = setup().await;
    evaluate(&mut ctx, "a.focus()").await;
    tab(&mut ctx, true).await;
    assert_eq!(focused(&mut ctx).await, json!("first"));
    tab(&mut ctx, true).await;
    assert_eq!(focused(&mut ctx).await, json!("link"), "wraps to the end");
}

#[tokio::test(flavor = "current_thread")]
async fn tab_fires_focus_events_on_both_elements() {
    let mut ctx = setup().await;
    evaluate(
        &mut ctx,
        "window.__events = []; for (const id of ['a', 'b']) for (const t of ['focus', 'blur']) document.getElementById(id).addEventListener(t, () => __events.push(id + ':' + t)); a.focus(); __events.length = 0;",
    )
    .await;
    tab(&mut ctx, false).await;
    assert_eq!(evaluate(&mut ctx, "__events").await, json!(["a:blur", "b:focus"]));
}

#[tokio::test(flavor = "current_thread")]
async fn a_canceled_tab_keydown_does_not_move_focus() {
    let mut ctx = setup().await;
    evaluate(
        &mut ctx,
        "a.addEventListener('keydown', e => { if (e.key === 'Tab') e.preventDefault(); }); a.focus();",
    )
    .await;
    tab(&mut ctx, false).await;
    assert_eq!(focused(&mut ctx).await, json!("a"));
}
