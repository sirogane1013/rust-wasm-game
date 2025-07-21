use crate::browser::LoopClosure;
use crate::{browser, sound};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::channel::oneshot::channel;
use futures::SinkExt;
use serde::Deserialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Mutex;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{AudioBuffer, AudioContext, CanvasRenderingContext2d, HtmlElement, HtmlImageElement};

#[derive(Deserialize, Clone)]
pub struct SheetRect {
    pub x: i16,
    pub y: i16,
    pub w: i16,
    pub h: i16,
}

#[derive(Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub frame: SheetRect,
    pub sprite_source_size: SheetRect,
}

#[derive(Deserialize, Clone)]
pub struct Sheet {
    pub frames: HashMap<String, Cell>,
}

pub struct Image {
    element: HtmlImageElement,
    bounding_box: Rect,
}

pub struct SpriteSheet {
    sheet: Sheet,
    image: HtmlImageElement,
}

#[async_trait(?Send)]
pub trait Game {
    async fn initialize(&self) -> Result<Box<dyn Game>>;
    fn update(&mut self, key_state: &KeyState);
    fn draw(&self, renderer: &Renderer);
}

const FRAME_SIZE: f32 = 1.0 / 60.0 * 1000.0;

pub struct GameLoop {
    last_frame: f64,
    accumulated_delta: f32,
}

pub struct Renderer {
    context: CanvasRenderingContext2d,
}

enum KeyPress {
    KeyUp(web_sys::KeyboardEvent),
    KeyDown(web_sys::KeyboardEvent),
}

pub struct KeyState {
    pressed_keys: HashMap<String, web_sys::KeyboardEvent>,
}

#[derive(Clone, Copy, Default)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl SpriteSheet {
    pub fn new(sheet: Sheet, image: HtmlImageElement) -> Self {
        SpriteSheet { sheet, image }
    }

    pub fn cell(&self, name: &str) -> Option<&Cell> {
        self.sheet.frames.get(name)
    }

    pub fn draw(&self, renderer: &Renderer, source: &Rect, destination: &Rect) -> Result<()> {
        renderer.draw_image(&self.image, source, destination)
    }
}

impl Image {
    pub fn new(element: HtmlImageElement, position: Point) -> Self {
        let bounding_box = Rect::new(position, element.width() as i16, element.height() as i16);
        Self {
            element,
            bounding_box,
        }
    }

    pub fn bounding_box(&self) -> &Rect {
        &self.bounding_box
    }

    pub fn draw(&self, renderer: &Renderer) -> Result<()> {
        renderer.draw_entire_image(&self.element, self.bounding_box.position)
    }

    pub fn draw_bounding_box(&self, renderer: &Renderer) {
        renderer.draw_rect(&self.bounding_box)
    }

    pub fn right(&self) -> i16 {
        self.bounding_box.right()
    }

    pub fn move_horizontally(&mut self, distance: i16) {
        self.set_x(self.bounding_box().x() + distance);
    }

    pub fn set_x(&mut self, x: i16) {
        self.bounding_box.set_x(x);
    }
}

impl KeyState {
    fn new() -> Self {
        KeyState {
            pressed_keys: HashMap::new(),
        }
    }

    pub fn is_pressed(&self, code: &str) -> bool {
        self.pressed_keys.contains_key(code)
    }

    fn set_pressed(&mut self, code: &str, event: web_sys::KeyboardEvent) {
        self.pressed_keys.insert(code.into(), event);
    }

    fn set_released(&mut self, code: &str) {
        self.pressed_keys.remove(code.into());
    }
}

type SharedLoopClosure = Rc<RefCell<Option<LoopClosure>>>;

