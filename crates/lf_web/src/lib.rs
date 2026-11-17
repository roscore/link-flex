pub mod gl;
pub mod math;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(start)]
pub fn start() {
    let d = web_sys::window().unwrap().document().unwrap();
    d.body().unwrap().set_inner_html("stub ok");
}
