2026-11-13

# Anchor downloads and Browser.setDownloadBehavior

IA VPM vm-4grg. Operator: "file an obscura fix", then "proceed".

## Problem

WebPilot's `<download>` step refused every engine but Chromium: Obscura could
not complete a download. `Browser.setDownloadBehavior` and
`Page.setDownloadBehavior` answered `{}` and stored nothing (the browser one
on the I/O fast path, before any state), no `Browser.downloadWillBegin` or
`Browser.downloadProgress` was ever emitted, and no file was written. An
activated `<a download>` was treated as a plain link: `location.assign(href)`,
which for the usual `blob:` link tried to navigate the page to the blob.
Decker X's export journey on apps.mxnxp.com reached Finish and stopped at the
download on Obscura, while Chromium downloaded the PowerPoint.

## Change

- Page JS (`bootstrap.js`): an activated `<a download>` with an href is
  downloaded, never navigated to. A `blob:` URL is read from the object URL
  registry at activation (a `revokeObjectURL` right after `click()` must not
  lose it); `data:` and http(s) URLs are fetched. The suggested name is the
  `download` attribute, else the URL's last path segment, reduced to a
  basename. `op_request_download` queues `{url, suggestedFilename, bytes}` on
  the realm (bounded at 32 between drains).
- `Page::take_pending_downloads` drains it, like binding calls.
- CDP: `Browser.setDownloadBehavior` goes to the dispatcher and is stored per
  browser context (`CdpContext::download_behaviors`; the default context when
  `browserContextId` is absent). `allow`/`allowAndName` need an absolute
  `downloadPath`; an unknown context or behavior is refused as Chrome refuses.
  `dispatch::drain_downloads` runs with `drain_binding_calls` after every
  command and every autonomous event-loop turn. Under `allow` the file gets
  the suggested name (`name (n).ext` when taken), under `allowAndName` the
  download's guid; it is written as `<name>.crdownload` and renamed into
  place, so a file under its final name is complete. With `eventsEnabled`,
  `Browser.downloadWillBegin` (page frame id, guid, url, suggestedFilename)
  and `Browser.downloadProgress` (`completed`, or `canceled` when the write
  failed) are emitted browser-level, without a session, as Chrome sends them.
  A context with no behavior, `deny` or `default` writes and reports nothing:
  there is no default download directory.
- `Browser.cancelDownload` is acknowledged; downloads are written whole when
  the page starts them, so none is in progress.

Not in this change: downloads started from a child frame's realm (its queue
is not drained), `Content-Disposition` navigations, and the deprecated
`Page.setDownloadBehavior` / `Page.download*` events.

## ToQ and ToT

ToQ: WebPilot reads the file the browser wrote and the browser's own
completion events, so the evidence describes what happened; nothing is
synthesized after the fact. ToT: the queue-and-drain path is the one
`Runtime.bindingCalled` uses; the policy lives with the context it belongs to.

## Validation

`cargo nextest run --release --features render -p obscura-cdp -E 'test(download)'`:
`an_anchor_download_is_written_and_reported_under_the_context_behavior`
(no policy: nothing, page not navigated; allowAndName: both events,
browser-level, guid file with the bytes, no `.crdownload` left; deny:
nothing) and `set_download_behavior_refuses_unknown_contexts_and_missing_paths`.

Full gate, 2026-11-13: `cargo nextest run --release --features render
--no-fail-fast` 2553 passed, 5 skipped; release build `-p obscura-cli --bins
--features render`; obstacle course 33/33. End to end: WebPilot's own
`rust/webpilot/fixtures/fixture-download.webpilot` (emgine) against
`obscura serve` from this build passed, both downloads browser-confirmed with
the fixture's exact SHA-256 in fresh contexts; its refusals hold on Obscura
(two downloads from one click: "more than one frame-owned download began";
wrong name: "suggested filename does not match"; 8 KiB over a 4 KiB bound:
"exceeds maxBytes").
