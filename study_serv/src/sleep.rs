use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
extern "C" {
    fn setTimeout(callback: &Closure<dyn FnMut()>, millis: i32) -> i32;
}

pub async fn sleep(millis: i32) -> Result<(), JsValue> {
    let promise = Promise::new(&mut |yes, _| {
        let closure = Closure::once(move || {
            yes.call0(&JsValue::NULL).unwrap();
        });
        setTimeout(&closure, millis);
    });

    JsFuture::from(promise).await?;

    Ok(())
}
