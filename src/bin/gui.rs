//! This example showcases an interactive `Canvas` for drawing Bézier curves.
use std::fmt::Display;

use equalizer::canvas_size;
use iced::widget::{button, column, text};
use iced::window::Position;
use iced::{Alignment, Element, Length, Sandbox, Settings};

const WEIGHTS_NUM: usize = 32;
const SEGMENTS_WIDTH: f32 = 10.0;
const SEGMENTS_WEIGHT_MAX: f32 = 1.0;
const CANVAS_PADDING: f32 = 20.0;
const WEIGHTS_PADDING_Y: f32 = 20.0;
const CANVAS_HEIGHT: f32 = 200.0;
const SCREEN_PADDING: u32 = 20;

pub fn main() -> iced::Result {
    let (width, height) = canvas_size();
    let window_size = (
        (width + 2.0 * CANVAS_PADDING) as u32,
        (height + 2.0 * CANVAS_PADDING) as u32,
    );
    let screen_size = (1920, 1080);
    let window_position = (
        screen_size.0 - SCREEN_PADDING - window_size.0,
        // TODO calculation does not work as expected
        screen_size.1 - SCREEN_PADDING - window_size.1 - 50,
    );

    Example::run(Settings {
        antialiasing: true,
        window: iced::window::Settings {
            size: window_size,
            max_size: Some(window_size),
            position: Position::Specific(window_position.0 as i32, window_position.1 as i32),
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    })
}

#[derive(Debug, Clone, Copy)]
struct Weights {
    pub v: [f32; WEIGHTS_NUM],
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            v: [SEGMENTS_WEIGHT_MAX; WEIGHTS_NUM],
        }
    }
}

#[derive(Debug, Default)]
struct Example {
    equalizer: equalizer::State,
    weights: Weights,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    SetWeight((usize, f32)),
    ConfirmWeights,
    Clear,
}

impl Sandbox for Example {
    type Message = Message;

    fn new() -> Self {
        Example::default()
    }

    fn title(&self) -> String {
        String::from("Equalizer")
    }

    fn update(&mut self, message: Message) {
        match message {
            Message::SetWeight((i, weight)) => {
                self.weights.v.get_mut(i).map(|w| *w = weight);
                self.equalizer.request_redraw();
            }
            Message::ConfirmWeights => {
                println!("Sending weights to backend.");
                adh_rs::generator::gen_weighted_noise_no_mirror(&self.weights.v);
                // TODO send weights to backend
            }
            Message::Clear => {
                self.equalizer = equalizer::State::default();
                self.weights = Weights::default();
            }
        }
    }

    fn view(&self) -> Element<Message> {
        column![
            self.equalizer.view(&self.weights),
            // button("Clear").padding(8).on_press(Message::Clear),
        ]
        .padding(CANVAS_PADDING)
        .spacing(CANVAS_PADDING)
        .align_items(Alignment::Center)
        .into()
    }
}

mod equalizer {
    use iced::widget::canvas::event::{self, Event};
    use iced::widget::canvas::{
        self, gradient, Canvas, Cursor, Fill, Frame, Geometry, Gradient, Path, Stroke,
    };
    use iced::{mouse, Color, Size};
    use iced::{Element, Length, Point, Rectangle, Theme};

    use crate::{
        Message, Weights, CANVAS_HEIGHT, SEGMENTS_WEIGHT_MAX, SEGMENTS_WIDTH, WEIGHTS_NUM,
        WEIGHTS_PADDING_Y,
    };

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ControlStatus {
        Active,
        Inactive,
    }

    impl ControlStatus {
        fn is_active(self) -> bool {
            match self {
                ControlStatus::Active => true,
                ControlStatus::Inactive => false,
            }
        }
    }

    impl Default for ControlStatus {
        fn default() -> Self {
            Self::Inactive
        }
    }

    #[derive(Debug, Default)]
    pub struct State {
        cache: canvas::Cache,
    }

    pub fn canvas_size() -> (f32, f32) {
        let width = WEIGHTS_NUM as f32 * SEGMENTS_WIDTH;
        let height = CANVAS_HEIGHT + 2.0 * WEIGHTS_PADDING_Y;
        (width, height)
    }

    impl State {
        pub(super) fn view<'a>(&'a self, weights: &'a Weights) -> Element<'a, Message> {
            let (width, height) = canvas_size();

            Canvas::new(Equalizer {
                state: self,
                weights,
            })
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .into()
        }

        pub fn request_redraw(&mut self) {
            self.cache.clear()
        }
    }

