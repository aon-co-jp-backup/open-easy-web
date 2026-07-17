//! open-runo-view(第二のReact)との配線 — Phase 3/4 実戦投入第1号。
//! (HYBRID_NETWORK_ARCHITECTURE.md §0.9.3: open-easy-web を最初の投入先とする)
//!
//! SSR側(open-runo / poem-cosmo-tauri の Poem ハンドラ)が
//! `ssr::render_page` で出力したページに対し、この関数が
//! `window.__OPEN_RUNO_STATE__` を読んで同一コンポーネントを再マウント
//! (hydration)する。単体でも空stateでマウント可能。
//!
//! Phase 4: 「詳細を表示」ボタンに宣言的 `on("click", id)` をバインドし、
//! `DomMount::attach_with_dispatch` の委譲リスナー → `Runtime::dispatch` →
//! (dirtyなら) `rerender` → `apply` という1ループで完結させる。
//! `mount` 自身を dispatch クロージャから参照する必要があるため、
//! `Rc<RefCell<Option<DomMount>>>` を先に確保してから
//! `attach_with_dispatch` で中身を埋める、という2段構えにする
//! (Rustの借用規則上、自己参照を避けるための標準的な回避策)。

use open_runo_view::dom::DomMount;
use open_runo_view::hooks::{Ctx, Runtime};
use open_runo_view::{h, VNode};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// SSR⇔クライアントで共有する初期状態(デモ: サイト稼働ステータスパネル)。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatusPanelState {
    pub site_name: String,
    pub domains: Vec<String>,
    pub healthy: bool,
}

/// ステータスパネル・コンポーネント(SSR側とクライアント側で同一定義を使う)。
///
/// SSR時は `expanded` の初期値(`false`)のまま描画され、クライアント側で
/// hydration後にボタンクリックのたびに `Runtime` の状態として開閉が変わる。
pub fn status_panel(ctx: &mut Ctx, props: &StatusPanelState) -> VNode {
    let (expanded, set_expanded) = ctx.use_state(|| false);
    let toggle_id = ctx.use_handler(move || set_expanded.update(|v| !v));

    let mut root = h("section")
        .attr("class", if props.healthy { "status ok" } else { "status ng" })
        .attr("data-openruno", "status-panel")
        .child(h("h2").child(props.site_name.as_str()).build())
        .child(
            h("p")
                .child(if props.healthy { "稼働中" } else { "停止中" })
                .build(),
        )
        .child(
            h("button")
                .attr("type", "button")
                .on("click", toggle_id)
                .child(if expanded { "詳細を隠す" } else { "詳細を表示" })
                .build(),
        );
    if expanded {
        root = root.child(
            h("ul")
                .children(
                    props
                        .domains
                        .iter()
                        .map(|d| h("li").key(d).child(d.as_str()).build()),
                )
                .build(),
        );
    }
    root.build()
}

fn read_hydration_state() -> StatusPanelState {
    web_sys::window()
        .and_then(|w| js_sys::Reflect::get(&w, &JsValue::from_str("__OPEN_RUNO_STATE__")).ok())
        .and_then(|v| {
            if v.is_undefined() || v.is_null() {
                None
            } else {
                js_sys::JSON::stringify(&v).ok().and_then(|s| s.as_string())
            }
        })
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

/// hydrationエントリポイント。SSRページ側の `scripts` から
/// `openruno_hydrate("open-runo-root")` を呼ぶ。
///
/// `Runtime`/`DomMount`/`props` を `Rc<RefCell<..>>` で包み、委譲リスナーの
/// クロージャ(`'static`)から安全に借用できるようにする。wasm32はシングル
/// スレッド実行なので `RefCell` で十分(マルチスレッドはネイティブ側
/// `ThreadedProxyServer` の担当、§0.9.3)。
#[wasm_bindgen]
pub fn openruno_hydrate(root_id: &str) -> Result<(), JsValue> {
    let props = Rc::new(read_hydration_state());
    let rt = Rc::new(RefCell::new(Runtime::new(status_panel)));
    // mountは attach_with_dispatch の中でしか作れないが、そのクロージャ自身が
    // mountを参照する必要があるため、先に空の置き場を用意しておく。
    let mount_slot: Rc<RefCell<Option<DomMount>>> = Rc::new(RefCell::new(None));

    let mount = {
        let rt = rt.clone();
        let props = props.clone();
        let mount_slot = mount_slot.clone();
        DomMount::attach_with_dispatch(root_id, move |handler_id, _event| {
            rt.borrow().dispatch(handler_id);
            if rt.borrow().is_dirty() {
                let patches = rt.borrow_mut().rerender(&props);
                if let Some(m) = mount_slot.borrow().as_ref() {
                    if let Err(e) = m.apply(&patches) {
                        web_sys::console::error_1(&JsValue::from_str(&format!(
                            "openruno_hydrate: apply failed: {e:?}"
                        )));
                    }
                }
            }
        })
        .map_err(|e| JsValue::from_str(&format!("mount failed: {e:?}")))?
    };

    // 初回マウント(SSR済みDOMをそのまま尊重するなら Patch::Replace は
    // 冪等な置換になるだけで、hydration上問題ない)。
    let initial_patches = rt.borrow_mut().rerender(&props);
    mount
        .apply(&initial_patches)
        .map_err(|e| JsValue::from_str(&format!("initial apply failed: {e:?}")))?;

    *mount_slot.borrow_mut() = Some(mount);
    // ルートマウントはページ寿命全体で1つだけ生き続ける想定のため、
    // Rcの参照カウントをリークして寿命をプロセス終了まで延ばす
    // (SPAのエントリポイントとして標準的な wasm-bindgen の慣例)。
    std::mem::forget(mount_slot);
    Ok(())
}
