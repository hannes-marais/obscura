2026-10-31

# Preserve null XML namespace through cloning

DevYard Decker server Build on fdevbox failed with `unexpected attribute mediatype` on a typed image asset (PCG vm-ncgu). A separate Obscura 3b90cfb probe parses an XML element with `mediaType` and `byteCount`, clones it, and shows that the original reports null namespace while the clone reports XHTML. `setAttribute` then folds the attribute names and introduces invalid duplicates. Evidence: `/home/oumy/anvil/work/evidence/decker-devyard-20261030/xml-case-namespace-obscura` and `build-traced-complete-obscura`.

The Rust clone preserves the underlying qualified name. The JS `namespaceURI` getter incorrectly treats the empty namespace from the DOM bridge as missing and invents XHTML; freshly created elements mask this through a wrapper-only `_ns` field, which the clone does not inherit. Return null for an actual empty namespace, while retaining real HTML and SVG namespaces. Do not paper over BXML or modify the attribute contract.

Validate detached and nested XML clone namespaces, case-sensitive replacement and XML serialization, HTML case folding, SVG and prefixed namespaces, and existing namespace/clone regression tests. Then run the mandated full Obscura release nextest, release build and obstacle checks, Anvil build from pushed source, and the real disposable DevYard server/placeholder Build and fresh-browser readback. Preserve unrelated dirty files. Risk: browser compatibility for namespace classification; regressions in HTML or SVG block release. Rollback is the getter and focused tests. Owner-document handling is a distinct pre-existing issue and is not silently claimed fixed.

Updated: 2026-10-31
Focused release nextest passed both the new null-namespace clone regression, including HTML and SVG checks, and the existing foreign-element clone test. Full release nextest is running. No end-to-end Build pass yet.
