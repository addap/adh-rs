use equalizer::canvas_size;
use fundsp::math::lerp;
use iced::widget::column;
use iced::window::{self, Position};
use iced::{
    executor, theme, Alignment, Application, Command, Element, Point, Settings, Subscription,
};
use iced_runtime::core::event::Status;
use iced_runtime::core::keyboard::KeyCode;
use std::usize;
use xdg::{self, BaseDirectories};

use adh_rs::{
    protocol, protocol::Protocol, slots::Slots, Weights, SEGMENTS_WEIGHT_MAX, WEIGHTS_NUM,
};

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
        // a.d. TODO calculation does not work as expected, eyeballed subtracting another 50.
        screen_size.1 - SCREEN_PADDING - window_size.1 - 50,
    );

    TrayUtility::run(Settings {
        antialiasing: true,
        window: iced::window::Settings {
            // set resizable = false to make window floating on wayland (sway).
            resizable: false,
            size: window_size,
            max_size: Some(window_size),
            position: Position::Specific(window_position.0 as i32, window_position.1 as i32),
            platform_specific: iced::window::PlatformSpecific {
                // set x11 window types to make window floating on x11 (i3).
                x11_window_type: vec![
                    winit::platform::x11::XWindowType::Notification,
                    winit::platform::x11::XWindowType::Utility,
                ],
            },
            ..iced::window::Settings::default()
        },
        ..Settings::default()
    })
}

/// This application is meant to be used as a small floating window.
/// It renders something like an equalizer to give weights to different frequency bands.
/// Based on the weights, colored noise is generated.
#[derive(Debug)]
struct TrayUtility {
    equalizer: equalizer::State,
    weights: Weights,
    protocol: Protocol,
    slots: Slots,
    xdg: BaseDirectories,
    last_segment_weight: Option<(usize, f32)>,
}

impl TrayUtility {
    fn new() -> Self {
        let protocol = Protocol::new_send().unwrap();
        let xdg = BaseDirectories::with_prefix("adh-rs").unwrap();
        let slots = Slots::load_from_disk(&xdg);
        let start_weights = slots.recall_slot(0);

        Self {
            equalizer: Default::default(),
            weights: start_weights,
            protocol,
            slots,
            xdg,
            last_segment_weight: None,
        }
    }

    /// Cleanup and return command to close the window.
    fn window_close(&mut self) -> Command<Message> {
        println!("exiting");
        self.slots.write_to_disk(&self.xdg);
        window::close()
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    ProcessCursorPosition(Point),
    OutOfBounds,
    ConfirmWeights,
    Clear,
    ExitApplication,
    TogglePlay,
    ExitDaemon,
    SaveSlot(usize),
    RecallSlot(usize),
}

impl Application for TrayUtility {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = theme::Theme;

    fn new(_: ()) -> (Self, Command<Message>) {
        (TrayUtility::new(), Command::none())
    }

    fn title(&self) -> String {
        String::from("Equalizer")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::ProcessCursorPosition(_) => {}
            // Reset the last segment so that it does not try to interpolate skipped segment weights when the user briefly exits the canvas area while mouse is pressed.
            _ => self.last_segment_weight = None,
        };

