use dioxus::prelude::*;

pub fn WxWorkRedirect(cx: Scope) -> Element {
    // let ua = get_user_agent().unwrap_or("".to_string());
    // if ua.contains("wxwork/") {
    cx.render(rsx! { div { "企业微信点击右上角，在内置浏览器打开" } })
    // } else {
    //     None
    // }
}

// fn get_user_agent() -> Option<String> {
//     #[cfg(feature = "ssr")]
//     return None;
//
//     #[cfg(feature = "web")]
//     {
//         use wasm_bindgen::prelude::*;
//         use web_sys::window;
//
//         #[wasm_bindgen]
//         pub fn browser_get_user_agent() -> Option<String> {
//             let window = window()?;
//             let navigator = window.navigator();
//             navigator.user_agent().ok()
//         }
//         browser_get_user_agent()
//     }
// }
