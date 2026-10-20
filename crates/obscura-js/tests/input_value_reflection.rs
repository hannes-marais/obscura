use obscura_js::runtime::ObscuraJsRuntime;
use serde_json::json;

#[test]
fn reflected_input_value_modes_follow_content_attributes() {
    let mut rt = ObscuraJsRuntime::new();
    rt.set_dom(obscura_dom::parse_html("<body></body>"));
    rt.run_page_init();
    let result = rt.evaluate(r#"(() => {
        return ['radio', 'checkbox', 'hidden', 'submit', 'image', 'reset', 'button'].map(type => {
            const input = document.createElement('input');
            input.type = type;
            document.body.append(input);
            const initial = input.value;
            input.value = 'paste';
            const assigned = [input.value, input.getAttribute('value'), input.matches('[value="paste"]')];
            input.setAttribute('value', 'changed');
            const changed = input.value;
            input.value = '';
            const empty = [input.value, input.getAttribute('value')];
            input.removeAttribute('value');
            return [type, initial, assigned, changed, empty, input.value];
        });
    })()"#).unwrap();
    let expected: Vec<_> = ["radio", "checkbox", "hidden", "submit", "image", "reset", "button"]
        .iter().map(|kind| {
            let default = if *kind == "radio" || *kind == "checkbox" { "on" } else { "" };
            json!([kind, default, ["paste", "paste", true], "changed", ["", ""], default])
        }).collect();
    assert_eq!(result, json!(expected));
}

#[test]
fn editable_input_value_does_not_reflect_into_its_default_attribute() {
    let mut rt = ObscuraJsRuntime::new();
    rt.set_dom(obscura_dom::parse_html("<input id='text' value='original'>"));
    rt.run_page_init();
    assert_eq!(rt.evaluate(r#"(() => {
        const input = document.getElementById('text');
        input.value = 'edited';
        return [input.value, input.getAttribute('value')];
    })()"#).unwrap(), json!(["edited", "original"]));
}

#[test]
fn input_size_reflects_positive_unsigned_attribute_with_default_twenty() {
    let mut rt = ObscuraJsRuntime::new();
    rt.set_dom(obscura_dom::parse_html("<body></body>"));
    rt.run_page_init();
    assert_eq!(rt.evaluate(r#"(() => {
        const input = document.createElement('input');
        const out = [input.size];
        input.size = 10.9;
        out.push([input.size, input.getAttribute('size')]);
        try { input.size = 0; } catch (e) { out.push(e.name); }
        out.push(input.getAttribute('size'));
        const attrs = ['0', '-1', 'garbage', '2147483648', '  +12tail', '2147483647'];
        out.push(attrs.map(value => { input.setAttribute('size', value); return input.size; }));
        input.size = -1;
        out.push([input.size, input.getAttribute('size')]);
        input.size = 4294967297;
        out.push([input.size, input.getAttribute('size')]);
        input.removeAttribute('size');
        out.push(input.size, 'size' in document.createElement('div'));
        return out;
    })()"#).unwrap(), json!([20, [10, "10"], "IndexSizeError", "10",
        [20, 20, 20, 20, 12, 2147483647], [20, "20"], [1, "1"], 20, false]));
}