        match message {
            Message::ProcessCursorPosition(cursor_position) => {
                // Calculate the segment we are in based on x position of cursor.
                let current_segment = util::xpos_to_segment(cursor_position.x);
                let current_weight = util::ypos_to_weight(cursor_position.y);

                // Moving the mouse quickly can lead to skipping over segments, i.e. no ProcessCursorPosition event is emitted for the segment.
                // For the user, this leads to unintended spikes in the frequency band weights.
                // So we calculate interpolated weights for the skipped segments.
                // The current segment will need to be set either way. We initialize the array with it so that even if self.last_segment_weight is None, the weight is set.
                let mut weight_changes = vec![(current_segment, current_weight)];

                if let Some((last_segment, last_weight)) = self.last_segment_weight {
                    let segment_diff = current_segment.abs_diff(last_segment);
                    // If the difference is 0 or 1, we are in the same segment as last time or an adjacent one, so no skipped segments.
                    if segment_diff >= 2 {
                        // Otherwise we interpolate between the last weight and current weight to set the weights for the segments we skipped
                        let ((start_segment, start_weight), (end_segment, end_weight)) =
                            if current_segment < last_segment {
                                (
                                    (current_segment, current_weight),
                                    (last_segment, last_weight),
                                )
                            } else {
                                (
                                    (last_segment, last_weight),
                                    (current_segment, current_weight),
                                )
                            };

                        // Due to the way the loop is structured, current_segment will be set twice. But it uses the same value so we do not filter it out.
                        for (i, segment) in (start_segment..=end_segment).enumerate() {
                            // segment_diff is never 0 here because we checked that segment_diff >= 2.
                            let t = i as f32 / segment_diff as f32;
                            let weight = lerp(start_weight, end_weight, t);

                            weight_changes.push((segment, weight));
                        }
                    }
                }

                for (segment, weight) in weight_changes {
                    self.weights.v.get_mut(segment).map(|w| *w = weight);
                }
                self.last_segment_weight = Some((current_segment, current_weight));
                self.equalizer.request_redraw();
            }
            Message::OutOfBounds => {
                // Don't need to do anything because self.last_segment_index is already reset.
            }
            Message::ConfirmWeights => {
                self.protocol
                    .send(&protocol::GUICommand::SetWeights(self.weights))
                    .unwrap();
            }
            Message::Clear => {
                self.weights = Weights::default();
                self.equalizer = equalizer::State::default();
            }
            Message::ExitApplication => {
                return self.window_close();
            }
            Message::ExitDaemon => {
                self.protocol.send(&protocol::GUICommand::Quit).unwrap();
                return self.window_close();
            }
            Message::TogglePlay => {
                self.protocol.send(&protocol::GUICommand::Toggle).unwrap();
            }
            Message::SaveSlot(idx) => self.slots.save_slot(idx, self.weights),
            Message::RecallSlot(idx) => {
                self.weights = self.slots.recall_slot(idx);
                self.equalizer.request_redraw();
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
        // subscribe to keyboard events
        // 'Q': quit GUI
        // 'D': exit daemon
        // 'P': pause playback
        // 'C': clear weights (go back to white noise)
        // '0'..'9': recall slot
        // Ctrl + '0'..'9': save slot
        iced_runtime::futures::subscription::events_with(|event, status| match (status, event) {
            /* TODO apparently this event is not emitted on mod+Shift+q in newer iced versions. Should debug. */
            (
                _,
                iced_runtime::core::Event::Window(
                    iced_runtime::core::window::Event::CloseRequested,
                ),
            ) => Some(Message::ExitApplication),
            (
                Status::Ignored,
                iced_runtime::core::Event::Keyboard(
                    iced_runtime::core::keyboard::Event::KeyPressed {
                        key_code: KeyCode::Q,
                        ..
                    },
                ),
            ) => Some(Message::ExitApplication),
            (
                Status::Ignored,
                iced_runtime::core::Event::Keyboard(
                    iced_runtime::core::keyboard::Event::KeyPressed {
                        key_code: KeyCode::D,
                        ..
                    },
                ),
            ) => Some(Message::ExitDaemon),
            (
                Status::Ignored,
                iced_runtime::core::Event::Keyboard(
                    iced_runtime::core::keyboard::Event::KeyPressed {
                        key_code: KeyCode::P,
                        ..
                    },
                ),
            ) => Some(Message::TogglePlay),
            (
                Status::Ignored,
                iced_runtime::core::Event::Keyboard(
                    iced_runtime::core::keyboard::Event::KeyPressed {
                        key_code: KeyCode::C,
                        ..
                    },
                ),
            ) => Some(Message::Clear),
            // this one should be last since it captures all keys to filter out the num keys
            (
                Status::Ignored,
                iced_runtime::core::Event::Keyboard(
                    iced_runtime::core::keyboard::Event::KeyPressed {
                        key_code,
                        modifiers,
                    },
                ),
            ) => {
                fn key_code_to_num(key_code: KeyCode) -> Option<usize> {
                    match key_code {
                        KeyCode::Key1 => Some(1),
                        KeyCode::Key2 => Some(2),
                        KeyCode::Key3 => Some(3),
                        KeyCode::Key4 => Some(4),
                        KeyCode::Key5 => Some(5),
                        KeyCode::Key6 => Some(6),
                        KeyCode::Key7 => Some(7),
                        KeyCode::Key8 => Some(8),
                        KeyCode::Key9 => Some(9),
                        KeyCode::Key0 => Some(0),
                        _ => None,
                    }
                }

                match (key_code_to_num(key_code), modifiers.control()) {
                    (Some(idx), true) => Some(Message::SaveSlot(idx)),
                    (Some(idx), false) => Some(Message::RecallSlot(idx)),
                    _ => None,
                }
            }
            (_, _) => None,
        })
        // ])
    }
}

/// Some utility functions for converting coordinates
mod util {
    use super::{
        CANVAS_HEIGHT, SEGMENTS_WEIGHT_MAX, SEGMENTS_WIDTH, WEIGHTS_NUM, WEIGHTS_PADDING_Y,
    };

