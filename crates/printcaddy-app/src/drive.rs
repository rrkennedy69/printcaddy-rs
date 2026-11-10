//! Google Drive app-data, using the same token client as the TypeScript site.
//! The client id is the public one that site already ships. It is not a secret.

use std::cell::RefCell;
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

pub type DriveNote = Rc<RefCell<Option<String>>>;
pub type DriveFile = Rc<RefCell<Option<String>>>;

#[cfg(target_arch = "wasm32")]
const CLIENT_ID: &str = "403035756867-1sedplme81unu7ktjt2ru1njsjhbv7nq.apps.googleusercontent.com";

pub fn save(name: String, body: String, note: DriveNote) {
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        let message = match save_async(&name, &body).await {
            Ok(()) => format!("Saved {name} to Google Drive."),
            Err(err) => err,
        };
        *note.borrow_mut() = Some(message);
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (name, body);
        *note.borrow_mut() = Some("Google Drive is only available in the browser.".into());
    }
}

pub fn open_latest(note: DriveNote, file: DriveFile) {
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(async move {
        match open_async().await {
            Ok((name, body)) => {
                *file.borrow_mut() = Some(body);
                *note.borrow_mut() = Some(format!("Opened {name} from Google Drive."));
            }
            Err(err) => *note.borrow_mut() = Some(err),
        }
    });
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = file;
        *note.borrow_mut() = Some("Google Drive is only available in the browser.".into());
    }
}

#[cfg(target_arch = "wasm32")]
async fn save_async(name: &str, body: &str) -> Result<(), String> {
    let token = token().await?;
    let boundary = "printcaddy";
    let file_name = format!("printcaddy-{name}.json");
    let meta = format!(
        "{{\"name\":{},\"parents\":[\"appDataFolder\"]}}",
        serde_json::to_string(&file_name).unwrap_or_else(|_| "\"printcaddy.json\"".into())
    );
    let payload = format!(
        "--{boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n{meta}\r\n--{boundary}\r\nContent-Type: application/json\r\n\r\n{body}\r\n--{boundary}--"
    );
    let url = "https://www.googleapis.com/upload/drive/v3/files?uploadType=multipart&fields=id";
    let status = authed_post(
        &token,
        url,
        &format!("multipart/related; boundary={boundary}"),
        &payload,
    )
    .await?;
    if !(200..300).contains(&status) {
        return Err(drive_error(status));
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
async fn open_async() -> Result<(String, String), String> {
    let token = token().await?;
    let list_url = "https://www.googleapis.com/drive/v3/files?spaces=appDataFolder&orderBy=modifiedTime%20desc&pageSize=1&fields=files(id,name)&q=name%20contains%20'printcaddy-'";
    let list = authed_get(&token, list_url).await?;
    let id = json_after(&list, "\"id\": \"").or_else(|| json_after(&list, "\"id\":\""));
    let Some(id) = id else {
        return Err("No PrintCaddy project is in this Google Drive yet.".into());
    };
    let name = json_after(&list, "\"name\": \"")
        .or_else(|| json_after(&list, "\"name\":\""))
        .unwrap_or_else(|| "project".into());
    let media = format!("https://www.googleapis.com/drive/v3/files/{id}?alt=media");
    let body = authed_get(&token, &media).await?;
    Ok((name, body))
}

#[cfg(target_arch = "wasm32")]
fn drive_error(status: u16) -> String {
    if status == 401 || status == 403 {
        "Google refused this site. The existing client must allow https://rrkennedy69.github.io as a JavaScript origin. No new client was created.".into()
    } else {
        format!("Google Drive returned HTTP {status}.")
    }
}

#[cfg(target_arch = "wasm32")]
fn json_after(text: &str, key: &str) -> Option<String> {
    let start = text.find(key)? + key.len();
    let end = text[start..].find('"')? + start;
    Some(text[start..end].to_string())
}

#[cfg(target_arch = "wasm32")]
async fn token() -> Result<String, String> {
    load_gis().await?;
    let window = web_sys::window().ok_or("No browser window.")?;
    let google = js_sys::Reflect::get(&window, &"google".into())
        .map_err(|_| "Google sign-in did not load.".to_string())?;
    let accounts = js_sys::Reflect::get(&google, &"accounts".into())
        .map_err(|_| "Google sign-in did not load.".to_string())?;
    let oauth = js_sys::Reflect::get(&accounts, &"oauth2".into())
        .map_err(|_| "Google sign-in did not load.".to_string())?;
    let init = js_sys::Reflect::get(&oauth, &"initTokenClient".into())
        .map_err(|_| "Google sign-in did not load.".to_string())?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "Google sign-in did not load.".to_string())?;
    let (sender, receiver) = futures_channel();
    let callback = wasm_bindgen::closure::Closure::once(move |value: wasm_bindgen::JsValue| {
        let _ = sender_send(&sender, value);
    });
    let opts = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&opts, &"client_id".into(), &CLIENT_ID.into());
    let _ = js_sys::Reflect::set(
        &opts,
        &"scope".into(),
        &"https://www.googleapis.com/auth/drive.appdata".into(),
    );
    let _ = js_sys::Reflect::set(&opts, &"callback".into(), callback.as_ref());
    let client = init
        .call1(&oauth, &opts)
        .map_err(|_| "Google refused to start sign-in. The existing client must allow https://rrkennedy69.github.io as a JavaScript origin. No new client was created.".to_string())?;
    callback.forget();
    let request = js_sys::Reflect::get(&client, &"requestAccessToken".into())
        .map_err(|_| "Google sign-in did not load.".to_string())?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| "Google sign-in did not load.".to_string())?;
    let prompt = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&prompt, &"prompt".into(), &"".into());
    request
        .call1(&client, &prompt)
        .map_err(|_| "Google refused this site. The existing client must allow https://rrkennedy69.github.io as a JavaScript origin. No new client was created.".to_string())?;
    let value = receiver_wait(receiver).await?;
    let token = js_sys::Reflect::get(&value, &"access_token".into()).ok();
    let Some(token) = token.and_then(|value| value.as_string()) else {
        let err = js_sys::Reflect::get(&value, &"error".into())
            .ok()
            .and_then(|value| value.as_string());
        return Err(match err.as_deref() {
            Some("origin_mismatch") | Some(_) => "Google refused this site. The existing client must allow https://rrkennedy69.github.io as a JavaScript origin. No new client was created.".into(),
            None => "Google sign-in did not return a token.".into(),
        });
    };
    Ok(token)
}

