2026-10-20

# Reflect HTMLInputElement.size

Independent local browser probes show input.size assignments do not update the size content attribute in Obscura; attribute assignments do affect layout. Missing reflector is distinct from the unresolved native font metrics/cyclic-flex sizing candidate in dom.rs. Preserve that uncommitted candidate while committing only this scoped JS repair.

Authority: https://html.spec.whatwg.org/multipage/input.html#dom-input-size and common-dom-interfaces.html reflected unsigned-long positive-number rules. Getter defaults20 for absent/invalid/zero/out-of-range attributes, parses HTML nonnegative integers. Setter converts unsigned32, rejectszero with IndexSizeError without mutation, substitutes20 above2147483647, writes attribute. Must be on HTMLInputElement only; no Element-wide size property.

Acceptance: defaults, attribute mutation/removal, property assignment, fractional conversion, zero rejection, invalid attributes, upper boundary, uint32 wrapping and non-input isolation. ToQ contract/source regression; ToT programmatically sized fields behave normally. Blast radius input.size only; rollback scoped reflector. Focused release nextest, full release suite and managed browser probe before shipped acceptance. No native metrics156px claim.