    #[derive(Debug)]
    struct Equalizer<'a> {
        state: &'a State,
        weights: &'a Weights,
    }

    fn weight_to_ypos(weight: f32) -> f32 {
        CANVAS_HEIGHT + WEIGHTS_PADDING_Y - (weight * CANVAS_HEIGHT)
    }

    fn ypos_to_weight(y: f32) -> f32 {
        ((CANVAS_HEIGHT + WEIGHTS_PADDING_Y - y) / CANVAS_HEIGHT).clamp(0.0, SEGMENTS_WEIGHT_MAX)
    }

    impl<'a> canvas::Program<Message> for Equalizer<'a> {
        type State = ControlStatus;

        fn update(
            &self,
            state: &mut Self::State,
            event: Event,
            bounds: Rectangle,
            cursor: Cursor,
        ) -> (event::Status, Option<Message>) {
            let cursor_position = if let Some(position) = cursor.position_in(&bounds) {
                position
            } else {
                // Deactivate control status if cursor leaves canvas.
                if state.is_active() {
                    *state = ControlStatus::Inactive;
                    return (event::Status::Ignored, Some(Message::ConfirmWeights));
                } else {
                    return (event::Status::Ignored, None);
                }
            };

            match event {
                Event::Mouse(mouse_event) => {
                    // Change control status if left mouse button is pressed/released.
                    // TODO can we get the current mouse button status in iced? Maybe we would have to add it.
                    match mouse_event {
                        mouse::Event::ButtonPressed(mouse::Button::Left) => {
                            *state = ControlStatus::Active;
                        }
                        mouse::Event::ButtonReleased(mouse::Button::Left) => {
                            if state.is_active() {
                                *state = ControlStatus::Inactive;
                                return (event::Status::Ignored, Some(Message::ConfirmWeights));
                            } else {
                                return (event::Status::Ignored, None);
                            }
                        }
                        _ => {}
                    };

                    let message = match mouse_event {
                        mouse::Event::CursorMoved { .. }
                        | mouse::Event::ButtonPressed(mouse::Button::Left)
                            if state.is_active() =>
                        {
                            // Calculate the segment we are in based on x position of cursor.
                            let segment = (cursor_position.x / SEGMENTS_WIDTH)
                                .floor()
                                .clamp(0.0, 31.0)
                                as usize;
                            let weight = ypos_to_weight(cursor_position.y);

                            let r = Message::SetWeight((segment, weight));
                            // println!("{}", r);
                            Some(r)
                        }
                        _ => None,
                    };

                    (event::Status::Captured, message)
                }
                _ => (event::Status::Ignored, None),
            }
        }

        fn draw(
            &self,
            _state: &Self::State,
            _theme: &Theme,
            bounds: Rectangle,
            _cursor: Cursor,
        ) -> Vec<Geometry> {
            let content = self.state.cache.draw(bounds.size(), |frame: &mut Frame| {
                let (width, height) = canvas_size();

                let gradient = Gradient::linear(gradient::Position::Relative {
                    top_left: Point { x: 0.0, y: 0.0 },
                    size: Size::new(width, height),
                    start: gradient::Location::Left,
                    end: gradient::Location::Right,
                })
                .add_stop(0.0, Color::from_rgb8(0x80, 0, 0))
                .add_stop(0.1, Color::from_rgb8(0xFF, 0, 0))
                .add_stop(0.3, Color::from_rgb8(0xFF, 0xFF, 0))
                .add_stop(0.5, Color::from_rgb8(0x00, 0x80, 0))
                .add_stop(0.7, Color::from_rgb8(0x00, 0x80, 0))
                .add_stop(0.9, Color::from_rgb8(0x00, 0xFF, 0xFF))
                .add_stop(1.0, Color::from_rgb8(0x80, 0, 0x80))
                .build()
                .unwrap();

                for (i, w) in self.weights.v.iter().enumerate() {
                    // Scale weight back into y position on canvas.
                    let x = i as f32 * SEGMENTS_WIDTH;
                    let y = weight_to_ypos(*w);
                    const GRADIENT_PADDING: f32 = 5.0;
                    const THUMB_SIZE: f32 = 3.0;

                    frame.fill_rectangle(
                        Point { x, y: y },
                        Size::new(SEGMENTS_WIDTH, height - WEIGHTS_PADDING_Y - y),
                        gradient.clone(),
                    );

                    frame.fill_rectangle(
                        Point {
                            x,
                            y: y - GRADIENT_PADDING,
                        },
                        Size::new(SEGMENTS_WIDTH, THUMB_SIZE),
                        Fill::default(),
                    );

                    frame.stroke(
                        &Path::line(
                            Point {
                                x,
                                y: y - GRADIENT_PADDING,
                            },
                            Point {
                                x,
                                y: height - WEIGHTS_PADDING_Y,
                            },
                        ),
                        Stroke::default(),
                    );

                    frame.stroke(
                        &Path::line(
                            Point {
                                x: x + SEGMENTS_WIDTH,
                                y: y - GRADIENT_PADDING,
                            },
                            Point {
                                x: x + SEGMENTS_WIDTH,
                                y: height - WEIGHTS_PADDING_Y,
                            },
                        ),
                        Stroke::default(),
                    );
                }

                // Draw a line around the canvas.
                frame.stroke(
                    &Path::rectangle(Point::ORIGIN, frame.size()),
                    Stroke::default().with_width(2.0),
                );
            });

            vec![content]
        }

        fn mouse_interaction(
            &self,
            _state: &Self::State,
            bounds: Rectangle,
            cursor: Cursor,
        ) -> mouse::Interaction {
            if cursor.is_over(&bounds) {
                mouse::Interaction::Crosshair
            } else {
                mouse::Interaction::default()
            }
        }
    }
}
