//! STL file dialogs. The browser build uses the page's file picker and a download link.

use std::cell::RefCell;
use std::rc::Rc;

pub type Incoming = Rc<RefCell<Option<(String, Vec<u8>)>>>;

pub fn incoming_slot() -> Incoming {
    Rc::new(RefCell::new(None))
}

#[cfg(target_arch = "wasm32")]
pub fn pick_stl(slot: Incoming) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let input = document
        .create_element("input")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    input.set_type("file");
    let _ = input.set_accept(".stl,model/stl");
    let input_cb = input.clone();
    let change = Closure::once(move |_event: web_sys::Event| {
        let Some(files) = input_cb.files() else {
            return;
        };
        let Some(file) = files.get(0) else { return };
        let name = file.name();
        let reader = web_sys::FileReader::new().unwrap();
        let reader_cb = reader.clone();
        let done = Closure::once(move |_event: web_sys::Event| {
            let Ok(result) = reader_cb.result() else {
                return;
            };
            let bytes = js_sys::Uint8Array::new(&result).to_vec();
            *slot.borrow_mut() = Some((name, bytes));
        });
        reader.set_onloadend(Some(done.as_ref().unchecked_ref()));
        let _ = reader.read_as_array_buffer(&file);
        done.forget();
    });
    input.set_onchange(Some(change.as_ref().unchecked_ref()));
    change.forget();
    input.click();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn pick_stl(_slot: Incoming) {}

#[cfg(target_arch = "wasm32")]
pub fn download_stl(bytes: &[u8]) {
    use wasm_bindgen::JsCast;

    let array = js_sys::Uint8Array::from(bytes);
    let parts = js_sys::Array::new();
    parts.push(&array);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("model/stl");
    let blob =
        web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).expect("blob");
    let url = web_sys::Url::create_object_url_with_blob(&blob).expect("url");
    let document = web_sys::window().unwrap().document().unwrap();
    let anchor = document
        .create_element("a")
        .unwrap()
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .unwrap();
    anchor.set_href(&url);
    anchor.set_download("printcaddy.stl");
    anchor.click();
    let _ = web_sys::Url::revoke_object_url(&url);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn download_stl(bytes: &[u8]) {
    let path = std::env::temp_dir().join("printcaddy.stl");
    let _ = std::fs::write(&path, bytes);
    eprintln!("wrote {}", path.display());
}

#[cfg(target_arch = "wasm32")]
pub fn publish_status(text: &str) {
    if let Some(body) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.body())
    {
        let _ = body.set_attribute("data-pc", text);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn publish_status(_text: &str) {}

pub type TextIncoming = std::rc::Rc<std::cell::RefCell<Option<(String, String)>>>;

pub fn text_slot() -> TextIncoming {
    std::rc::Rc::new(std::cell::RefCell::new(None))
}

#[cfg(target_arch = "wasm32")]
pub fn pick_text(slot: TextIncoming) {
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::JsCast;

    let window = web_sys::window().expect("window");
    let document = window.document().expect("document");
    let input = document
        .create_element("input")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    input.set_type("file");
    let _ = input.set_accept(".json,application/json");
    let input_cb = input.clone();
    let change = Closure::once(move |_event: web_sys::Event| {
        let Some(files) = input_cb.files() else {
            return;
        };
        let Some(file) = files.get(0) else { return };
        let name = file.name();
        let reader = web_sys::FileReader::new().unwrap();
        let reader_cb = reader.clone();
        let done = Closure::once(move |_event: web_sys::Event| {
            let Ok(result) = reader_cb.result() else {
                return;
            };
            let text = result.as_string().unwrap_or_default();
            *slot.borrow_mut() = Some((name, text));
        });
        reader.set_onloadend(Some(done.as_ref().unchecked_ref()));
        let _ = reader.read_as_text(&file);
        done.forget();
    });
    input.set_onchange(Some(change.as_ref().unchecked_ref()));
    change.forget();
    input.click();
}

#[cfg(not(target_arch = "wasm32"))]
pub fn pick_text(_slot: TextIncoming) {}

#[cfg(target_arch = "wasm32")]
pub fn download_text(name: &str, text: &str) {
    use wasm_bindgen::JsCast;
    let bytes = js_sys::Uint8Array::from(text.as_bytes());
    let parts = js_sys::Array::new();
    parts.push(&bytes);
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("application/json");
    let blob =
        web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &options).expect("blob");
    let url = web_sys::Url::create_object_url_with_blob(&blob).expect("url");
    let document = web_sys::window().unwrap().document().unwrap();
    let anchor = document
        .create_element("a")
        .unwrap()
        .dyn_into::<web_sys::HtmlAnchorElement>()
        .unwrap();
    anchor.set_href(&url);
    anchor.set_download(name);
    anchor.click();
    let _ = web_sys::Url::revoke_object_url(&url);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn download_text(name: &str, text: &str) {
    let path = std::env::temp_dir().join(name);
    let _ = std::fs::write(&path, text);
}