impl GameLoop {
    pub async fn start(game: impl Game + 'static) -> Result<()> {
        let mut keyevent_receiver = prepare_input()?;
        let mut game = game.initialize().await?;

        let mut game_loop = GameLoop {
            last_frame: browser::now()?,
            accumulated_delta: 0.0,
        };

        let renderer = Renderer {
            context: browser::context()?,
        };

        let f: SharedLoopClosure = Rc::new(RefCell::new(None));
        let g = f.clone();

        let mut key_state = KeyState::new();
        *g.borrow_mut() = Some(browser::create_raf_closure(move |perf: f64| {
            process_input(&mut key_state, &mut keyevent_receiver);
            game_loop.accumulated_delta += (perf - game_loop.last_frame) as f32;
            while game_loop.accumulated_delta > FRAME_SIZE {
                game.update(&key_state);
                game_loop.accumulated_delta -= FRAME_SIZE;
            }
            game_loop.last_frame = perf;
            game.draw(&renderer);

            browser::request_animation_frame(f.borrow().as_ref().unwrap());
        }));

        browser::request_animation_frame(
            g.borrow()
                .as_ref()
                .ok_or_else(|| anyhow!("GameLoop: Loop is None"))?,
        )?;
        Ok(())
    }
}

#[derive(Default)]
pub struct Rect {
    pub position: Point,
    pub w: i16,
    pub h: i16,
}

impl Rect {
    pub const fn new(position: Point, w: i16, h: i16) -> Self {
        Rect { position, w, h }
    }

    pub const fn new_from_x_y(x: i16, y: i16, w: i16, h: i16) -> Self {
        Rect::new(Point { x, y }, w, h)
    }

    pub fn x(&self) -> i16 {
        self.position.x
    }

    pub fn set_x(&mut self, x: i16) {
        self.position.x = x;
    }

    pub fn y(&self) -> i16 {
        self.position.y
    }

    pub fn set_y(&mut self, y: i16) {
        self.position.y = y;
    }

    pub fn interests(&self, rect: &Rect) -> bool {
        self.x() < rect.right()
            && self.right() > rect.x()
            && self.y() < rect.bottom()
            && self.bottom() > rect.y()
    }

    pub fn right(&self) -> i16 {
        self.x() + self.w
    }

    pub fn bottom(&self) -> i16 {
        self.y() + self.h
    }
}

impl Renderer {
    pub fn clear(&self, rect: &Rect) {
        self.context.clear_rect(
            rect.x().into(),
            rect.y().into(),
            rect.w.into(),
            rect.h.into(),
        )
    }

    pub fn draw_entire_image(&self, image: &HtmlImageElement, position: Point) -> Result<()> {
        self.context
            .draw_image_with_html_image_element(image, position.x.into(), position.y.into())
            .map_err(|e| anyhow!("Failed to draw image: {:#?}", e))
    }

    pub fn draw_image(
        &self,
        image: &HtmlImageElement,
        frame: &Rect,
        destination: &Rect,
    ) -> Result<()> {
        self.context
            .draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &image,
                frame.x().into(),
                frame.y().into(),
                frame.w.into(),
                frame.h.into(),
                destination.x().into(),
                destination.y().into(),
                destination.w.into(),
                destination.h.into(),
            )
            .map_err(|e| anyhow!("Failed to draw image: {:#?}", e))
    }

    pub fn draw_rect(&self, bounding_box: &Rect) {
        self.context.set_stroke_style(&JsValue::from_str("#FF0000"));
        self.context.begin_path();
        self.context.rect(
            bounding_box.x().into(),
            bounding_box.y().into(),
            bounding_box.w.into(),
            bounding_box.h.into(),
        );
        self.context.stroke();
    }
}

pub async fn load_image(source: &str) -> Result<HtmlImageElement> {
    let image = browser::new_image()?;

    let (complete_tx, complete_rx) = channel::<Result<()>>();
    let success_tx = Rc::new(Mutex::new(Some(complete_tx)));
    let error_tx = Rc::clone(&success_tx);
    let success_callback = browser::closure_once(move || {
        if let Some(success_tx) = success_tx.lock().ok().and_then(|mut opt| opt.take()) {
            success_tx.send(Ok(()));
        }
    });

    let error_callback: Closure<dyn FnMut(JsValue)> = browser::closure_once(move |err| {
        if let Some(error_tx) = error_tx.lock().ok().and_then(|mut opt| opt.take()) {
            error_tx.send(Err(anyhow!("Error loading image: {:#?}", err)));
        }
    });

    image.set_onload(Some(&success_callback.as_ref().unchecked_ref()));
    image.set_onerror(Some(&error_callback.as_ref().unchecked_ref()));
    image.set_src(source);

    complete_rx.await??;

    Ok(image)
}

