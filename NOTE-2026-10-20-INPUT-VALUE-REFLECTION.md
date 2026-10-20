2026-10-20

# Input default and default/on value reflection

Live Decker148 Chromium passes Materials29steps; verified Obscura fails before Materials because radio.value assignment does not reflect its content attribute. Isolated same-fixture receipt: Chromium property paste/attribute paste/selector1; Obscura property paste/attribute null/selector0. AppUI sets type=radio before value=id correctly.

Authority: https://html.spec.whatwg.org/multipage/input.html#dom-input-value defines default/on for checkbox/radio and default for hidden/submit/image/reset/button. Both getters read content attributes, setters write them; default/on falls back to on, default to empty. Current generic getter reads form cache first and setter only writes cache. Root cause chain: wrong value mode -> absent attribute -> original CSS cannot match -> journey cannot select Paste.

Repair both getter and setter mode dispatch before form-state cache. Preserve current editable value mode and select/textarea behavior. Tests cover all affected types, property/attribute roundtrips, empty/removal defaults, selector matching and text input isolation. No app-specific selector workaround. ToQ: real DOM and selectors match browser contract; ToT: ordinary lesson source remains usable. Blast radius standard reflected input types only; rollback revert scoped patch. Relevant workflow root AGENTS nextest release, managed Anvil executable build, full release suite,33/33 obstacle, real Materials rerun. Existing shared9224 stays untouched. Candidate is not accepted until those gates are measured.

Updated: 2026-10-20
Focused release nextest:2passed0skipped, run287060e6-93eb-4546-8cc6-37d70c117e37; log /tmp/fdevbox-obscura-input-reflection-focused-20261020.log. Full release nextest session77057 compiling current final source; acceptance remains pending.
