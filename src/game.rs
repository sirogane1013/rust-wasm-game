use crate::engine::{Game, KeyState, Rect, Renderer, Sheet};
use crate::game::red_hat_boy_states::*;
use crate::{browser, engine};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use serde::Deserialize;
use web_sys::HtmlImageElement;

pub struct WalkTheDog {
    rhb: Option<RedHatBoy>,
}

impl WalkTheDog {
    pub fn new() -> Self {
        WalkTheDog { rhb: None }
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

    fn update(&mut self) {
        self.state_machine = self.state_machine.update()
    }

    fn draw(&self, renderer: &Renderer) {
        let frame_name = format!(
            "{} ({}).png",
            self.state_machine.frame_name(),
            (self.state_machine.context().frame / 3) + 1,
        );
        let sprite = self
            .sprite_sheet
            .frames
            .get(&frame_name)
            .expect("Cell not found");

        renderer
            .draw_image(
                &self.image,
                &Rect {
                    x: sprite.frame.x.into(),
                    y: sprite.frame.y.into(),
                    w: sprite.frame.w.into(),
                    h: sprite.frame.h.into(),
                },
                &Rect {
                    x: self.state_machine.context().position.x.into(),
                    y: self.state_machine.context().position.y.into(),
                    w: sprite.frame.w.into(),
                    h: sprite.frame.h.into(),
                },
            )
            .expect("failed to draw image");
    }

    fn run_right(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Run)
    }

    fn back_left(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Back)
    }

    fn slide(&mut self) {
        self.state_machine = self.state_machine.transition(Event::Slide)
    }
}

#[derive(Copy, Clone)]
enum RedHatBoyStateMachine {
    Idle(RedHatBoyState<Idle>),
    Running(RedHatBoyState<Running>),
    Backing(RedHatBoyState<Backing>),
    Sliding(RedHatBoyState<Sliding>),
}

pub enum Event {
    Run,
    Back,
    Slide,
    Update
}

impl RedHatBoyStateMachine {
    fn transition(self, event: Event) -> Self {
        match (self, event) {
            (RedHatBoyStateMachine::Idle(state), Event::Run) => state.run().into(),
            (RedHatBoyStateMachine::Idle(state), Event::Back) => state.back().into(),
            (RedHatBoyStateMachine::Idle(_), Event::Slide) => self,
            (RedHatBoyStateMachine::Running(state), Event::Back) => state.back().into(),
            (RedHatBoyStateMachine::Running(_), Event::Run) => self,
            (RedHatBoyStateMachine::Running(state), Event::Slide) => state.slide().into(),
            (RedHatBoyStateMachine::Backing(state), Event::Run) => state.run().into(),
            (RedHatBoyStateMachine::Backing(_), Event::Back) => self,
            (RedHatBoyStateMachine::Backing(_), Event::Slide) => self,
            (RedHatBoyStateMachine::Sliding(_), Event::Run) => self,
            (RedHatBoyStateMachine::Sliding(_), Event::Back) => self,
            (RedHatBoyStateMachine::Sliding(_), Event::Slide) => self,
            (RedHatBoyStateMachine::Idle(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Running(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Backing(state), Event::Update) => state.update().into(),
            (RedHatBoyStateMachine::Sliding(state), Event::Update) => state.update().into(),
        }
    }

    fn update(self) -> Self {
        self.transition(Event::Update)
    }

    fn frame_name(&self) -> &str {
        match self {
            RedHatBoyStateMachine::Idle(state) => state.frame_name(),
            RedHatBoyStateMachine::Running(state) => state.frame_name(),
            RedHatBoyStateMachine::Backing(state) => state.frame_name(),
            RedHatBoyStateMachine::Sliding(state) => state.frame_name(),
        }
    }

    fn context(&self) -> &RedHatBoyContext {
        match self {
            RedHatBoyStateMachine::Idle(state) => &state.context(),
            RedHatBoyStateMachine::Running(state) => &state.context(),
            RedHatBoyStateMachine::Backing(state) => &state.context(),
            RedHatBoyStateMachine::Sliding(state) => &state.context(),
        }
    }
}

impl From<RedHatBoyState<Idle>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Idle>) -> Self {
        RedHatBoyStateMachine::Idle(state)
    }
}

impl From<RedHatBoyState<Running>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Running>) -> Self {
        RedHatBoyStateMachine::Running(state)
    }
}

impl From<RedHatBoyState<Backing>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Backing>) -> Self {
        RedHatBoyStateMachine::Backing(state)
    }
}

impl From<RedHatBoyState<Sliding>> for RedHatBoyStateMachine {
    fn from(state: RedHatBoyState<Sliding>) -> Self {
        RedHatBoyStateMachine::Sliding(state)
    }
}

impl From<SlidingEndState> for RedHatBoyStateMachine {
    fn from(end_state: SlidingEndState) -> Self {
        match end_state {
            SlidingEndState::Complete(running_state) => running_state.into(),
            SlidingEndState::Sliding(sliding_state) => sliding_state.into(),
        }
    }
}

