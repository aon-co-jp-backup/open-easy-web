//! DOM 操作の共通ヘルパー。

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element};

pub fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

pub fn document() -> Document {
    window().document().expect("window should have a document")
}

pub fn by_id(id: &str) -> Element {
    document()
        .get_element_by_id(id)
        .unwrap_or_else(|| panic!("missing #{id} element"))
}

pub fn try_by_id(id: &str) -> Option<Element> {
    document().get_element_by_id(id)
}

pub fn log(msg: &str) {
    web_sys::console::log_1(&JsValue::from_str(msg));
}

pub fn set_status(msg: &str) {
    by_id("status").set_text_content(Some(msg));
}

/// 最低限の HTML エスケープ(ユーザー入力/サーバー応答をそのまま
/// `inner_html` に差し込むための保護)。
pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// `content` をブラウザにファイルとしてダウンロードさせる
/// (CSVエクスポート・サイト一覧のJSONエクスポートで共用)。
pub fn trigger_download(filename: &str, content: &str, mime: &str) -> Option<()> {
    use js_sys::Array;
    use web_sys::{Blob, BlobPropertyBag, HtmlAnchorElement, Url};

    let parts = Array::new();
    parts.push(&JsValue::from_str(content));
    let props = BlobPropertyBag::new();
    props.set_type(mime);
    let blob = Blob::new_with_str_sequence_and_options(&parts, &props).ok()?;
    let url = Url::create_object_url_with_blob(&blob).ok()?;

    let anchor = document()
        .create_element("a")
        .ok()?
        .dyn_into::<HtmlAnchorElement>()
        .ok()?;
    anchor.set_href(&url);
    anchor.set_download(filename);
    anchor.click();
    Url::revoke_object_url(&url).ok();
    Some(())
}
