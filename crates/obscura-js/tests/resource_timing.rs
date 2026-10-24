use std::sync::Arc;
use obscura_js::runtime::ObscuraJsRuntime;
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn fixture(replies: Vec<(u16, &'static str, Vec<u8>)>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(async move {
        for (status, extra, body) in replies {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0u8; 4096];
            socket.read(&mut request).await.unwrap();
            socket.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{extra}\r\n", body.len()).as_bytes()).await.unwrap();
            // Headers are observable before the real response body completes.
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            socket.write_all(&body).await.unwrap();
        }
    });
    base
}

fn runtime(base: &str) -> ObscuraJsRuntime {
    let mut rt = ObscuraJsRuntime::new();
    rt.set_dom(obscura_dom::parse_html("<html><body></body></html>"));
    rt.set_url(&format!("{base}/page"));
    rt.set_http_client(Arc::new(obscura_net::ObscuraHttpClient::with_full_options(
        Arc::new(obscura_net::CookieJar::new()), None, true,
    )));
    rt.run_page_init();
    rt
}

async fn finish(rt: &mut ObscuraJsRuntime) {
    assert!(rt.resolve_promises_until(|rt| rt.evaluate("globalThis.done === true").unwrap() == json!(true), 4000).await);
    assert_eq!(rt.evaluate("globalThis.failure || null").unwrap(), json!(null));
    rt.run_event_loop_bounded(40).await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_records_real_completion_and_observer_entries() {
    let base = fixture(vec![(201, "", b"actual bytes".to_vec())]).await;
    let mut rt = runtime(&base);
    rt.execute_script("timing-fetch", &format!(r#"
        globalThis.done = false; globalThis.observed = [];
        new PerformanceObserver(list => observed.push(...list.getEntries().map(e => e.toJSON())))
            .observe({{type: 'resource'}});
        performance.mark('retained-mark');
        (async () => {{
            const response = await fetch({base:?} + '/dossier');
            globalThis.beforeBody = performance.getEntriesByType('resource').length;
            globalThis.body = await response.text();
            globalThis.entry = performance.getEntriesByType('resource')[0];
            done = true;
        }})().catch(e => {{ globalThis.failure = String(e); done = true; }});
    "#)).unwrap();
    finish(&mut rt).await;
    assert_eq!(rt.evaluate(r#"(() => [beforeBody, body, entry.name.endsWith('/dossier'),
        entry.initiatorType, entry.responseStatus, entry.encodedBodySize, entry.decodedBodySize,
        entry.transferSize, entry.responseEnd > entry.responseStart,
        entry.responseStart >= entry.startTime, entry.duration === entry.responseEnd - entry.startTime,
        entry instanceof PerformanceResourceTiming, entry instanceof PerformanceEntry,
        Object.keys(entry).length, entry.domainLookupStart, entry.secureConnectionStart,
        observed.length, observed[0].responseStatus,
        performance.getEntriesByName(entry.name, 'resource').length,
        PerformanceObserver.supportedEntryTypes.includes('resource')])()"#).unwrap(),
        json!([0, "actual bytes", true, "fetch", 201, 12, 12, 312, true, true, true,
            true, true, 0, 0, 0, 1, 201, 1, true]));
    assert_eq!(rt.evaluate(r#"(() => {
        performance.clearResourceTimings();
        let error; try { new PerformanceResourceTiming(); } catch(e) { error = e.name; }
        return [performance.getEntriesByType('resource').length,
            performance.getEntriesByType('mark').length, error];
    })()"#).unwrap(), json!([0, 1, "TypeError"]));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_counts_encoded_bytes_and_xhr_http_errors() {
    let gzip = "1f8b08000000000002ff2bc9cccdcc4b572819a586260500f1677809c0010000";
    let wire: Vec<u8> = gzip.as_bytes().chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()).collect();
    let base = fixture(vec![(200, "Content-Encoding: gzip\r\n", wire), (503, "", b"unavailable".to_vec())]).await;
    let mut rt = runtime(&base);
    rt.execute_script("timing-compressed-xhr", &format!(r#"
        globalThis.done = false;
        (async () => {{
            const response = await fetch({base:?} + '/gzip');
            globalThis.decoded = await response.text();
            await new Promise((resolve, reject) => {{
                const xhr = new XMLHttpRequest(); xhr.open('GET', {base:?} + '/error');
                xhr.onload = resolve; xhr.onerror = reject; xhr.send();
            }});
            done = true;
        }})().catch(e => {{ globalThis.failure = String(e); done = true; }});
    "#)).unwrap();
    finish(&mut rt).await;
    assert_eq!(rt.evaluate(r#"(() => {
        const entries = performance.getEntriesByType('resource');
        return [decoded === 'timing '.repeat(64), entries.map(e => [e.initiatorType,
            e.responseStatus, e.encodedBodySize, e.decodedBodySize, e.transferSize])];
    })()"#).unwrap(), json!([true, [["fetch", 200, 32, 448, 332], ["xmlhttprequest", 503, 11, 11, 311]]]));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_requires_timing_allow_origin_for_cross_origin_details() {
    let base = fixture(vec![
        (200, "Access-Control-Allow-Origin: *\r\n", b"hidden size".to_vec()),
        (200, "Access-Control-Allow-Origin: *\r\nTiming-Allow-Origin: *\r\n", b"exposed".to_vec()),
        (200, "", b"opaque".to_vec()),
    ]).await;
    let mut rt = runtime("https://page.example");
    rt.execute_script("timing-origin", &format!(r#"
        globalThis.done = false;
        (async () => {{
            await (await fetch({base:?} + '/hidden')).text();
            await (await fetch({base:?} + '/allowed')).text();
            await fetch({base:?} + '/opaque', {{mode: 'no-cors'}});
            // Wait for the opaque network body, not a synthetic body read.
            while (performance.getEntriesByType('resource').length < 3)
                await new Promise(resolve => setTimeout(resolve, 1));
            done = true;
        }})().catch(e => {{ globalThis.failure = String(e); done = true; }});
    "#)).unwrap();
    finish(&mut rt).await;
    assert_eq!(rt.evaluate(r#"performance.getEntriesByType('resource').map(e =>
        [e.encodedBodySize, e.decodedBodySize, e.transferSize, e.responseStatus,
         e.responseStart === 0, e.responseEnd > e.startTime])"#).unwrap(),
        json!([[0, 0, 0, 200, true, true], [7, 7, 307, 200, false, true], [0, 0, 0, 0, true, true]]));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_buffer_full_handler_can_extend_storage() {
    let base = fixture((0..4).map(|_| (200, "", b"body".to_vec())).collect()).await;
    let mut rt = runtime(&base);
    rt.execute_script("timing-buffer", &format!(r#"
        globalThis.done = false; globalThis.full = 0; globalThis.observed = [];
        performance.setResourceTimingBufferSize(1);
        performance.onresourcetimingbufferfull = e => {{
            if (e.target !== performance) throw Error('wrong event target');
            full++; performance.setResourceTimingBufferSize(4);
        }};
        new PerformanceObserver(list => observed.push(...list.getEntries().map(e => e.name)))
            .observe({{type: 'resource', buffered: true}});
        (async () => {{
            for (let i = 0; i < 4; i++) await (await fetch({base:?} + '/' + i)).text();
            done = true;
        }})().catch(e => {{ globalThis.failure = String(e); done = true; }});
    "#)).unwrap();
    finish(&mut rt).await;
    assert_eq!(rt.evaluate("[full, performance.getEntriesByType('resource').length, observed.length]").unwrap(), json!([1, 4, 4]));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_failed_transport_never_reports_http_success() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let mut rt = runtime(&base);
    rt.execute_script("timing-failure", &format!(r#"
        globalThis.done = false;
        fetch({base:?} + '/refused').then(() => {{ failure = 'unexpected success'; done = true; }},
            () => {{ done = true; }});
    "#)).unwrap();
    finish(&mut rt).await;
    assert_eq!(rt.evaluate("performance.getEntriesByType('resource').map(e => [e.responseStatus, e.transferSize, e.decodedBodySize])").unwrap(), json!([[0, 0, 0]]));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_cancelled_body_records_failure_and_closes_transport() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.unwrap();
        let mut request = [0u8; 4096];
        socket.read(&mut request).await.unwrap();
        socket.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\nConnection: close\r\n\r\n").await.unwrap();
        tokio::time::timeout(std::time::Duration::from_secs(3), socket.read(&mut request)).await.unwrap().unwrap()
    });
    let mut rt = runtime(&base);
    rt.execute_script("timing-cancel", &format!(r#"
        globalThis.done = false; globalThis.headers = false;
        globalThis.controller = new AbortController();
        fetch({base:?} + '/cancel', {{signal: controller.signal}}).then(response => {{
            headers = true; return response.text();
        }}).then(() => {{ globalThis.failure = 'unexpected body'; done = true; }},
            error => {{ globalThis.outcome = error.name; done = true; }});
    "#)).unwrap();
    assert!(rt.resolve_promises_until(|rt| rt.evaluate("headers").unwrap() == json!(true), 2000).await);
    rt.execute_script("abort", "controller.abort();").unwrap();
    finish(&mut rt).await;
    assert_eq!(server.await.unwrap(), 0, "native cancellation must close the socket");
    assert_eq!(rt.evaluate("[outcome, performance.getEntriesByType('resource').map(e => [e.responseStatus, e.transferSize, e.decodedBodySize])]").unwrap(), json!(["AbortError", [[0, 0, 0]]]));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_zero_buffer_does_not_suppress_observer_delivery() {
    let base = fixture((0..3).map(|_| (200, "", b"body".to_vec())).collect()).await;
    let mut rt = runtime(&base);
    rt.execute_script("timing-zero-buffer", &format!(r#"
        globalThis.done = false; globalThis.full = 0; globalThis.observed = [];
        performance.setResourceTimingBufferSize(0);
        performance.addEventListener('resourcetimingbufferfull', () => full++);
        new PerformanceObserver(list => observed.push(...list.getEntries().map(e => e.name)))
            .observe({{type: 'resource'}});
        performance.mark('keep');
        (async () => {{
            for (let i = 0; i < 3; i++) await (await fetch({base:?} + '/' + i)).text();
            done = true;
        }})().catch(e => {{ globalThis.failure = String(e); done = true; }});
    "#)).unwrap();
    finish(&mut rt).await;
    assert_eq!(rt.evaluate("[full > 0, performance.getEntriesByType('resource').length, observed.length, performance.getEntriesByType('mark').length]").unwrap(), json!([true, 0, 3, 1]));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_internal_cross_origin_script_keeps_opaque_status_private() {
    let base = fixture(vec![(200, "", b"globalThis.crossLoaded = true;".to_vec())]).await;
    let mut rt = runtime("https://page.example");
    rt.execute_script("timing-cross-script", &format!(r#"
        globalThis.done = false;
        const script = document.createElement('script');
        script.onload = () => {{ done = true; }};
        script.onerror = () => {{ globalThis.failure = 'script failed'; done = true; }};
        script.src = {base:?} + '/classic.js'; document.body.appendChild(script);
    "#)).unwrap();
    finish(&mut rt).await;
    assert_eq!(rt.evaluate("[crossLoaded, performance.getEntriesByType('resource').map(e => [e.initiatorType, e.responseStatus, e.decodedBodySize])]").unwrap(), json!([true, [["script", 0, 0]]]));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_internal_failed_script_notifies_observer_without_a_timeline_read() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    let mut rt = runtime(&base);
    rt.execute_script("timing-failed-script", &format!(r#"
        globalThis.done = false; globalThis.observed = [];
        new PerformanceObserver(list => observed.push(...list.getEntries().map(e => [e.initiatorType, e.responseStatus])))
            .observe({{type: 'resource'}});
        const script = document.createElement('script');
        script.onload = () => {{ globalThis.failure = 'unexpected script'; done = true; }};
        script.onerror = () => {{ done = true; }};
        script.src = {base:?} + '/missing.js'; document.body.appendChild(script);
    "#)).unwrap();
    finish(&mut rt).await;
    assert_eq!(rt.evaluate("observed").unwrap(), json!([["script", 0]]));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_module_uses_wire_bytes_and_real_header_body_boundaries() {
    let wire: Vec<u8> = "1f8b08000000000002ffedc6b10980301040d1de29d2a533038833d8b840e21d2a9c9c1814dd5ee790ffaa379b976ce3b2d67673394d8752f5b854421fa2e8e4f275cf8f7996d83529dd0000000000e0f75e81f2db6a31100000".as_bytes().chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap()).collect();
    let wire_size = wire.len();
    let base = fixture(vec![
        (200, "Content-Type: application/javascript\r\n", b"import './child.js';".to_vec()),
        (200, "Content-Type: application/javascript\r\nContent-Encoding: gzip\r\n", wire),
    ]).await;
    let mut rt = ObscuraJsRuntime::with_base_url(&format!("{base}/page"));
    rt.set_url(&format!("{base}/page"));
    rt.set_dom(obscura_dom::parse_html("<html><body></body></html>"));
    rt.set_http_client(Arc::new(obscura_net::ObscuraHttpClient::with_full_options(
        Arc::new(obscura_net::CookieJar::new()), None, true,
    )));
    rt.run_page_init();
    rt.load_module(&format!("{base}/entry.js"), 4000).await.unwrap();
    assert_eq!(rt.evaluate("moduleObserved").unwrap(), json!("decoded payload"));
    let entries = rt.evaluate("performance.getEntriesByType('resource').map(e => e.toJSON())").unwrap();
    let entry = entries.as_array().unwrap().iter().find(|entry| entry["name"].as_str().unwrap().ends_with("/child.js")).unwrap();
    assert_eq!(entries.as_array().unwrap().len(), 2);
    assert_eq!(entry["initiatorType"], json!("script"));
    assert_eq!(entry["encodedBodySize"], json!(wire_size));
    assert_eq!(entry["decodedBodySize"], json!(4145));
    assert_eq!(entry["transferSize"], json!(wire_size + 300));
    assert!(entry["responseEnd"].as_f64().unwrap() > entry["responseStart"].as_f64().unwrap());
    let events = rt.take_js_network_events();
    assert_eq!(events.len(), 2);
    let child = events.iter().find(|event| event.url.ends_with("/child.js")).unwrap();
    assert_eq!(child.encoded_data_length, Some(wire_size));
    assert_eq!(child.body_size, 4145);
    assert_eq!(child.response_headers.get("content-encoding").map(String::as_str), Some("gzip"));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_module_http_failure_keeps_actual_status() {
    let base = fixture(vec![(404, "Content-Type: application/javascript\r\n", b"missing module".to_vec())]).await;
    let mut rt = ObscuraJsRuntime::with_base_url(&format!("{base}/page"));
    rt.set_url(&format!("{base}/page"));
    rt.set_http_client(Arc::new(obscura_net::ObscuraHttpClient::with_full_options(
        Arc::new(obscura_net::CookieJar::new()), None, true,
    )));
    rt.run_page_init();
    assert!(rt.load_module(&format!("{base}/missing.js"), 4000).await.unwrap_err().contains("404"));
    let events = rt.take_js_network_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].status, 404);
    assert_eq!(events[0].encoded_data_length, Some(14));
    assert_eq!(rt.evaluate("performance.getEntriesByType('resource')[0].responseStatus").unwrap(), json!(404.0));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_modules_enforce_timing_allow_origin() {
    let code = b"export const value = 1;".to_vec();
    let size = code.len();
    let base = fixture(vec![
        (200, "Content-Type: application/javascript\r\nAccess-Control-Allow-Origin: *\r\n", code.clone()),
        (200, "Content-Type: application/javascript\r\nAccess-Control-Allow-Origin: *\r\nTiming-Allow-Origin: *\r\n", code),
    ]).await;
    let mut rt = ObscuraJsRuntime::with_base_url("https://document.test/page");
    rt.set_url("https://document.test/page");
    rt.set_http_client(Arc::new(obscura_net::ObscuraHttpClient::with_full_options(
        Arc::new(obscura_net::CookieJar::new()), None, true,
    )));
    rt.run_page_init();
    rt.load_module(&format!("{base}/hidden.js"), 4000).await.unwrap();
    rt.load_module(&format!("{base}/allowed.js"), 4000).await.unwrap();
    assert_eq!(rt.evaluate("performance.getEntriesByType('resource').map(e => [e.encodedBodySize, e.decodedBodySize, e.transferSize, e.responseStart > 0])").unwrap(),
        json!([[0, 0, 0, false], [size, size, size + 300, true]]));
    let events = rt.take_js_network_events();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|event| event.encoded_data_length == Some(size)));
}

#[tokio::test(flavor = "current_thread")]
async fn resource_timing_modules_require_tao_across_every_redirect_hop() {
    let code = b"export const redirected = true;".to_vec();
    let size = code.len();
    let base = fixture(vec![
        (302, "Access-Control-Allow-Origin: *\r\nLocation: /first-final.js\r\n", vec![]),
        (200, "Content-Type: application/javascript\r\nAccess-Control-Allow-Origin: *\r\nTiming-Allow-Origin: *\r\n", code.clone()),
        (302, "Access-Control-Allow-Origin: *\r\nTiming-Allow-Origin: *\r\nLocation: /second-final.js\r\n", vec![]),
        (200, "Content-Type: application/javascript\r\nAccess-Control-Allow-Origin: *\r\nTiming-Allow-Origin: *\r\n", code),
    ]).await;
    let mut rt = ObscuraJsRuntime::with_base_url("https://document.test/page");
    rt.set_url("https://document.test/page");
    rt.set_http_client(Arc::new(obscura_net::ObscuraHttpClient::with_full_options(
        Arc::new(obscura_net::CookieJar::new()), None, true,
    )));
    rt.run_page_init();
    rt.load_module(&format!("{base}/hidden-redirect.js"), 4000).await.unwrap();
    rt.load_module(&format!("{base}/allowed-redirect.js"), 4000).await.unwrap();
    assert_eq!(rt.evaluate("performance.getEntriesByType('resource').map(e => [e.encodedBodySize, e.transferSize])").unwrap(),
        json!([[0, 0], [size, size + 300]]));
}
