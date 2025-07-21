use anyhow::{anyhow, Result};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::HtmlElement;
use crate::browser::{document, fetch_response};

pub fn draw_ui(html: &str) -> Result<()> {
    Ok(())
}

pub fn hide_ui() -> Result<()> {
    Ok(())
}

pub fn find_html_element_by_id(id: &str) -> Result<HtmlElement> {
    Err(anyhow!("Not implemented yet!"))
}

pub async fn fetch_json(json_path: &str) -> Result<JsValue> {
    Err(anyhow!("Not implemented yet!"))
}
