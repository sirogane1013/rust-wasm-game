use crate::engine::{Game, KeyState, Point, Rect, Renderer, Sheet};
use crate::game::red_hat_boy_states::*;
use crate::{browser, engine};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use web_sys::HtmlImageElement;

pub struct WalkTheDog {
    image: Option<HtmlImageElement>,
    sheet: Option<Sheet>,
    frame: u8,
    position: Point,
    rhb: Option<RedHatBoy>,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog {
            image: None,
            sheet: None,
            frame: 0,
            position: Point { x: 0, y: 0 },
            rhb: None,
        }
    }
}

struct RedHatBoy {
    state_machine: RedHatBoyStateMachine,
    sprite_sheet: Sheet,
    image: HtmlImageElement,
}

impl RedHatBoy {
    fn new(sheet: Sheet, image: HtmlImageElement) -> Self {
        RedHatBoy {
            state_machine: RedHatBoyStateMachine::Idle(RedHatBoyState::new()),
            sprite_sheet: sheet,
            image,
        }
    }
}

#[derive(Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
}

pub enum Event {
    Run,
}

impl RedHatBoyStateMachine {
    fn transition(self, event: Event) -> Self {
        match (self, event) {
            (RedHatBoyStateMachine::Idle(state), Event::Run) => state.run().into(),
            _ => self,
        }
    }
}

impl From<RedHatBoyState<Running>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Running>) -> Self {
        RedHatBoyStateMachine::Running(state)
    }
}

mod red_hat_boy_states {
    use crate::engine::Point;
    const FLOOR: i16 = 475;

    #[derive(Copy, Clone)]
    pub struct RedHatBoyState<S> {
        context: RedHatBoyContext,
        _state: S,
    }

    #[derive(Copy, Clone)]
    pub struct RedHatBoyContext {
        frame: u8,
        position: Point,
        velocity: Point,
    }

    #[derive(Copy, Clone)]
    pub struct Idle;
    #[derive(Copy, Clone)]
    pub struct Running;

    impl RedHatBoyState<Idle> {
        pub fn new() -> Self {
            RedHatBoyState {
                context: RedHatBoyContext {
                    frame: 0,
                    position: Point { x: 0, y: FLOOR },
                    velocity: Point { x: 0, y: 0 },
                },
                _state: Idle,
            }
        }

        pub fn run(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context,
                _state: Running {},
            }
        }
    }
}

#[async_trait(?Send)]
impl Game for WalkTheDog {
    async fn initialize(&self) -> Result<Box<dyn Game>> {
        let sheet: Option<Sheet> =
            serde_wasm_bindgen::from_value(browser::fetch_json("rhb.json").await?)
                .expect("Unable to deserialize rhb.json");

        let image = Some(engine::load_image("rhb.png").await?);

        Ok(Box::new(WalkTheDog {
            image: image.clone(),
            sheet: sheet.clone(),
            frame: self.frame,
            position: self.position,
            rhb: Some(RedHatBoy::new(
                sheet.clone().ok_or_else(|| anyhow!("No Sheet Present"))?,
                image.clone().ok_or_else(|| anyhow!("No Image Present"))?,
            )),
        }))
    }

    fn update(&mut self, key_state: &KeyState) {
        let mut velocity = Point { x: 0, y: 0 };
        if key_state.is_pressed("ArrowDown") {
            velocity.y += 3;
        }
        if key_state.is_pressed("ArrowUp") {
            velocity.y -= 3;
        }
        if key_state.is_pressed("ArrowRight") {
            velocity.x += 3;
        }
        if key_state.is_pressed("ArrowLeft") {
            velocity.x -= 3;
        }

        self.position.x += velocity.x;
        self.position.y += velocity.y;

        if self.frame < 23 {
            self.frame += 1;
        } else {
            self.frame = 0;
        }
    }

    fn draw(&self, renderer: &Renderer) {
        let current_sprite = (self.frame / 3) + 1;
        let frame_name = format!("Run ({}).png", current_sprite);
        let sprite = self
            .sheet
            .as_ref()
            .and_then(|sheet| sheet.frames.get(&frame_name))
            .expect("Cell not found");

        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 600.0,
        });

        self.image.as_ref().map(|image| {
            renderer
                .draw_image(
                    &image,
                    &Rect {
                        x: sprite.frame.x.into(),
                        y: sprite.frame.y.into(),
                        w: sprite.frame.w.into(),
                        h: sprite.frame.h.into(),
                    },
                    &Rect {
                        x: self.position.x.into(),
                        y: self.position.y.into(),
                        w: sprite.frame.w.into(),
                        h: sprite.frame.h.into(),
                    },
                )
                .expect("failed to draw image");
        });
    }
}
