use rand::prelude::*;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Rect {
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Deserialize)]
struct Cell {
    frame: Rect,
}

#[derive(Deserialize)]
struct Sheet {
    frames: HashMap<String, Cell>,
}

// When the `wee_alloc` feature is enabled, this uses `wee_alloc` as the global
// allocator.
//
// If you don't want to use `wee_alloc`, you can safely delete this.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

// This is like the `main` function, except for JavaScript.
#[wasm_bindgen(start)]
pub fn main_js() -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    // Your code goes here!
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id("canvas")
        .expect("Failed to get #canvas element")
        .dyn_into::<web_sys::HtmlCanvasElement>()?;

    let context = canvas
        .get_context("2d")?
        .unwrap()
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;

    wasm_bindgen_futures::spawn_local(async move {
        let (success_tx, success_rx) = futures::channel::oneshot::channel::<Result<(), JsValue>>();
        let success_tx = Rc::new(Mutex::new(Some(success_tx)));
        let error_tx = Rc::clone(&success_tx);
        let image = web_sys::HtmlImageElement::new().unwrap();

        let callback = Closure::once(move || {
            if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
                success_tx.send(Ok(()));
            }
        });
        let error_callback = Closure::once(move |err| {
            if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
                error_tx.send(Err(err));
            }
        });
        image.set_onload(Some(callback.as_ref().unchecked_ref()));
        image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));

        image.set_src("Idle (1).png");

        success_rx
            .await
            .expect("Failed to receive completed signal");
        context
            .draw_image_with_html_image_element(&image, 0.0, 0.0);

        sierpinski(
            &context,
            [(300.0, 0.0), (0.0, 600.0), (600.0, 600.0)],
            (0, 255, 0),
            2,
        );

        let json = fetch_json("rhb.json")
            .await
            .expect("Failed to fetch json");

        let sheet: Sheet = serde_wasm_bindgen::from_value(json).unwrap();

        let (success_tx, success_rx) = futures::channel::oneshot::channel::<Result<(), JsValue>>();
        let success_tx = Rc::new(Mutex::new(Some(success_tx)));
        let error_tx = Rc::clone(&success_tx);
        let image = web_sys::HtmlImageElement::new().unwrap();

        let callback = Closure::once(move || {
            if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
                success_tx.send(Ok(()));
            }
        });
        let error_callback = Closure::once(move |err| {
            if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
                error_tx.send(Err(err));
            }
        });
        image.set_onload(Some(callback.as_ref().unchecked_ref()));
        image.set_onerror(Some(error_callback.as_ref().unchecked_ref()));

        image.set_src("rhb.png");

        success_rx
            .await
            .expect("Failed to receive completed signal");

        let sprite = sheet.frames.get("Run (1).png").expect("Cell not found");
        context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
            &image,
            sprite.frame.x.into(),
            sprite.frame.y.into(),
            sprite.frame.w.into(),
            sprite.frame.h.into(),
            300.0,
            300.0,
            sprite.frame.w.into(),
            sprite.frame.h.into(),
        );
    });

    Ok(())
}

async fn fetch_json(json_path: &str) -> Result<JsValue, JsValue> {
    let window = web_sys::window().unwrap();
    let resp_value = wasm_bindgen_futures::JsFuture::from(
        window.fetch_with_str(json_path),
    ).await?;
    let resp: web_sys::Response = resp_value.dyn_into()?;

    wasm_bindgen_futures::JsFuture::from(resp.json()?).await
}

fn sierpinski(
    context: &web_sys::CanvasRenderingContext2d,
    points: [(f64, f64); 3],
    color: (u8, u8, u8),
    depth: u8,
) {
    if depth == 0 {
        return;
    }

    let mut rng = thread_rng();
    let next_color = (
        rng.gen_range(0..255),
        rng.gen_range(0..255),
        rng.gen_range(0..255),
    );

    let [top, left, right] = points;

    let left_m = midpoint(top, left);
    let right_m = midpoint(right, top);
    let bottom_m = midpoint(left, right);

    draw_triangle(&context, [top, left_m, right_m], color);
    draw_triangle(&context, [left_m, left, bottom_m], color);
    draw_triangle(&context, [right_m, bottom_m, right], color);

    sierpinski(context, [top, left_m, right_m], next_color, depth - 1);
    sierpinski(context, [left_m, left, bottom_m], next_color, depth - 1);
    sierpinski(context, [right_m, bottom_m, right], next_color, depth - 1);
}

fn midpoint(a: (f64, f64), b: (f64, f64)) -> (f64, f64) {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

fn draw_triangle(
    context: &web_sys::CanvasRenderingContext2d,
    points: [(f64, f64); 3],
    color: (u8, u8, u8),
) {
    let color_str = format!("rgb({},{},{})", color.0, color.1, color.2);
    context.set_fill_style_str(color_str.as_str());

    let [top, left, right] = points;

    context.move_to(top.0, top.1);
    context.begin_path();
    context.line_to(left.0, left.1);
    context.line_to(right.0, right.1);
    context.line_to(top.0, top.1);
    context.close_path();
    context.stroke();
    context.fill();
}
