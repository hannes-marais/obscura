#![cfg(feature = "paint")]

use std::sync::{Arc, Mutex};
use obscura_dom::{parse_html, tree::ShadowRootMode};
use obscura_render::{prepare_dom, RenderResourceCache};

fn resources() -> (RenderResourceCache, Arc<Mutex<Vec<String>>>) {
    let loads = Arc::new(Mutex::new(Vec::new()));
    let seen = loads.clone();
    let cache = RenderResourceCache::with_loader(move |url: &str| {
        seen.lock().unwrap().push(url.to_string());
        Some(include_bytes!("../assets/liberation-serif.ttf").to_vec())
    });
    (cache, loads)
}

#[test]
fn font_demand_ignores_unused_families_and_follows_inherited_text() {
    let tree = parse_html(r#"<style>
        @font-face{font-family:Used;src:url(used.ttf)}
        @font-face{font-family:Unused;src:url(unused.ttf)}
        body{font-family:Used} .empty{font-family:Unused}
        </style><body><span id=text style="display:inline-block">Visible text</span>
        <div class=empty></div></body>"#);
    let (mut cache, loads) = resources();
    let prepared = prepare_dom(&tree, (500.0, 200.0), Some("https://example.test/"), &mut cache).unwrap();
    assert_eq!(*loads.lock().unwrap(), vec!["https://example.test/used.ttf"]);
    assert!(prepared.document_rect(tree.get_element_by_id("text").unwrap()).unwrap().width > 0.0);
}

#[test]
fn font_demand_tracks_hidden_and_dynamic_style_changes() {
    let tree = parse_html(r#"<style>
        @font-face{font-family:Active;src:url(active.ttf)}
        @font-face{font-family:Hidden;src:url(hidden.ttf)}
        </style><body style="font-family:Arial"><div id=target>Visible</div>
        <div id=hidden style="display:none;font-family:Hidden"><span>Hidden</span></div></body>"#);
    let (mut cache, loads) = resources();
    prepare_dom(&tree, (500.0, 200.0), Some("https://example.test/"), &mut cache).unwrap();
    assert!(loads.lock().unwrap().is_empty());
    tree.with_node_mut(tree.get_element_by_id("target").unwrap(), |node|
        node.set_attribute("style", "font-family:Active".into()));
    prepare_dom(&tree, (500.0, 200.0), Some("https://example.test/"), &mut cache).unwrap();
    assert_eq!(*loads.lock().unwrap(), vec!["https://example.test/active.ttf"]);
    tree.with_node_mut(tree.get_element_by_id("hidden").unwrap(), |node|
        node.set_attribute("style", "font-family:Hidden".into()));
    prepare_dom(&tree, (500.0, 200.0), Some("https://example.test/"), &mut cache).unwrap();
    assert_eq!(*loads.lock().unwrap(), vec!["https://example.test/active.ttf", "https://example.test/hidden.ttf"]);
}

#[test]
fn font_demand_preserves_pseudo_control_and_shadow_text() {
    let tree = parse_html(r#"<style>
        @font-face{font-family:Pseudo;src:url(pseudo.ttf)}
        @font-face{font-family:Control;src:url(control.ttf)}
        #pseudo::before{content:'Generated';font-family:Pseudo}
        input{font-family:Control}
        </style><div id=pseudo></div><input value=Input><x-card id=host></x-card>
        <div id=source><style>@font-face{font-family:Shadow;src:url(shadow.ttf)}
        span{font-family:Shadow}</style><span>Shadow text</span></div>"#);
    let shadow = tree.attach_shadow_root(tree.get_element_by_id("host").unwrap(), ShadowRootMode::Open).unwrap();
    let source = tree.get_element_by_id("source").unwrap();
    for child in tree.children(source) { tree.append_child(shadow, child); }
    tree.remove(source);
    let (mut cache, loads) = resources();
    prepare_dom(&tree, (500.0, 200.0), Some("https://example.test/"), &mut cache).unwrap();
    let mut actual = loads.lock().unwrap().clone(); actual.sort();
    assert_eq!(actual, vec!["https://example.test/control.ttf", "https://example.test/pseudo.ttf", "https://example.test/shadow.ttf"]);
}

#[test]
fn font_demand_preserves_explicit_unused_preload() {
    let tree = parse_html(r#"<link rel=preload as=font href=preload.ttf>
        <style>@font-face{font-family:Unused;src:url(preload.ttf)}</style>
        <body style="font-family:Arial">System text</body>"#);
    let (mut cache, loads) = resources();
    prepare_dom(&tree, (500.0, 200.0), Some("https://example.test/"), &mut cache).unwrap();
    assert_eq!(*loads.lock().unwrap(), vec!["https://example.test/preload.ttf"]);
}

#[test]
fn font_demand_refreshes_after_content_image_container_correction() {
    let tree = parse_html(r#"<style>
        @font-face{font-family:Narrow;src:url(narrow.ttf)}
        @font-face{font-family:Wide;src:url(wide.ttf)}
        @font-face{font-family:Unused;src:url(unused.ttf)}
        #grid{display:inline-grid;grid-template-columns:auto}
        #image{content:url(wide.svg)}
        #container{container-type:inline-size}
        #text{font-family:Narrow}
        @container(min-width:100px){#text{font-family:Wide}}
        </style><div id=grid><img id=image src=narrow.svg>
        <div id=container><span id=text>Demand follows corrected geometry</span></div></div>"#);
    let loads = Arc::new(Mutex::new(Vec::new()));
    let seen = loads.clone();
    let mut cache = RenderResourceCache::with_loader(move |url: &str| {
        seen.lock().unwrap().push(url.to_string());
        if url.ends_with(".svg") {
            let width = if url.ends_with("wide.svg") { 200 } else { 20 };
            Some(format!(r#"<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="20"/>"#).into_bytes())
        } else {
            Some(include_bytes!("../assets/liberation-serif.ttf").to_vec())
        }
    });
    let prepared = prepare_dom(&tree, (500.0, 300.0), Some("https://example.test/"), &mut cache).unwrap();
    let text = tree.get_element_by_id("text").unwrap();
    assert_eq!(prepared.layout().styles[&text].font_family.as_deref(), Some("wide"));
    let actual = loads.lock().unwrap();
    assert!(actual.iter().any(|url| url.ends_with("wide.ttf")), "final demanded face must load: {actual:?}");
    assert!(!actual.iter().any(|url| url.ends_with("unused.ttf")), "unrelated family must stay unloaded");
}

#[test]
fn font_demand_preserves_svg_presentation_family_and_css_priority() {
    let tree = parse_html(r#"<style>
        @font-face{font-family:SvgFace;src:url(svg.ttf)}
        @font-face{font-family:CssFace;src:url(css.ttf)}
        @font-face{font-family:Ignored;src:url(ignored.ttf)}
        #css{font-family:CssFace}
        </style><svg width=300 height=100><g font-family=SvgFace>
        <text x=0 y=30>Presentation text</text></g>
        <text id=css font-family=Ignored x=0 y=60>Author CSS wins</text></svg>"#);
    let (mut cache, loads) = resources();
    let prepared = prepare_dom(&tree, (500.0, 200.0), Some("https://example.test/"), &mut cache).unwrap();
    assert_eq!(prepared.layout().styles[&tree.get_element_by_id("css").unwrap()].font_family.as_deref(), Some("cssface"));
    let mut actual = loads.lock().unwrap().clone(); actual.sort();
    assert_eq!(actual, vec!["https://example.test/css.ttf", "https://example.test/svg.ttf"]);
}
