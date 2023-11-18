use dioxus::prelude::*;

pub fn WxWorkRedirect(cx: Scope) -> Element {
    cx.render(rsx! { div { "企业微信点击右上角，在内置浏览器打开" } })
}
