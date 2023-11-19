use dioxus::prelude::*;

pub fn WxWorkRedirect(cx: Scope) -> Element {
    let is_wxwork = use_state(cx, || is_wx());

    if *is_wxwork.get() {
        cx.render(rsx!{
            div { class: "font-regular relative mb-4 block w-full rounded-lg bg-blue-500 p-4 text-base leading-5 text-white opacity-100",
                "企业微信点击右上角，在内置浏览器打开"
            }
        })
    } else {
        None
    }
}

#[cfg(feature = "web")]
fn is_wx() -> bool {
    use wasm_bindgen::prelude::*;
    use web_sys::window;

    let window = match window() {
        Some(w) => w,
        None => return false,
    };
    let navigator = window.navigator();
    let ua = navigator.user_agent().unwrap_or("".to_string());
    ua.contains("wxwork/")
}
#[cfg(feature = "ssr")]
fn is_wx() -> bool {
    true
}
