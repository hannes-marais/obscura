use obscura_js::runtime::ObscuraJsRuntime;
use serde_json::json;

#[test]
fn image_alt_reflects_the_content_attribute_including_empty_text() {
    let mut rt = ObscuraJsRuntime::new();
    rt.set_dom(obscura_dom::parse_html("<body></body>"));
    rt.run_page_init();
    let result = rt.evaluate(r#"(() => {
        const image = document.createElement('img');
        const state = () => [image.alt, image.hasAttribute('alt'), image.getAttribute('alt')];
        const initial = state();
        image.alt = '';
        const empty = state();
        image.alt = 'Forest';
        const assigned = state();
        image.setAttribute('alt', 'Sample');
        const changed = state();
        image.removeAttribute('alt');
        return [initial, empty, assigned, changed, state()];
    })()"#).unwrap();
    assert_eq!(result, json!([["", false, null], ["", true, ""],
        ["Forest", true, "Forest"], ["Sample", true, "Sample"], ["", false, null]]));
}