mod red_hat_boy_states {
    use crate::engine::Point;
    const FLOOR: i16 = 475;
    const IDLE_FRAME_NAME: &str = "Idle";
    const RUN_FRAME_NAME: &str = "Run";
    const SLIDING_FRAME_NAME: &str = "Slide";
    const IDLE_FRAMES: u8 = 29;
    const RUNNING_FRAMES: u8 = 23;
    const SLIDING_FRAMES: u8 = 14;
    const RUNNING_SPEED: i16 = 3;

    #[derive(Copy, Clone)]
    pub struct RedHatBoyState<S> {
        context: RedHatBoyContext,
        _state: S,
    }

    #[derive(Copy, Clone)]
    pub struct RedHatBoyContext {
        pub frame: u8,
        pub position: Point,
        pub velocity: Point,
    }

    impl RedHatBoyContext {
        fn update(mut self, frame_count: u8) -> Self {
            if self.frame < frame_count {
                self.frame += 1;
            } else {
                self.frame = 0;
            }
            self.position.x += self.velocity.x;
            self.position.y += self.velocity.y;
            self
        }

        fn reset_frame(mut self) -> Self {
            self.frame = 0;
            self
        }

        fn run_right(mut self) -> Self {
            self.velocity.x = RUNNING_SPEED;
            self
        }

        fn back_left(mut self) -> Self {
            self.velocity.x = -RUNNING_SPEED;
            self
        }
    }

    #[derive(Copy, Clone)]
    pub struct Idle;
    #[derive(Copy, Clone)]
    pub struct Running;
    #[derive(Copy, Clone)]
    pub struct Backing;
    #[derive(Copy, Clone)]
    pub struct Sliding;

    impl<S> RedHatBoyState<S> {
        pub fn context(&self) -> &RedHatBoyContext {
            &self.context
        }
    }

    impl RedHatBoyState<Idle> {
        pub fn update(mut self) -> Self {
            self.context = self.context.update(IDLE_FRAMES);
            self
        }

        pub fn frame_name(&self) -> &str {
            IDLE_FRAME_NAME
        }

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
                context: self.context.reset_frame().run_right(),
                _state: Running {},
            }
        }

        pub fn back(self) -> RedHatBoyState<Backing> {
            RedHatBoyState {
                context: self.context.reset_frame().back_left(),
                _state: Backing {},
            }
        }
    }

    impl RedHatBoyState<Running> {
        pub fn update(mut self) -> Self {
            self.context = self.context.update(RUNNING_FRAMES);
            self
        }

        pub fn frame_name(&self) -> &str {
            RUN_FRAME_NAME
        }

        pub fn back(self) -> RedHatBoyState<Backing> {
            RedHatBoyState {
                context: self.context.reset_frame().back_left(),
                _state: Backing {},
            }
        }

        pub fn slide(self) -> RedHatBoyState<Sliding> {
            RedHatBoyState {
                context: self.context.reset_frame(),
                _state: Sliding {}
            }
        }
    }

    impl RedHatBoyState<Backing> {
        pub fn update(mut self) -> Self {
            self.context = self.context.update(RUNNING_FRAMES);
            self
        }

        pub fn frame_name(&self) -> &str {
            RUN_FRAME_NAME
        }

        pub fn run(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame().run_right(),
                _state: Running {},
            }
        }
    }

    impl RedHatBoyState<Sliding> {
        pub fn update(mut self) -> SlidingEndState {
            self.context = self.context.update(SLIDING_FRAMES);

            if self.context.frame >= SLIDING_FRAMES {
                SlidingEndState::Complete(self.stand())
            } else {
                SlidingEndState::Sliding(self)
            }
        }

        pub fn frame_name(&self) -> &str {
            SLIDING_FRAME_NAME
        }

        pub fn stand(self) -> RedHatBoyState<Running> {
            RedHatBoyState {
                context: self.context.reset_frame(),
                _state: Running {},
            }
        }
    }

    pub enum SlidingEndState {
        Complete(RedHatBoyState<Running>),
        Sliding(RedHatBoyState<Sliding>),
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
            rhb: Some(RedHatBoy::new(
                sheet.clone().ok_or_else(|| anyhow!("No Sheet Present"))?,
                image.clone().ok_or_else(|| anyhow!("No Image Present"))?,
            )),
        }))
    }

    fn update(&mut self, key_state: &KeyState) {
        if key_state.is_pressed("ArrowDown") {
            self.rhb.as_mut().unwrap().slide();
        }
        if key_state.is_pressed("ArrowUp") {}
        if key_state.is_pressed("ArrowRight") {
            self.rhb.as_mut().unwrap().run_right();
        }
        if key_state.is_pressed("ArrowLeft") {
            self.rhb.as_mut().unwrap().back_left();
        }

        self.rhb.as_mut().unwrap().update()
    }

    fn draw(&self, renderer: &Renderer) {
        renderer.clear(&Rect {
            x: 0.0,
            y: 0.0,
            w: 600.0,
            h: 600.0,
        });

        self.rhb.as_ref().unwrap().draw(renderer);
    }
}
