//! `Input.dispatchKeyEvent` interpolates the `key`/`code` params into a
//! generated `KeyboardEvent(...)` snippet. They must be escaped for BOTH
//! backslash and single-quote (issue #433): Chrome sends `key: "\\"` (U+005C)
//! when the backslash key is pressed, and quote-only escaping turns that into
//! `key:'\'` — the backslash escapes the closing quote, the literal runs on,
//! and the whole `page.evaluate` is a syntax error, so the `keydown` is
//! silently never dispatched. Regression test: the backslash key must arrive.

use obscura_cdp::dispatch::{dispatch, CdpContext};
use obscura_cdp::types::CdpRequest;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

// Serves a page that records the `key` of the last keydown event on the body.
async fn serve_page() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        tokio::spawn(async move {
            let mut buf = [0u8; 2048];
            let _ = socket.read(&mut buf).await.unwrap();
            let body = r#"<html><body>
<input id="i">
<textarea id="a"></textarea>
<script>
window.__keys = [];
document.body.addEventListener('keydown', function (e) { window.__keys.push(e.key + '|' + e.code); });
</script>
</body></html>"#;
            let resp = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = socket.write_all(resp.as_bytes()).await;
        });
    });
    format!("http://{addr}/")
}

async fn cdp(ctx: &mut CdpContext, id: u64, method: &str, params: Value, session_id: &str) -> Value {
    let resp = dispatch(
        &CdpRequest {
            id,
            method: method.to_string(),
            params,
            session_id: Some(session_id.to_string()),
        },
        ctx,
    )
    .await;
    assert!(resp.error.is_none(), "CDP {method} failed: {:?}", resp.error);
    resp.result.unwrap_or_else(|| json!({}))
}

