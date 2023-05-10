//! This example showcases an interactive `Canvas` for drawing Bézier curves.
use iced::widget::{button, column, text};
use iced::window::Position;
use iced::{Alignment, Element, Length, Sandbox, Settings};

const WEIGHTS_NUM: usize = 32;
const segments_width: f32 = 10.0;
const segments_weight_max: f32 = 100.0;

pub fn main() -> iced::Result {
    Example::run(Settings {
        antialiasing: true,
        window: iced::window::Settings {
            size: (380, 200),
            // max_size: Some((250, 150)),
            position: Position::Specific(1600, 800),
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    })
}

struct Example {
    equalizer: equalizer::State,
    weights: [f32; WEIGHTS_NUM],
}

impl Default for Example {
    fn default() -> Self {
        Self {
            equalizer: Default::default(),
            weights: [segments_weight_max; WEIGHTS_NUM],
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    SetWeight((usize, f32)),
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
            Message::SetWeight((i, new_weight)) => {
                self.weights.get_mut(i).map(|weight| *weight = new_weight);
                self.equalizer.request_redraw();
            }
            Message::Clear => {
                self.equalizer = equalizer::State::default();
                self.weights = [segments_weight_max; WEIGHTS_NUM];
            }
        }
    }

    fn view(&self) -> Element<Message> {
        column![
            self.equalizer.view(&self.weights).map(Message::SetWeight),
            // button("Clear").padding(8).on_press(Message::Clear),
        ]
        .padding(20)
        .spacing(20)
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

    use crate::{segments_width, WEIGHTS_NUM};

    #[derive(Default)]
    pub struct State {
        cache: canvas::Cache,
    }

    impl State {
        pub fn view<'a>(&'a self, weights: &'a [f32]) -> Element<'a, (usize, f32)> {
            Canvas::new(Equalizer {
                state: self,
                weights,
            })
            .width(Length::Fixed(WEIGHTS_NUM as f32 * segments_width))
            .height(Length::Fixed(150.0))
            .into()
        }

        pub fn request_redraw(&mut self) {
            self.cache.clear()
        }
    }

    struct Equalizer<'a> {
        state: &'a State,
        weights: &'a [f32],
    }

    impl<'a> canvas::Program<(usize, f32)> for Equalizer<'a> {
        type State = ();

        fn update(
            &self,
            state: &mut Self::State,
            event: Event,
            bounds: Rectangle,
            cursor: Cursor,
        ) -> (event::Status, Option<(usize, f32)>) {
            let cursor_position = if let Some(position) = cursor.position_in(&bounds) {
                position
            } else {
                return (event::Status::Ignored, None);
            };

            match event {
                Event::Mouse(mouse_event) => {
                    let message = match mouse_event {
                        mouse::Event::CursorMoved { .. } => {
                            // let segment = clamp(Math.floor(x / segments_width), 0, segments_num - 1);
                            // let y2 = clamp(y, 10, segments_weight_max + 10);

                            let p = cursor_position;
                            // segments[segment] = y2;
                            let segment = (p.x / segments_width).floor().clamp(0.0, 31.0) as usize;
                            let y = p.y.clamp(10.0, 110.0);
                            let weight = 110.0 - y;
                            let r = Some((segment, weight));
                            println!("{:?}", r);

                            r
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
            state: &Self::State,
            _theme: &Theme,
            bounds: Rectangle,
            cursor: Cursor,
        ) -> Vec<Geometry> {
            let content = self.state.cache.draw(bounds.size(), |frame: &mut Frame| {
                // let gradient = ctx.createLinearGradient(0, 110, c.width, 120);
                // gradient.addColorStop(0, "maroon");
                // gradient.addColorStop(0.1, "red");
                // gradient.addColorStop(0.3, "yellow");
                // gradient.addColorStop(0.5, "green");
                // gradient.addColorStop(0.7, "aqua");
                // gradient.addColorStop(0.9, "blue");
                // gradient.addColorStop(1, "purple");
                let gradient = Gradient::linear(gradient::Position::Relative {
                    top_left: Point { x: 0.0, y: 0.0 },
                    size: Size {
                        width: segments_width * WEIGHTS_NUM as f32,
                        height: 120.0,
                    },
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

                for (i, w) in self.weights.iter().enumerate() {
                    frame.fill_rectangle(
                        Point {
                            x: i as f32 * segments_width,
                            y: 110.0 - *w,
                        },
                        Size::new(segments_width, *w),
                        gradient.clone(),
                    );

                    frame.fill_rectangle(
                        Point {
                            x: i as f32 * segments_width,
                            y: 110.0 - *w,
                        },
                        Size::new(segments_width, 5.0),
                        Fill::default(),
                    )
                }
                // Curve::draw_all(self.curves, frame);

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

mod bezier {

    use iced::mouse;
    use iced::widget::canvas::event::{self, Event};
    use iced::widget::canvas::{self, Canvas, Cursor, Frame, Geometry, Path, Stroke};
    use iced::{Element, Length, Point, Rectangle, Theme};

    #[derive(Default)]
    pub struct State {
        cache: canvas::Cache,
    }

    impl State {
        pub fn view<'a>(&'a self, curves: &'a [Curve]) -> Element<'a, Curve> {
            Canvas::new(Bezier {
                state: self,
                curves,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }

        pub fn request_redraw(&mut self) {
            self.cache.clear()
        }
    }

    struct Bezier<'a> {
        state: &'a State,
        curves: &'a [Curve],
    }

    impl<'a> canvas::Program<Curve> for Bezier<'a> {
        type State = Option<Pending>;

        fn update(
            &self,
            state: &mut Self::State,
            event: Event,
            bounds: Rectangle,
            cursor: Cursor,
        ) -> (event::Status, Option<Curve>) {
            let cursor_position = if let Some(position) = cursor.position_in(&bounds) {
                position
            } else {
                return (event::Status::Ignored, None);
            };

            match event {
                Event::Mouse(mouse_event) => {
                    let message = match mouse_event {
                        mouse::Event::ButtonPressed(mouse::Button::Left) => match *state {
                            None => {
                                *state = Some(Pending::One {
                                    from: cursor_position,
                                });

                                None
                            }
                            Some(Pending::One { from }) => {
                                *state = Some(Pending::Two {
                                    from,
                                    to: cursor_position,
                                });

                                None
                            }
                            Some(Pending::Two { from, to }) => {
                                *state = None;

                                Some(Curve {
                                    from,
                                    to,
                                    control: cursor_position,
                                })
                            }
                        },
                        _ => None,
                    };

                    (event::Status::Captured, message)
                }
                _ => (event::Status::Ignored, None),
            }
        }

        fn draw(
            &self,
            state: &Self::State,
            _theme: &Theme,
            bounds: Rectangle,
            cursor: Cursor,
        ) -> Vec<Geometry> {
            let content = self.state.cache.draw(bounds.size(), |frame: &mut Frame| {
                Curve::draw_all(self.curves, frame);

                frame.stroke(
                    &Path::rectangle(Point::ORIGIN, frame.size()),
                    Stroke::default().with_width(2.0),
                );
            });

            if let Some(pending) = state {
                let pending_curve = pending.draw(bounds, cursor);

                vec![content, pending_curve]
            } else {
                vec![content]
            }
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

    #[derive(Debug, Clone, Copy)]
    pub struct Curve {
        from: Point,
        to: Point,
        control: Point,
    }

    impl Curve {
        fn draw_all(curves: &[Curve], frame: &mut Frame) {
            let curves = Path::new(|p| {
                for curve in curves {
                    p.move_to(curve.from);
                    p.quadratic_curve_to(curve.control, curve.to);
                }
            });

            frame.stroke(&curves, Stroke::default().with_width(2.0));
        }
    }

    #[derive(Debug, Clone, Copy)]
    enum Pending {
        One { from: Point },
        Two { from: Point, to: Point },
    }

    impl Pending {
        fn draw(&self, bounds: Rectangle, cursor: Cursor) -> Geometry {
            let mut frame = Frame::new(bounds.size());

            if let Some(cursor_position) = cursor.position_in(&bounds) {
                match *self {
                    Pending::One { from } => {
                        let line = Path::line(from, cursor_position);
                        frame.stroke(&line, Stroke::default().with_width(2.0));
                    }
                    Pending::Two { from, to } => {
                        let curve = Curve {
                            from,
                            to,
                            control: cursor_position,
                        };

                        Curve::draw_all(&[curve], &mut frame);
                    }
                };
            }

            frame.into_geometry()
        }
    }
}
