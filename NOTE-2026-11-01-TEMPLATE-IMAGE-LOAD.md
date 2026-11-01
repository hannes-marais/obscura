2026-11-01

# Images in template contents load only once inserted

## Symptom

Hosted Decker v293 under Obscura reported 18 "Rendered Block image failed to
load." frontend errors on the Read screen. The same journey on Chromium
reported none, and every image URL on the screen returned 200 (emgine
WebPilot evidence 20261101T003115Z vs 20261101T004751Z).

## Cause

MAW's generated runtime (emgine `rust/maw/catalogue/lower/ts.moot`
`mountedHTMLFragment`) parses Block HTML with `template.innerHTML`. It
rewrites `/assets/` references and adds an `error` listener to each `<img>`
while the image is still in `template.content`, then inserts the fragment.

In Obscura, `HTMLImageElement.addEventListener('error')` queues the image
request at once. An image in template contents cannot be resolved by the
render op, which fails, so `error` fires. The later insertion never retries:
the request id is unchanged and the image is already complete.

HTML: template contents belong to an inert document with no browsing
context, so their images do not fetch. Insertion into the live document (the
adopting steps) restarts "update the image data". Chromium does this.

## Change

- `_queueImageRequest` does not start a request for an image inside a
  template-contents fragment (an ancestor `DocumentFragment` whose
  `_fragmentContext` is `template`); it marks the image deferred.
- `__prepareInsertedSubtree` re-queues deferred images in an inserted
  subtree; the queue re-checks, so an image still in a template stays
  deferred.
- Disconnected images outside templates (`new Image()`, `createElement`)
  keep loading, as the spec requires.

## ToQ

- Same object: wrappers are cached by node id (`_wrap`), so the deferred flag
  and the page's listeners stay on the element that is inserted.
- Fragment insertion: `appendChild(fragment)` inserts each child through
  `appendChild`, which calls `__prepareInsertedSubtree` per child.

## ToT

`crates/obscura/tests/image_shim.rs`, render feature:
- a template image with an error listener, inserted into the body, fires
  `load` and never `error`;
- a template image never inserted fires neither;
- the existing `new Image()` load and invalid-bytes error tests still pass.

Then rerun the hosted Decker journey on Obscura.
