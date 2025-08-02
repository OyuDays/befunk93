use crossterm::{
    ExecutableCommand,
    event::{
        self, Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    terminal,
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{self, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, Padding, Paragraph, Wrap},
};
use std::{
    fs::File,
    io::{Read, Stdout, Write, stdout},
    thread,
    time::Duration,
};
mod befunge;
use befunge::*;

fn panic_hook(info: &std::panic::PanicHookInfo<'_>) {
    let backtrace = std::backtrace::Backtrace::capture();

    setdown();

    eprintln!("backtrace:\n{}", backtrace);
    eprintln!("{}", info);
}

fn draw_space(
    frame: &mut Frame,
    state: &FungedState,
    area: Rect,
    offset: Position<u16>,
    cursorpos: Position<u16>,
) {
    let mut text = Text::default();
    for y in offset.y..offset.y + area.height {
        let mut line = Line::default();
        for x in offset.x..offset.x + area.width {
            let mut span = Span::default();

            let char = char::from_u32(state.get(x, y).try_into().unwrap_or(0)).unwrap_or('�');
            span = span.content(char.to_string());
            if x == state.position.x && y == state.position.y {
                span = span.style(Style::default().fg(Color::Black).bg(Color::Blue));
            } else {
                span = span.style(Style::default().fg(Color::White));
                if x == cursorpos.x && y == cursorpos.y {
                    span = span.patch_style(Style::default().add_modifier(Modifier::REVERSED));
                }
            };

            if char.is_control() || char.is_whitespace() {
                if char == ' ' {
                    span = span.content(" ")
                } else {
                    span = span
                        .content("X")
                        .patch_style(Style::default().fg(Color::Red))
                }
            }

            line.push_span(span);
        }
        text.push_line(line)
    }
    frame.render_widget(text, area);
}

fn draw_commandbar(frame: &mut Frame, area: Rect, command_prompt: &str, command: &str) {
    let block = Block::new()
        .borders(Borders::NONE)
        .padding(Padding::ZERO)
        .bg(Color::Black);

    let paragraph = Paragraph::new(command)
        .block(block.title(command_prompt))
        .style(Style::new().white())
        .wrap(Wrap { trim: true });

    frame.render_widget(paragraph, area);
}

fn draw_sidebar(frame: &mut Frame, state: &FungedState, area: Rect) {
    let block = Block::new()
        .borders(Borders::NONE)
        .padding(Padding::ZERO)
        .bg(Color::Black);

    let list = List::new(state.stack.clone().into_iter().map(|i| i.to_string()))
        .block(block.clone().title("stack:"))
        .style(Style::new().white());

    let output = Paragraph::new(state.output.clone())
        .block(block.clone().title("output:"))
        .style(Style::new().white())
        .wrap(Wrap { trim: false });

    let commands_vec = vec![
        // Step
        Line::from(vec![
            Span::styled("^S", Style::new().blue()),
            Span::raw("tep"),
        ]),
        // Playpause
        Line::from(vec![
            Span::styled("^P", Style::new().blue()),
            Span::raw("laypause"),
        ]),
        // Reset
        Line::from(vec![
            Span::styled("^R", Style::new().blue()),
            Span::raw("eset"),
        ]),
        // Write
        Line::from(vec![
            Span::styled("^W", Style::new().blue()),
            Span::raw("rite"),
        ]),
        // Open
        Line::from(vec![
            Span::styled("^O", Style::new().blue()),
            Span::raw("pen"),
        ]),
        // Close
        Line::from(vec![
            Span::styled("^C", Style::new().blue()),
            Span::raw("lose"),
        ]),
    ];

    let commands = List::new(commands_vec.clone())
        .block(block.title("commands:"))
        .style(Style::new().white());

    let inner_layout = Layout::default()
        .direction(layout::Direction::Vertical)
        .constraints([
            Constraint::Min(10),
            Constraint::Length(10),
            Constraint::Length((commands_vec.len() + 1).try_into().unwrap()),
        ])
        .split(Rect::new(0, 0, area.width, area.height));

    frame.render_widget(list, inner_layout[0]);
    frame.render_widget(output, inner_layout[1]);
    frame.render_widget(commands, inner_layout[2]);
}

struct App {
    pub cursorpos: Position<u16>,
    pub posdirection: Direction,
    pub state: FungedState,
    pub camera_offset: Position<u16>,
    pub space_area: Rect,

    pub autoplay: bool,

    pub input_mode: InputMode,
    pub command_type: CommandType,

    pub terminal: Terminal<CrosstermBackend<Stdout>>,

    pub command_prompt: String,
    pub command: String,

    pub should_stop: bool,
}

impl App {
    pub fn new() -> Self {
        terminal::enable_raw_mode().expect("failed to enable raw mode");
        std::panic::set_hook(Box::new(panic_hook));

        stdout()
            .execute(terminal::EnterAlternateScreen)
            .expect("failed to enter alternate screen")
            .execute(event::EnableMouseCapture)
            .expect("failed to enable mouse capture");

        App {
            cursorpos: Position::new(0, 0),
            posdirection: Direction::Right,
            state: FungedState::new(),
            camera_offset: Position::new(0, 0),
            space_area: Rect::default(),

            autoplay: false,

            input_mode: InputMode::Normal,
            command_type: CommandType::Command,

            terminal: Terminal::new(CrosstermBackend::new(stdout()))
                .expect("failed to get ratatui terminal"),

            command_prompt: String::new(),
            command: String::new(),

            should_stop: false,
        }
    }

    fn get_file(&mut self, filename: &str) -> std::io::Result<String> {
        let mut file = File::open(filename)?;
        let mut string = String::new();
        file.read_to_string(&mut string)?;

        Ok(string)
    }

    fn write_file(&mut self, filename: &str, contents: String) -> std::io::Result<()> {
        let mut file = File::create(filename)?;
        file.write_all(contents.as_bytes())?;

        Ok(())
    }

    fn draw(&mut self) {
        self.terminal
            .draw(|frame| {
                let size = frame.area();

                let layout = Layout::default()
                    .direction(layout::Direction::Horizontal)
                    .constraints([Constraint::Length(12), Constraint::Min(20)])
                    .split(Rect::new(0, 0, size.width, size.height));
                let right_layout = Layout::default()
                    .direction(layout::Direction::Vertical)
                    .constraints([Constraint::Min(10), Constraint::Length(2)])
                    .split(layout[1]);

                frame.area();
                self.space_area = right_layout[0];

                self.camera_offset.x = self
                    .cursorpos
                    .x
                    .saturating_sub(self.space_area.width / 2)
                    .clamp(0, u16::MAX - self.space_area.width);
                self.camera_offset.y = self
                    .cursorpos
                    .y
                    .saturating_sub(self.space_area.height / 2)
                    .clamp(0, u16::MAX - self.space_area.height);

                draw_space(
                    frame,
                    &self.state,
                    right_layout[0],
                    self.camera_offset.clone(),
                    self.cursorpos.clone(),
                );
                draw_commandbar(frame, right_layout[1], &self.command_prompt, &self.command);
                draw_sidebar(frame, &self.state, layout[0]);

                //                frame.set_cursor_position(layout::Position::new(
                //                        (self.cursorpos
                //                        .x - self.camera_offset.x)
                //                        .wrapping_add(self.space_area.x)
                //                        .clamp(self.space_area.x, self.space_area.x + self.space_area.width),
                //                        (self.cursorpos.
                //                         y - self.camera_offset.y)
                //                         .wrapping_add(self.space_area.y)
                //                         .clamp(
                //                        self.space_area.y,
                //                        self.space_area.y + self.space_area.height,
                //                    ),
                //                ));
            })
            .expect("failed to draw frame");
    }

    fn handle_command_inputmode(&mut self, key: KeyEvent) {
        if let KeyModifiers::NONE | KeyModifiers::SHIFT = key.modifiers {
            match key.code {
                KeyCode::Char(char) => self.command.push(char),
                KeyCode::Esc => {
                    self.command = String::new();
                    self.command_prompt = String::new();
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Enter => {
                    match self.command_type {
                        CommandType::BefungeInput => {
                            self.state.input = self.command.clone();
                            self.command = String::new();
                        }

                        CommandType::OpenFile => match self.get_file(&self.command.clone()) {
                            Err(err) => self.command = err.to_string(),
                            Ok(string) => {
                                self.state = FungedState::new();
                                self.state.map_from_string(&string);
                            }
                        },
                        CommandType::WriteFile => {
                            let contents = self.state.map_to_string();
                            if let Err(err) = self.write_file(&self.command.clone(), contents) {
                                self.command = err.to_string();
                            }
                        }

                        CommandType::Command => todo!(),
                    }
                    self.command_prompt = String::new();
                    self.input_mode = InputMode::Normal;
                }
                KeyCode::Backspace => {
                    self.command.pop();
                }
                _ => (),
            }
        }
    }

    fn handle_control_keys(&mut self, key: char) {
        match key {
            'c' => self.should_stop = true,
            's' => self.do_step(),
            'r' => self.state.restart(),
            'p' => self.autoplay = !self.autoplay,
            'o' => {
                self.command_prompt = String::from("Open file");
                self.command.clear();
                self.input_mode = InputMode::Command;
                self.command_type = CommandType::OpenFile;
            }
            'w' => {
                self.command_prompt = String::from("Write file");
                self.command.clear();
                self.input_mode = InputMode::Command;
                self.command_type = CommandType::WriteFile;
            }

            _ => (),
        }
    }

    fn handle_normal_inputmode(&mut self, key: KeyEvent) {
        match key.modifiers {
            KeyModifiers::CONTROL => {
                if let KeyCode::Char(key) = key.code {
                    self.handle_control_keys(key)
                }
            }

            KeyModifiers::NONE | KeyModifiers::SHIFT => match key.code {
                // opposite direction
                KeyCode::Backspace => match self.posdirection {
                    Direction::Up => self.cursorpos.y = self.cursorpos.y.wrapping_add(1),
                    Direction::Down => self.cursorpos.y = self.cursorpos.y.wrapping_sub(1),
                    Direction::Left => self.cursorpos.x = self.cursorpos.x.wrapping_add(1),
                    Direction::Right => self.cursorpos.x = self.cursorpos.x.wrapping_sub(1),
                },

                KeyCode::Char(char) => {
                    self.state.setc(self.cursorpos.x, self.cursorpos.y, char);
                    // switch direction on direction items
                    match char {
                        '^' => self.posdirection = Direction::Up,
                        'v' => self.posdirection = Direction::Down,
                        '<' => self.posdirection = Direction::Left,
                        '>' => self.posdirection = Direction::Right,
                        _ => (),
                    }

                    match self.posdirection {
                        Direction::Up => self.cursorpos.y = self.cursorpos.y.wrapping_sub(1),
                        Direction::Down => self.cursorpos.y = self.cursorpos.y.wrapping_add(1),
                        Direction::Left => self.cursorpos.x = self.cursorpos.x.wrapping_sub(1),
                        Direction::Right => self.cursorpos.x = self.cursorpos.x.wrapping_add(1),
                    }
                }

                KeyCode::Up => {
                    self.cursorpos.y = self.cursorpos.y.wrapping_sub(1);
                    self.posdirection = Direction::Up;
                }
                KeyCode::Down => {
                    self.cursorpos.y = self.cursorpos.y.wrapping_add(1);
                    self.posdirection = Direction::Down;
                }
                KeyCode::Left => {
                    self.cursorpos.x = self.cursorpos.x.wrapping_sub(1);
                    self.posdirection = Direction::Left;
                }
                KeyCode::Right => {
                    self.cursorpos.x = self.cursorpos.x.wrapping_add(1);
                    self.posdirection = Direction::Right;
                }

                _ => (),
            },
            _ => (),
        }
    }

    fn handle_mouse_event(&mut self, event: MouseEvent) {
        if let MouseEventKind::Down(MouseButton::Left) = event.kind {
            if event.column < self.space_area.x + self.space_area.width
                && event.column >= self.space_area.x
                && event.row < self.space_area.y + self.space_area.height
                && event.row >= self.space_area.y
            {
                self.cursorpos.x = event.column - self.space_area.x + self.camera_offset.x;
                self.cursorpos.y = event.row - self.space_area.y + self.camera_offset.y;
            }
        }
    }

    fn handle_events(&mut self) {
        while event::poll(Duration::ZERO).unwrap() {
            match event::read().expect("failed to read events") {
                Event::Key(key) => match self.input_mode {
                    InputMode::Command => self.handle_command_inputmode(key),

                    InputMode::Normal => self.handle_normal_inputmode(key),
                },
                Event::Mouse(event) => self.handle_mouse_event(event),
                _ => (),
            }
        }
    }

    pub fn do_step(&mut self) {
        match self.state.do_step() {
            NeedsInputType::None => (),
            NeedsInputType::Decimal => {
                self.command_prompt = String::from("Enter Decimal");
                self.command.clear();
                self.input_mode = InputMode::Command;
                self.command_type = CommandType::BefungeInput;
            }
            NeedsInputType::Character => {
                self.command_prompt = String::from("Enter Character");
                self.command.clear();
                self.input_mode = InputMode::Command;
                self.command_type = CommandType::BefungeInput;
            }
        }
    }

    pub fn do_loop(&mut self) {
        while !self.should_stop {
            self.draw();
            self.handle_events();

            if self.autoplay {
                self.do_step();
            }
            // we don't need more than 30 fps
            thread::sleep(Duration::from_millis((1.0 / 30.0 * 1000.0) as u64));
        }
    }
}

pub fn setdown() {
    ratatui::restore();

    // restore sucks at its job so i gotta do it myself
    stdout()
        .execute(terminal::LeaveAlternateScreen)
        .expect("failed to leave alternate screen")
        .execute(event::DisableMouseCapture)
        .expect("failed to disable mouse capture");
    terminal::disable_raw_mode().expect("failed to disable raw mode");
}

fn main() {
    let mut app = App::new();

    app.do_loop();

    setdown();
}

enum InputMode {
    Normal,
    Command,
}

enum CommandType {
    Command,
    BefungeInput,
    OpenFile,
    WriteFile,
}
