#!/usr/bin/env python3
"""Capture one deterministic Chromium reference with Playwright."""

import argparse
import base64
import hashlib
import json
from pathlib import Path
from urllib.parse import unquote, urlsplit

from playwright.sync_api import sync_playwright



def reference_fonts(url):
    """Load a fixture's explicit font inputs; never silently use missing files."""
    parsed = urlsplit(url)
    if parsed.scheme != "file":
        return []
    fixture = Path(unquote(parsed.path))
    contract = fixture.with_suffix(".fonts.json")
    if not contract.exists():
        return []
    rows = json.loads(contract.read_text())["fonts"]
    if not rows:
        raise ValueError("reference font contract is empty")
    fonts = []
    for row in rows:
        family, selectors = row["family"], row["selectors"]
        if not isinstance(family, str) or not family or not selectors or not all(isinstance(s, str) and s for s in selectors):
            raise ValueError("reference fonts need a family and nonempty selectors")
        path = (contract.parent / row["file"]).resolve()
        data = path.read_bytes()
        fonts.append({"family": family, "selectors": selectors, "path": str(path),
                      "sha256": hashlib.sha256(data).hexdigest(), "size": len(data),
                      "data_base64": base64.b64encode(data).decode("ascii")})
    return fonts


def install_reference_fonts(page, fonts):
    if not fonts:
        return
    page.evaluate("""async fonts => {
        const rules = [];
        for (const font of fonts) {
            const bytes = Uint8Array.from(atob(font.data_base64), c => c.charCodeAt(0));
            const face = new FontFace(font.family, bytes.buffer);
            await face.load();
            document.fonts.add(face);
            for (const selector of font.selectors)
                rules.push(selector + '{font-family:' + JSON.stringify(font.family) + ';}');
        }
        const style = document.createElement('style');
        style.textContent = rules.join('\\n');
        document.head.append(style);
        await document.fonts.ready;
    }""", fonts)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("url")
    parser.add_argument("output", type=Path)
    parser.add_argument("--width", type=int, default=900)
    parser.add_argument("--height", type=int, default=1000)
    parser.add_argument("--settle-ms", type=int, default=2000)
    parser.add_argument("--executable")
    args = parser.parse_args()
    fonts = reference_fonts(args.url)

    launch = {"headless": True}
    if args.executable:
        launch["executable_path"] = args.executable

    with sync_playwright() as playwright:
        browser = playwright.chromium.launch(**launch)
        context = browser.new_context(
            viewport={"width": args.width, "height": args.height},
            device_scale_factor=1,
            locale="en-US",
            timezone_id="UTC",
            color_scheme="light",
        )
        page = context.new_page()
        page.goto(args.url, wait_until="load", timeout=30_000)
        install_reference_fonts(page, fonts)
        page.wait_for_timeout(args.settle_ms)
        page.screenshot(path=str(args.output))
        print(
            json.dumps(
                {
                    "browser": "chromium",
                    "version": browser.version,
                    "viewport": [args.width, args.height],
                    "device_scale_factor": 1,
                    "settle_ms": args.settle_ms,
                    "reference_fonts": [{k: v for k, v in f.items() if k != "data_base64"} for f in fonts],
                }
            )
        )
        context.close()
        browser.close()


if __name__ == "__main__":
    main()
