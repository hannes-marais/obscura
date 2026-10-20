2026-10-20

# Keep system-font fit-content button labels on one line

Live148 Materials repaired inputs remain readable, but Use selected and Report an issue still wrap in owned19228 f4b8f5f. Isolated sameHTML/CSS control switches only exact bundled Liberation to system-ui: Obscura heights44px versus Chromium36px; exactLiberation control stays36px. Receipts served-v148-fdevbox-20261020/{fit-content-buttons-before-results,fit-content-system-buttons-before-results}; Gossipmsg_f9f88ba6.

Candidate cause: native_button_intrinsic_content is measured with paint::measure_text (regular DejaVu advance plus per-character bold approximation), while actual inline shaping chooses DejaVu Sans Bold at CSS600 through TextEngine. This is a hypothesis until a red source regression and same-shaper measurement prove it. Preserve hidden/out-of-flow label exclusion, icons, pseudo contributions and constrained wrapping. Avoid app CSS/hostname changes.

ToQ: red36pxheight regression for nested/direct labels, then exact source test/fullrelease2135+ tests/managedbuild/33obstacle/freshsamefixture+liveMaterials screenshots. ToT: ordinary action labels readable on one line without increasing app widths artificially. Blast radius native button intrinsic labels; rollback scoped measurement patch. Keep release151nativeownerhold and hosted148 unchanged. Before changes inspect fit-content path and measure using existing selected-font shaping only if source regression proves it.

Updated: 2026-10-20
Focused paint regression terminalRED nestedheight44px, matching isolatedbrowser. Sourceconfirms native labels use paint::measure_text regularface advances+syntheticbold, while TextEngine test explicitly selectsDejaVuSansBold atCSS600. Candidate uses existing TextEngine generated-text shaping for unwrapped intrinsic label width, removing its temporary item after measurement. Preserve fallbacklayout-only approximation and existing exclusion/icon/pseudo logic.

Fullrelease64909 terminal2135PASS1FAIL5skip. Failure existing layout_test auto_width_flex_button_keeps_native_intrinsic_sizing: buttonwidth159,x221, expectedcenter220.5 within0.01. Need independentlytest unchanged19228 odd159/even160 margin-auto geometry before attributing to repair or changing assertion. No fullgreen/candidatebuild claim.

Independent19228 baseline proves odd159pxbox centers at221,e ven160pxat220 whileChrome220.5/220, before candidate. Existing layout_test documents device-pixel rounding atline1710. Correct centering test expectedcoordinate to nearestpixel, keeping strict0.01comparison and120..220widthbounds; no widened tolerance/testskip. This asserts existinggeometrycontract, not fractionalCSSOMparity. FractionalDOMroundinggap remains explicit with baseline receipt; buttoncandidate not claimed tofix it.

Fullrelease55228 terminalexit0:2136passed5skipped, log /tmp/fdevbox-obscura-system-buttons-rounded-full-20261020.log. Proceedcanonicalcommit/push andmanagedbuild; requireexactbinary33obstacle+isolatedbuttonheight+liveMaterialshealth/screenshot. Retainprior1failure and roundingbaseline, nofractionalDOMparityclaim.
