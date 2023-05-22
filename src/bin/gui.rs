//! This example showcases an interactive `Canvas` for drawing Bézier curves.
use std::sync::mpsc::{self, Sender, TryRecvError};
use std::thread;
use std::time::Duration;

use adh_rs::audio_bridge::{play, BlendType, SampleChunks};
use iced::widget::{button, column, text};
use iced::window::{self, Position};
use iced::{
    executor, theme, Alignment, Application, Command, Element, Length, Sandbox, Settings,
    Subscription,
};

use adh_rs::{Weights, SEGMENTS_WEIGHT_MAX, WEIGHTS_NUM};
use equalizer::canvas_size;

const SEGMENTS_WIDTH: f32 = 10.0;
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

    TrayUtility::run(Settings {
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

// This application is meant to be used as a small floating window above the system tray.
#[derive(Debug, Default)]
struct TrayUtility {
    equalizer: equalizer::State,
    weights: Weights,
    send_close: Option<Sender<()>>,
}

#[derive(Debug, Clone, Copy)]
enum Message {
    SetWeight((usize, f32)),
    ConfirmWeights,
    Clear,
    ExitApplication,
}

impl Application for TrayUtility {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = theme::Theme;

    fn new(_: ()) -> (Self, Command<Message>) {
        (TrayUtility::default(), Command::none())
    }

    fn title(&self) -> String {
        String::from("Equalizer")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SetWeight((i, weight)) => {
                self.weights.v.get_mut(i).map(|w| *w = weight);
                self.equalizer.request_redraw();
            }
            Message::ConfirmWeights => {
                println!("Sending weights to backend.");
                if let Some(ref tx) = self.send_close {
                    tx.send(()).ok();
                }

                let (tx, rx) = mpsc::channel();
                self.send_close = Some(tx);

                thread::spawn({
                    let t_weights = self.weights.clone();

                    move || {
                        let samples1 = adh_rs::generator::gen_weighted_noise(&t_weights);
                        let samples2 = adh_rs::generator::gen_weighted_noise(&t_weights);
                        // let chunks = SampleChunks::new(samples1).unwrap();
                        let chunks = SampleChunks::new(vec![samples1, samples2])
                            .unwrap()
                            .with_blend(BlendType::Sigmoid);
                        let audio_stream = play(chunks);

                        loop {
                            match rx.try_recv() {
                                Ok(()) | Err(TryRecvError::Disconnected) => return,
                                _ => {}
                            };
                            thread::sleep(Duration::from_secs(1));
                        }
                    }
                });
            }
            Message::Clear => {
                self.equalizer = equalizer::State::default();
                self.weights = Weights::default();
            }
            Message::ExitApplication => {
                if let Some(ref tx) = self.send_close {
                    tx.send(()).ok();
                }
                return window::close();
            }
        };

        Command::none()
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

    fn subscription(&self) -> Subscription<Message> {
        // Subscription::batch([
        // subscribe to keyboard events
        iced_native::subscription::events_with(|event, status| match (status, event) {
            /* event when closing the window e.g. mod+Shift+q in i3 */
            (_, iced_native::Event::Window(iced_native::window::Event::CloseRequested)) => {
                Some(Message::ExitApplication)
            }
            (_, _) => None,
        })
        // ])
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
            // Change control status if left mouse button is pressed/released.
            // TODO can we get the current mouse button status in iced? Maybe we would have to add it.
            match event {
                Event::Mouse(mouse_event) => match mouse_event {
                    mouse::Event::ButtonPressed(mouse::Button::Left) => {
                        *state = ControlStatus::Active;
                    }
                    mouse::Event::CursorLeft
                    | mouse::Event::ButtonReleased(mouse::Button::Left)
                        if state.is_active() =>
                    {
                        *state = ControlStatus::Inactive;
                        return (event::Status::Ignored, Some(Message::ConfirmWeights));
                    }
                    // mouse::Event::CursorLeft
                    // | mouse::Event::ButtonReleased(mouse::Button::Left)
                    //     if !state.is_active() =>
                    // {
                    //     return (event::Status::Ignored, None);
                    // }
                    _ => {}
                },
                _ => {}
            };

            let cursor_position = if let Some(position) = cursor.position_in(&bounds) {
                position
            } else {
                return (event::Status::Ignored, None);
            };

            match event {
                Event::Mouse(mouse_event) => {
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