#[cfg(target_arch = "wasm32")]
async fn load_gis() -> Result<(), String> {
    let window = web_sys::window().ok_or("No browser window.")?;
    if js_sys::Reflect::get(&window, &"google".into())
        .ok()
        .filter(|v| !v.is_undefined())
        .is_some()
    {
        return Ok(());
    }
    let document = window.document().ok_or("No document.")?;
    let script = document
        .create_element("script")
        .map_err(|_| "Could not load Google sign-in.".to_string())?;
    script
        .set_attribute("src", "https://accounts.google.com/gsi/client")
        .ok();
    script.set_attribute("async", "true").ok();
    let head = document.head().ok_or("No document head.")?;
    head.append_child(&script).ok();
    for _ in 0..50 {
        sleep_ms(100).await;
        if js_sys::Reflect::get(&window, &"google".into())
            .ok()
            .filter(|v| !v.is_undefined())
            .is_some()
        {
            return Ok(());
        }
    }
    Err("Could not load Google sign-in.".into())
}

#[cfg(target_arch = "wasm32")]
async fn authed_get(token: &str, url: &str) -> Result<String, String> {
    let window = web_sys::window().ok_or("No browser window.")?;
    let init = web_sys::RequestInit::new();
    init.set_method("GET");
    let headers = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &headers,
        &"Authorization".into(),
        &format!("Bearer {token}").into(),
    );
    init.set_headers(&headers);
    let request = web_sys::Request::new_with_str_and_init(url, &init)
        .map_err(|_| "Drive request failed.".to_string())?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| "Drive request failed.".to_string())?;
    let response: web_sys::Response = response
        .dyn_into()
        .map_err(|_| "Drive request failed.".to_string())?;
    if !response.ok() {
        return Err(drive_error(response.status()));
    }
    let text = wasm_bindgen_futures::JsFuture::from(
        response
            .text()
            .map_err(|_| "Drive request failed.".to_string())?,
    )
    .await
    .map_err(|_| "Drive request failed.".to_string())?;
    text.as_string()
        .ok_or_else(|| "Drive request failed.".into())
}

#[cfg(target_arch = "wasm32")]
async fn authed_post(
    token: &str,
    url: &str,
    content_type: &str,
    body: &str,
) -> Result<u16, String> {
    let window = web_sys::window().ok_or("No browser window.")?;
    let init = web_sys::RequestInit::new();
    init.set_method("POST");
    let headers = js_sys::Object::new();
    let _ = js_sys::Reflect::set(
        &headers,
        &"Authorization".into(),
        &format!("Bearer {token}").into(),
    );
    let _ = js_sys::Reflect::set(&headers, &"Content-Type".into(), &content_type.into());
    init.set_headers(&headers);
    init.set_body(&wasm_bindgen::JsValue::from_str(body));
    let request = web_sys::Request::new_with_str_and_init(url, &init)
        .map_err(|_| "Drive request failed.".to_string())?;
    let response = wasm_bindgen_futures::JsFuture::from(window.fetch_with_request(&request))
        .await
        .map_err(|_| "Drive request failed.".to_string())?;
    let response: web_sys::Response = response
        .dyn_into()
        .map_err(|_| "Drive request failed.".to_string())?;
    Ok(response.status())
}

#[cfg(target_arch = "wasm32")]
fn futures_channel() -> (ChannelSender, ChannelReceiver) {
    let slot = Rc::new(RefCell::new(None));
    (ChannelSender(slot.clone()), ChannelReceiver(slot))
}

#[cfg(target_arch = "wasm32")]
struct ChannelSender(Rc<RefCell<Option<wasm_bindgen::JsValue>>>);
#[cfg(target_arch = "wasm32")]
struct ChannelReceiver(Rc<RefCell<Option<wasm_bindgen::JsValue>>>);

#[cfg(target_arch = "wasm32")]
fn sender_send(sender: &ChannelSender, value: wasm_bindgen::JsValue) {
    *sender.0.borrow_mut() = Some(value);
}

#[cfg(target_arch = "wasm32")]
async fn receiver_wait(receiver: ChannelReceiver) -> Result<wasm_bindgen::JsValue, String> {
    for _ in 0..600 {
        if let Some(value) = receiver.0.borrow_mut().take() {
            return Ok(value);
        }
        sleep_ms(100).await;
    }
    Err("Google sign-in was cancelled.".into())
}

#[cfg(target_arch = "wasm32")]
async fn sleep_ms(ms: i32) {
    let promise = js_sys::Promise::new(&mut |resolve, _| {
        let _ = web_sys::window()
            .unwrap()
            .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, ms);
    });
    let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
}