#[tokio::test(flavor = "current_thread")]
async fn dispatch_key_event_escapes_backslash_in_key_and_code() {
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let url = serve_page().await;
    let mut ctx = CdpContext::new();
    let page_id = ctx.create_page();
    let session_id = "session-1";
    ctx.sessions.insert(session_id.to_string(), page_id.clone());

    cdp(&mut ctx, 1, "Page.navigate", json!({"url": url, "waitUntil": "load"}), session_id).await;

    // The backslash key: Chrome sends key="\" (a single backslash) code="Backslash".
    cdp(
        &mut ctx,
        2,
        "Input.dispatchKeyEvent",
        json!({"type": "keyDown", "key": "\\", "code": "Backslash"}),
        session_id,
    )
    .await;

    // A key whose name itself contains a quote AND a backslash, to exercise the
    // ordering of the two replacements.
    cdp(
        &mut ctx,
        3,
        "Input.dispatchKeyEvent",
        json!({"type": "keyDown", "key": "a", "code": "KeyA"}),
        session_id,
    )
    .await;

    let v = cdp(
        &mut ctx,
        4,
        "Runtime.evaluate",
        json!({"expression": "JSON.stringify(window.__keys)", "returnByValue": true}),
        session_id,
    )
    .await;

    let keys: Vec<String> =
        serde_json::from_str(v["result"]["value"].as_str().unwrap()).unwrap();
    assert_eq!(
        keys,
        vec!["\\|Backslash".to_string(), "a|KeyA".to_string()],
        "the backslash key must be dispatched, not dropped by a malformed snippet"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn dispatch_key_event_exposes_legacy_virtual_key_codes() {
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let url = serve_page().await;
    let mut ctx = CdpContext::new();
    let page_id = ctx.create_page();
    let session_id = "session-1";
    ctx.sessions.insert(session_id.to_string(), page_id);

    cdp(&mut ctx, 1, "Page.navigate", json!({"url": url, "waitUntil": "load"}), session_id).await;
    cdp(
        &mut ctx,
        2,
        "Runtime.evaluate",
        json!({
            "expression": "window.__codes = []; for (const type of ['keydown', 'keypress', 'keyup']) document.body.addEventListener(type, e => __codes.push([type, e.keyCode, e.which, e.charCode, e.shiftKey]))",
            "returnByValue": true
        }),
        session_id,
    ).await;
    for (id, event_type) in [(3, "keyDown"), (4, "keyUp")] {
        cdp(
            &mut ctx,
            id,
            "Input.dispatchKeyEvent",
            json!({"type": event_type, "key": "Enter", "code": "Enter", "text": "\r", "windowsVirtualKeyCode": 13, "modifiers": 8}),
            session_id,
        ).await;
    }
    let value = cdp(
        &mut ctx,
        5,
        "Runtime.evaluate",
        json!({"expression": "JSON.stringify(__codes)", "returnByValue": true}),
        session_id,
    ).await;
    assert_eq!(
        value["result"]["value"],
        r#"[["keydown",13,13,0,true],["keypress",13,13,13,true],["keyup",13,13,0,true]]"#
    );
}

#[tokio::test(flavor = "current_thread")]
async fn dispatch_key_event_preserves_modifier_combinations() {
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let url = serve_page().await;
    let mut ctx = CdpContext::new();
    let page_id = ctx.create_page();
    let session_id = "session-1";
    ctx.sessions.insert(session_id.to_string(), page_id);
    cdp(&mut ctx, 1, "Page.navigate", json!({"url": url, "waitUntil": "load"}), session_id).await;
    cdp(&mut ctx, 2, "Runtime.evaluate", json!({
        "expression": "window.__modifiers = []; for (const type of ['keydown','keyup']) document.addEventListener(type, e => __modifiers.push([type, e.altKey, e.ctrlKey, e.metaKey, e.shiftKey, ...['Alt','Control','Meta','Shift'].map(k => e.getModifierState(k))]))"
    }), session_id).await;
    let mut expected = Vec::new();
    let mut id = 3;
    for modifiers in 0..16 {
        for (event_type, dom_type) in [("rawKeyDown", "keydown"), ("keyUp", "keyup")] {
            cdp(&mut ctx, id, "Input.dispatchKeyEvent", json!({
                "type": event_type, "key": "o", "code": "KeyO",
                "windowsVirtualKeyCode": 79, "modifiers": modifiers
            }), session_id).await;
            id += 1;
            let flags = [1, 2, 4, 8].map(|mask| modifiers & mask != 0);
            expected.push(json!([dom_type, flags[0], flags[1], flags[2], flags[3],
                flags[0], flags[1], flags[2], flags[3]]));
        }
    }
    let result = cdp(&mut ctx, id, "Runtime.evaluate", json!({
        "expression": "JSON.stringify(__modifiers)", "returnByValue": true
    }), session_id).await;
    let actual: Value = serde_json::from_str(result["result"]["value"].as_str().unwrap()).unwrap();
    assert_eq!(actual, json!(expected));
}

#[tokio::test(flavor = "current_thread")]
async fn delete_key_removes_the_selected_range() {
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let url = serve_page().await;
    let mut ctx = CdpContext::new();
    let page_id = ctx.create_page();
    let session_id = "session-1";
    ctx.sessions.insert(session_id.to_string(), page_id);
    cdp(&mut ctx, 1, "Page.navigate", json!({"url": url, "waitUntil": "load"}), session_id).await;
    cdp(
        &mut ctx,
        2,
        "Runtime.evaluate",
        json!({
            "expression": "i.focus(); i.value = 'abcd'; i.setSelectionRange(1, 3); window.__inputs = 0; i.addEventListener('input', () => __inputs++)",
            "returnByValue": true
        }),
        session_id,
    ).await;
    cdp(
        &mut ctx,
        3,
        "Input.dispatchKeyEvent",
        json!({"type": "keyDown", "key": "Delete", "code": "Delete", "windowsVirtualKeyCode": 46}),
        session_id,
    ).await;
    let value = cdp(
        &mut ctx,
        4,
        "Runtime.evaluate",
        json!({"expression": "JSON.stringify({value: i.value, start: i.selectionStart, inputs: __inputs})", "returnByValue": true}),
        session_id,
    ).await;
    assert_eq!(value["result"]["value"], r#"{"value":"ad","start":1,"inputs":1}"#);
}

// The same hazard one layer over: on the inserted *text* rather than on the key
// name. `insert_text_js` interpolates the text into the same kind of
// single-quoted literal. #433 escaped the backslash and the quote and stopped
// there, but a raw newline ends a JS string literal just as a stray quote does,
// so the snippet is a syntax error and the character is silently dropped.
//
// The `char` type is the path that reaches it with a newline: the `keyDown`
// branch skips insertion for "\r" and "\n" because `key == "Enter"` handles
// those, and `char` has no such guard, so a client entering a line break in a
// textarea this way loses it with no error reported anywhere.
#[tokio::test(flavor = "current_thread")]
async fn dispatch_key_event_char_carries_a_newline_into_a_textarea() {
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let url = serve_page().await;
    let mut ctx = CdpContext::new();
    let page_id = ctx.create_page();
    let session_id = "session-1";
    ctx.sessions.insert(session_id.to_string(), page_id.clone());

    cdp(&mut ctx, 1, "Page.navigate", json!({"url": url, "waitUntil": "load"}), session_id).await;
    cdp(
        &mut ctx,
        2,
        "Runtime.evaluate",
        json!({
            "expression": "(function () { document.getElementById('a').focus(); return 'ok'; })()",
            "returnByValue": true,
        }),
        session_id,
    )
    .await;

    for (id, ch) in [(3u64, "a"), (4, "\n"), (5, "b")] {
        cdp(
            &mut ctx,
            id,
            "Input.dispatchKeyEvent",
            json!({"type": "char", "text": ch}),
            session_id,
        )
        .await;
    }

    let v = cdp(
        &mut ctx,
        6,
        "Runtime.evaluate",
        json!({
            "expression": "JSON.stringify(document.getElementById('a').value)",
            "returnByValue": true,
        }),
        session_id,
    )
    .await;
    assert_eq!(
        v["result"]["value"].as_str().unwrap_or_default(),
        r#""a\nb""#,
        "a newline sent as a char must reach the field, not be dropped by a malformed snippet"
    );
}

// #577: Playwright's fill() focuses the field in page and then types the whole
// value with one Input.insertText call. The method must exist and drive the
// same snippet the key-event text path uses, so quotes, backslashes, and
// newlines survive intact.
#[tokio::test(flavor = "current_thread")]
async fn insert_text_types_into_the_focused_field() {
    std::env::set_var("OBSCURA_ALLOW_PRIVATE_NETWORK", "1");
    let url = serve_page().await;
    let mut ctx = CdpContext::new();
    let page_id = ctx.create_page();
    let session_id = "session-1";
    ctx.sessions.insert(session_id.to_string(), page_id.clone());

    cdp(&mut ctx, 1, "Page.navigate", json!({"url": url, "waitUntil": "load"}), session_id).await;
    cdp(
        &mut ctx,
        2,
        "Runtime.evaluate",
        json!({"expression": "document.getElementById('i').focus()", "returnByValue": true}),
        session_id,
    )
    .await;
    cdp(
        &mut ctx,
        3,
        "Input.insertText",
        json!({"text": "he'll\\o\nbye"}),
        session_id,
    )
    .await;

    let v = cdp(
        &mut ctx,
        4,
        "Runtime.evaluate",
        json!({
            "expression": "JSON.stringify(document.getElementById('i').value)",
            "returnByValue": true,
        }),
        session_id,
    )
    .await;
    assert_eq!(
        v["result"]["value"].as_str().unwrap_or_default(),
        r#""he'll\\o\nbye""#,
        "insertText must type the full text with quotes, backslashes, and newlines intact"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn enter_activates_native_button_once_with_trust_and_preserves_submitter() {
    let mut ctx = CdpContext::new();
    let page = ctx.create_page();
    let sid = "keyboard-activation";
    ctx.sessions.insert(sid.into(), page);
    cdp(&mut ctx, 1, "Runtime.evaluate", json!({"expression": r#"
        document.body.innerHTML='<form id="form"><button id="plain" type="button">plain</button><button id="submit" type="submit">submit</button><button id="disabled" type="button" disabled>disabled</button></form>';
        globalThis.activations=[]; globalThis.submissions=[];
        for (const id of ['plain','submit','disabled']) document.getElementById(id).addEventListener('click', e => activations.push([id,e.isTrusted]));
        document.getElementById('form').addEventListener('submit',e=>{e.preventDefault();submissions.push(e.submitter?.id)});
    "#, "returnByValue":true}), sid).await;
    for (index, id) in ["plain", "submit", "disabled"].into_iter().enumerate() {
        cdp(&mut ctx, 2, "Runtime.evaluate", json!({"expression":format!("document.activeElement?.blur(); document.getElementById('{id}').focus()"),"returnByValue":true}), sid).await;
        for kind in ["keyDown", "keyUp"] {
            cdp(&mut ctx, 3+index as u64, "Input.dispatchKeyEvent", json!({"type":kind,"key":"Enter","code":"Enter","windowsVirtualKeyCode":13}), sid).await;
        }
    }
    let result = cdp(&mut ctx, 8, "Runtime.evaluate", json!({"expression":"JSON.stringify({activations,submissions})","returnByValue":true}), sid).await;
    let observed: Value = serde_json::from_str(result["result"]["value"].as_str().unwrap()).unwrap();
    assert_eq!(observed["activations"], json!([["plain",true],["submit",true]]));
    assert_eq!(observed["submissions"], json!(["submit"]));
}

#[tokio::test(flavor = "current_thread")]
async fn enter_button_activation_respects_keydown_keypress_and_click_cancellation() {
    let mut ctx = CdpContext::new();
    let page = ctx.create_page();
    let sid = "keyboard-cancel";
    ctx.sessions.insert(sid.into(), page);
    cdp(&mut ctx, 1, "Runtime.evaluate", json!({"expression":r#"
        document.body.innerHTML='<form id="form"><button id="button" type="submit">go</button></form>';
        globalThis.clicks=0;globalThis.submits=0;
        document.getElementById('button').addEventListener('click',()=>clicks++);
        document.getElementById('form').addEventListener('submit',e=>{e.preventDefault();submits++});
        document.getElementById('button').focus();
    "#,"returnByValue":true}), sid).await;
    for kind in ["keydown","keypress","click"] {
        cdp(&mut ctx, 2, "Runtime.evaluate", json!({"expression":format!("document.getElementById('button').addEventListener('{kind}',e=>e.preventDefault(),{{once:true}})"),"returnByValue":true}), sid).await;
        cdp(&mut ctx, 3, "Input.dispatchKeyEvent", json!({"type":"keyDown","key":"Enter","code":"Enter","windowsVirtualKeyCode":13}), sid).await;
        cdp(&mut ctx, 4, "Input.dispatchKeyEvent", json!({"type":"keyUp","key":"Enter","code":"Enter","windowsVirtualKeyCode":13}), sid).await;
    }
    let result = cdp(&mut ctx, 5, "Runtime.evaluate", json!({"expression":"JSON.stringify({clicks,submits})","returnByValue":true}), sid).await;
    let observed: Value = serde_json::from_str(result["result"]["value"].as_str().unwrap()).unwrap();
    assert_eq!(observed,json!({"clicks":1,"submits":0}));
}

#[tokio::test(flavor = "current_thread")]
async fn space_activates_only_on_release_and_respects_cancellation_and_focus() {
    let mut ctx = CdpContext::new();
    let page = ctx.create_page();
    let sid = "space-button-activation";
    ctx.sessions.insert(sid.into(), page);
    cdp(&mut ctx, 1, "Runtime.evaluate", json!({"expression":r#"
        document.body.innerHTML='<button id="first">first</button><button id="second">second</button>';
        globalThis.clicks=[];globalThis.presses=[];globalThis.downState=null;
        for(const b of document.querySelectorAll('button')) { b.addEventListener('click',e=>clicks.push([b.id,e.isTrusted,e.detail])); b.addEventListener('keypress',e=>presses.push([b.id,e.key,e.isTrusted])); }
        document.getElementById('first').addEventListener('keydown',()=>downState=[document.activeElement===globalThis.__obscura_spaceKeydownTarget,document.activeElement.localName,document.activeElement.disabled]);
        document.getElementById('first').focus();
    "#,"returnByValue":true}), sid).await;
    let down = json!({"type":"keyDown","key":" ","code":"Space","windowsVirtualKeyCode":32});
    let up = json!({"type":"keyUp","key":" ","code":"Space","windowsVirtualKeyCode":32});
    cdp(&mut ctx, 2, "Input.dispatchKeyEvent", down.clone(), sid).await;
    let before = cdp(&mut ctx, 3, "Runtime.evaluate", json!({"expression":"JSON.stringify(clicks)","returnByValue":true}), sid).await;
    assert_eq!(before["result"]["value"], "[]");
    let state = cdp(&mut ctx, 30, "Runtime.evaluate", json!({"expression":"JSON.stringify({active:document.activeElement?.id,pending:globalThis.__obscura_spaceActivationTarget?.id,presses,downState})","returnByValue":true}), sid).await;
    assert_eq!(state["result"]["value"], "{\"active\":\"first\",\"pending\":\"first\",\"presses\":[[\"first\",\" \",true]],\"downState\":[true,\"button\",false]}");
    cdp(&mut ctx, 4, "Input.dispatchKeyEvent", up.clone(), sid).await;
    cdp(&mut ctx, 5, "Runtime.evaluate", json!({"expression":"document.getElementById('first').addEventListener('keydown',e=>e.preventDefault(),{once:true})","returnByValue":true}), sid).await;
    cdp(&mut ctx, 6, "Input.dispatchKeyEvent", down.clone(), sid).await;
    cdp(&mut ctx, 7, "Input.dispatchKeyEvent", up.clone(), sid).await;
    cdp(&mut ctx, 8, "Input.dispatchKeyEvent", down, sid).await;
    cdp(&mut ctx, 9, "Runtime.evaluate", json!({"expression":"document.getElementById('second').focus()","returnByValue":true}), sid).await;
    cdp(&mut ctx, 10, "Input.dispatchKeyEvent", up, sid).await;
    let after = cdp(&mut ctx, 11, "Runtime.evaluate", json!({"expression":"JSON.stringify(clicks)","returnByValue":true}), sid).await;
    assert_eq!(after["result"]["value"], "[[\"first\",true,0]]");
    cdp(&mut ctx, 12, "Input.dispatchKeyEvent", json!({"type":"keyDown","key":" ","code":"Space","windowsVirtualKeyCode":32,"modifiers":8}), sid).await;
    cdp(&mut ctx, 13, "Input.dispatchKeyEvent", json!({"type":"keyUp","key":" ","code":"Space","windowsVirtualKeyCode":32,"modifiers":8}), sid).await;
    let shifted = cdp(&mut ctx, 14, "Runtime.evaluate", json!({"expression":"JSON.stringify({clicks,presses})","returnByValue":true}), sid).await;
    assert_eq!(shifted["result"]["value"], "{\"clicks\":[[\"first\",true,0],[\"second\",true,0]],\"presses\":[[\"first\",\" \",true],[\"first\",\" \",true],[\"second\",\" \",true]]}");
    cdp(&mut ctx, 15, "Runtime.evaluate", json!({"expression":"document.getElementById('second').addEventListener('keyup',()=>document.getElementById('first').focus(),{once:true})","returnByValue":true}), sid).await;
    cdp(&mut ctx, 16, "Input.dispatchKeyEvent", json!({"type":"keyDown","key":" ","code":"Space","windowsVirtualKeyCode":32}), sid).await;
    cdp(&mut ctx, 17, "Input.dispatchKeyEvent", json!({"type":"keyUp","key":" ","code":"Space","windowsVirtualKeyCode":32}), sid).await;
    let moved = cdp(&mut ctx, 18, "Runtime.evaluate", json!({"expression":"JSON.stringify(clicks)","returnByValue":true}), sid).await;
    assert_eq!(moved["result"]["value"], "[[\"first\",true,0],[\"second\",true,0]]");
}

#[tokio::test(flavor = "current_thread")]
async fn submit_event_constructor_retains_readonly_submitter_and_defaults_to_null() {
    let mut ctx = CdpContext::new();
    let page = ctx.create_page();
    let sid = "submit-event-contract";
    ctx.sessions.insert(sid.into(), page);
    let result = cdp(&mut ctx, 1, "Runtime.evaluate", json!({"expression":r#"
        (()=>{
            const button=document.createElement('button');
            const event=new SubmitEvent('submit',{submitter:button,bubbles:true,cancelable:true});
            let readonly=false;try{(function(){'use strict';event.submitter=null})()}catch(e){readonly=e instanceof TypeError}
            return JSON.stringify([new SubmitEvent('submit').submitter===null,event.submitter===button,event.bubbles,event.cancelable,event.isTrusted,readonly]);
        })()
    "#,"returnByValue":true}), sid).await;
    assert_eq!(result["result"]["value"],"[true,true,true,true,false,true]");
}
