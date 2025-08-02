use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyModifiers, MouseButton, MouseEventKind},
    terminal,
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{self, Constraint, Layout, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, Padding, Paragraph, Wrap},
};
use std::{io::stdout, thread, time::Duration};
mod befunge;
use befunge::*;

fn panic_hook(info: &std::panic::PanicHookInfo<'_>) {
    let backtrace = std::backtrace::Backtrace::capture();
    ratatui::restore();

    // double check
    stdout()
        .execute(terminal::LeaveAlternateScreen)
        .expect("failed to leave alternate screen")
        .execute(event::DisableMouseCapture)
        .expect("failed to disable mouse capture");

    terminal::disable_raw_mode().expect("failed to disable raw mode");

    eprintln!("backtrace:\n{}", backtrace);
    eprintln!("{}", info);
}

fn draw_space(frame: &mut Frame, state: &FungedState, area: Rect, offset: Position<u16>) {
    let mut text = Text::default();
    for y in offset.y..offset.y + area.height {
        let mut line = Line::default();
        for x in offset.x..offset.x + area.width {
            let mut span = Span::default();

            let char = char::from_u32(state.get(x, y).try_into().unwrap_or(0)).unwrap_or('�');
            span = span.content(char.to_string());
            span = if x == state.position.x && y == state.position.y {
                span.style(Style::default().fg(Color::Black).bg(Color::Blue))
            } else {
                span.style(Style::default().fg(Color::White))
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
        // Restart
        Line::from(vec![
            Span::styled("^R", Style::new().blue()),
            Span::raw("estart"),
        ]),
        // Cancel
        Line::from(vec![
            Span::styled("^C", Style::new().blue()),
            Span::raw("ancel"),
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

// TODO: this code is a fucking mess
// split it up into a struct
fn main() {
    let mut cursorpos: Position<u16> = Position::new(0, 0);
    let mut posdirection: Direction = Direction::Right;
    let mut state: FungedState = FungedState::new();
    let camera_offset: Position<u16> = Position::new(0, 0);

    let mut input_mode = InputMode::Normal;
    let mut command_type = CommandType::Command;

    let mut terminal =
        Terminal::new(CrosstermBackend::new(stdout())).expect("failed to get ratatui terminal");

    let mut command_prompt = "";
    let mut command = String::new();

    terminal::enable_raw_mode().expect("failed to enable raw mode");
    std::panic::set_hook(Box::new(panic_hook));

    stdout()
        .execute(terminal::EnterAlternateScreen)
        .expect("failed to enter alternate screen")
        .execute(event::EnableMouseCapture)
        .expect("failed to enable mouse capture");

    'top: loop {
        //terminal.clear().expect("failed to clear screen");
        let mut space_area: Rect = Default::default();
        terminal
            .draw(|frame| {
                let size = frame.area();
                let layout = Layout::default()
                    .direction(layout::Direction::Horizontal)
                    .constraints([Constraint::Length(10), Constraint::Min(20)])
                    .split(Rect::new(0, 0, size.width, size.height));
                let right_layout = Layout::default()
                    .direction(layout::Direction::Vertical)
                    .constraints([Constraint::Min(10), Constraint::Length(2)])
                    .split(layout[1]);

                frame.area();
                space_area = right_layout[0];
                draw_space(frame, &state, right_layout[0], camera_offset.clone());
                draw_commandbar(frame, right_layout[1], command_prompt, &command);
                draw_sidebar(frame, &state, layout[0]);

                frame.set_cursor_position(layout::Position::new(
                    cursorpos
                        .x
                        .wrapping_add(space_area.x)
                        .clamp(space_area.x, space_area.x + space_area.width),
                    cursorpos
                        .y
                        .wrapping_add(space_area.y)
                        .clamp(space_area.y, space_area.y + space_area.height),
                ));
            })
            .expect("failed to draw frame");

        while event::poll(Duration::ZERO).unwrap() {
            match event::read().expect("failed to read events") {
                Event::Key(key) => match input_mode {
                    InputMode::Command => {
                        if let KeyModifiers::NONE | KeyModifiers::SHIFT = key.modifiers {
                            match key.code {
                                KeyCode::Char(char) => command.push(char),
                                KeyCode::Esc => {
                                    command.clear();
                                    command_prompt = "";
                                    input_mode = InputMode::Normal;
                                }
                                KeyCode::Enter => {
                                    match command_type {
                                        CommandType::BefungeInput => state.input = command,
                                        CommandType::Command => todo!(),
                                    }
                                    command = String::new();
                                    command_prompt = "";
                                    input_mode = InputMode::Normal;
                                }
                                KeyCode::Backspace => {
                                    command.pop();
                                }
                                _ => (),
                            }
                        }
                    }
                    InputMode::Normal => match key.modifiers {
                        KeyModifiers::CONTROL => {
                            if let KeyCode::Char(key) = key.code {
                                match key {
                                    'c' => break 'top,
                                    's' => {
                                        // TODO: handle input

                                        match state.do_step() {
                                            NeedsInputType::None => (),
                                            NeedsInputType::Decimal => {
                                                command_prompt = "Enter Decimal";
                                                input_mode = InputMode::Command;
                                                command_type = CommandType::BefungeInput;
                                            }
                                            NeedsInputType::Character => {
                                                command_prompt = "Enter Character";
                                                input_mode = InputMode::Command;
                                                command_type = CommandType::BefungeInput;
                                            }
                                        }
                                    }
                                    'r' => state.restart(),

                                    _ => (),
                                }
                            }
                        }
                        KeyModifiers::NONE | KeyModifiers::SHIFT => match key.code {
                            // opposite direction
                            KeyCode::Backspace => match posdirection {
                                Direction::Up => cursorpos.y = cursorpos.y.wrapping_add(1),
                                Direction::Down => cursorpos.y = cursorpos.y.wrapping_sub(1),
                                Direction::Left => cursorpos.x = cursorpos.x.wrapping_add(1),
                                Direction::Right => cursorpos.x = cursorpos.x.wrapping_sub(1),
                            },

                            KeyCode::Char(char) => {
                                state.setc(cursorpos.x, cursorpos.y, char);
                                // switch direction on direction items
                                match char {
                                    '^' => posdirection = Direction::Up,
                                    'v' => posdirection = Direction::Down,
                                    '<' => posdirection = Direction::Left,
                                    '>' => posdirection = Direction::Right,
                                    _ => (),
                                }

                                match posdirection {
                                    Direction::Up => cursorpos.y = cursorpos.y.wrapping_sub(1),
                                    Direction::Down => cursorpos.y = cursorpos.y.wrapping_add(1),
                                    Direction::Left => cursorpos.x = cursorpos.x.wrapping_sub(1),
                                    Direction::Right => cursorpos.x = cursorpos.x.wrapping_add(1),
                                }
                            }

                            KeyCode::Up => {
                                cursorpos.y = cursorpos.y.wrapping_sub(1);
                                posdirection = Direction::Up;
                            }
                            KeyCode::Down => {
                                cursorpos.y = cursorpos.y.wrapping_add(1);
                                posdirection = Direction::Down;
                            }
                            KeyCode::Left => {
                                cursorpos.x = cursorpos.x.wrapping_sub(1);
                                posdirection = Direction::Left;
                            }
                            KeyCode::Right => {
                                cursorpos.x = cursorpos.x.wrapping_add(1);
                                posdirection = Direction::Right;
                            }

                            _ => (),
                        },
                        _ => (),
                    },
                },
                Event::Mouse(event) => {
                    if let MouseEventKind::Down(MouseButton::Left) = event.kind {
                        if event.column < space_area.x + space_area.width
                            && event.column >= space_area.x
                            && event.row < space_area.y + space_area.height
                            && event.row >= space_area.y
                        {
                            cursorpos.x = event.column - space_area.x + camera_offset.x;
                            cursorpos.y = event.row - space_area.y + camera_offset.y;
                        }
                    }
                }
                _ => (),
            }
        }

        // we don't need more than 30 fps
        thread::sleep(Duration::from_millis((1.0 / 30.0 * 1000.0) as u64));
    }
    ratatui::restore();

    // double check
    stdout()
        .execute(terminal::LeaveAlternateScreen)
        .expect("failed to leave alternate screen")
        .execute(event::DisableMouseCapture)
        .expect("failed to disable mouse capture");
    terminal::disable_raw_mode().expect("failed to disable raw mode");
}

enum InputMode {
    Normal,
    Command,
}

enum CommandType {
    Command,
    BefungeInput,
}