fn prepare_input() -> Result<UnboundedReceiver<KeyPress>> {
    let (keydown_sender, keyevent_receiver) = unbounded();
    let keydown_sender = Rc::new(RefCell::new(keydown_sender));
    let keyup_sender = Rc::clone(&keydown_sender);

    let onkeydown = browser::closure_wrap(Box::new(move |keycode: web_sys::KeyboardEvent| {
        keydown_sender
            .borrow_mut()
            .start_send(KeyPress::KeyDown(keycode))
            .expect("keydown_sender cannot send keydown event");
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    let onkeyup = browser::closure_wrap(Box::new(move |keycode: web_sys::KeyboardEvent| {
        keyup_sender
            .borrow_mut()
            .start_send(KeyPress::KeyUp(keycode))
            .expect("keyup_sender cannot send keyup event");
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);

    browser::canvas()?.set_onkeydown(Some(onkeydown.as_ref().unchecked_ref()));
    browser::canvas()?.set_onkeyup(Some(onkeyup.as_ref().unchecked_ref()));
    onkeydown.forget();
    onkeyup.forget();
    Ok(keyevent_receiver)
}

fn process_input(state: &mut KeyState, keyevent_receiver: &mut UnboundedReceiver<KeyPress>) {
    loop {
        match keyevent_receiver.try_next() {
            Ok(None) => break,
            Err(_err) => break,
            Ok(Some(evt)) => match evt {
                KeyPress::KeyUp(evt) => state.set_released(&evt.code()),
                KeyPress::KeyDown(evt) => state.set_pressed(&evt.code(), evt),
            },
        }
    }
}

#[derive(Clone)]
pub struct Audio {
    context: AudioContext,
}

#[derive(Clone)]
pub struct Sound {
    buffer: AudioBuffer,
}

impl Audio {
    pub fn new() -> Result<Self> {
        Ok(Audio {
            context: sound::create_audio_context()?,
        })
    }

    pub async fn load_sound(&self, file_name: &str) -> Result<Sound> {
        let array_buffer = browser::fetch_array_buffer(file_name).await?;
        let audio_buffer = sound::decode_audio_data(&self.context, &array_buffer).await?;
        Ok(Sound {
            buffer: audio_buffer,
        })
    }

    pub fn play_sound(&self, sound: &Sound) -> Result<()> {
        sound::play_sound(&self.context, &sound.buffer, sound::LOOPING::NO)
    }

    pub fn play_looping_sound(&self, sound: &Sound) -> Result<()> {
        sound::play_sound(&self.context, &sound.buffer, sound::LOOPING::YES)
    }
}

pub fn add_click_handler(elem: HtmlElement) -> UnboundedReceiver<()> {
    let (mut click_sender, click_receiver) = unbounded();
    let on_click = browser::closure_wrap(Box::new(move || {
        click_sender.start_send(()).expect("Failed to start send");
    }) as Box<dyn FnMut()>);
    elem.set_onclick(Some(on_click.as_ref().unchecked_ref()));
    on_click.forget();
    click_receiver
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_rects_that_intersect_on_the_left() {
        let rect1 = Rect {
            position: Point { x: 10, y: 10 },
            w: 100,
            h: 100,
        };
        let rect2 = Rect {
            position: Point { x: 0, y: 10 },
            w: 100,
            h: 100,
        };

        assert_eq!(rect2.interests(&rect1), true)
    }

    #[test]
    fn two_rects_that_intersect_on_the_bottom() {
        let rect1 = Rect {
            position: Point { x: 10, y: 10 },
            w: 100,
            h: 100,
        };
        let rect2 = Rect {
            position: Point { x: 10, y: 0 },
            w: 100,
            h: 100,
        };

        assert_eq!(rect2.interests(&rect1), true)
    }

    #[test]
    fn two_rects_that_doesnt_intersect() {
        let rect1 = Rect {
            position: Point { x: 0, y: 0 },
            w: 100,
            h: 100,
        };
        let rect2 = Rect {
            position: Point { x: 100, y: 100 },
            w: 100,
            h: 100,
        };

        assert_eq!(rect2.interests(&rect1), false)
    }
}
