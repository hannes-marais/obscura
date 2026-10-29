2026-10-29

# Native Space button activation

WebPilot's local Obscura self-test fails when Space is pressed on its second
button: Enter activates the first, but the second counter stays zero. The CDP
input dispatcher implements Enter default activation and has no corresponding
Space path. PCG Decker issue vm-zdoe records the evidence under
`/home/oumy/.cache/webpilot/selftest/20261029T215212Z`.

Implement native button Space activation on key release, bound to the element
that received the uncancelled keydown. Avoid activation after cancellation,
focus movement or disabled state; preserve typing in editable
controls and use the existing trusted activation helper. Cover release timing,
cancellation and changed focus with focused CDP tests. Build and validate via
Anvil on fdevbox, rerun WebPilot and the Obscura documented checks. Rollback
is limited to the new Space default action and its tests. ACC: EMgine
`docs/ACC.md`; ToQ/ToT: match native input semantics and keep evidence honest.

Chromium CDP control: plain, Shift and Control Space emit keydown, keypress,
keyup and one click; canceled keydown or keyup prevents activation, while
canceled keypress does not. A keydown handler moving focus prevents activation.
The implementation must retain these observations, not infer modifier rules.
Additional Chromium controls show focus movement in keypress or keyup, and
disabling the button in keyup, also prevent the click.

The first release nextest exposed an invalid two-statement expression passed
to the engine's expression wrapper. Wrapping the initialization in an IIFE
fixed it. Focused nextest run c56cfc15-9dd5-43ad-b07b-9fedc15fe1f9 passed;
full nextest, Anvil build, obstacle course and live WebPilot retest are pending.