    pub fn weight_to_ypos(weight: f32) -> f32 {
        CANVAS_HEIGHT + WEIGHTS_PADDING_Y - (weight * CANVAS_HEIGHT)
    }

    pub fn ypos_to_weight(y: f32) -> f32 {
        ((CANVAS_HEIGHT + WEIGHTS_PADDING_Y - y) / CANVAS_HEIGHT).clamp(0.0, SEGMENTS_WEIGHT_MAX)
    }

    pub fn xpos_to_segment(x: f32) -> usize {
        ((x / SEGMENTS_WIDTH).floor() as usize).clamp(0, WEIGHTS_NUM - 1)
    }
}

mod equalizer {
    use iced::widget::canvas::event::{self, Event};
    use iced::widget::canvas::{self, gradient, Canvas, Fill, Frame, Geometry, Path, Stroke};
    use iced::{
        mouse::{self, Cursor},
        Color, Size,
    };
    use iced::{Element, Length, Point, Rectangle, Theme};

    use super::util::weight_to_ypos;

    use super::{Message, Weights, CANVAS_HEIGHT, SEGMENTS_WIDTH, WEIGHTS_NUM, WEIGHTS_PADDING_Y};

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
                        // a.d. iirc when the cursor left we want something else to be able to handle the event so we return Ignored.
                        return (event::Status::Ignored, Some(Message::ConfirmWeights));
                    }
                    _ => {}
                },
                _ => {}
            };

            let cursor_position = if let Some(position) = cursor.position_in(bounds) {
                position
            } else {
                return (event::Status::Ignored, Some(Message::OutOfBounds));
            };

            match event {
                Event::Mouse(mouse_event) => {
                    let message = match mouse_event {
                        mouse::Event::CursorMoved { .. }
                        | mouse::Event::ButtonPressed(mouse::Button::Left)
                            if state.is_active() =>
                        {
                            Some(Message::ProcessCursorPosition(cursor_position))
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
            renderer: &iced::Renderer,
            _theme: &Theme,
            bounds: Rectangle,
            _cursor: Cursor,
        ) -> Vec<Geometry> {
            let content = self
                .state
                .cache
                .draw(renderer, bounds.size(), |frame: &mut Frame| {
                    let (width, height) = canvas_size();

                    let gradient =
                        gradient::Linear::new(Point { x: 0.0, y: 0.0 }, Point { x: width, y: 0.0 })
                            .add_stop(0.0, Color::from_rgb8(0x80, 0, 0))
                            .add_stop(0.1, Color::from_rgb8(0xFF, 0, 0))
                            .add_stop(0.3, Color::from_rgb8(0xFF, 0xFF, 0))
                            .add_stop(0.5, Color::from_rgb8(0x00, 0x80, 0))
                            .add_stop(0.7, Color::from_rgb8(0x00, 0x80, 0))
                            .add_stop(0.9, Color::from_rgb8(0x00, 0xFF, 0xFF))
                            .add_stop(1.0, Color::from_rgb8(0x80, 0, 0x80));

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
            if cursor.is_over(bounds) {
                mouse::Interaction::Crosshair
            } else {
                mouse::Interaction::default()
            }
        }
    }
}
