use crossterm::{
    ExecutableCommand,
    event::{self, Event, KeyCode, KeyModifiers},
    terminal::{self},
};
use ratatui::{
    backend::CrosstermBackend, layout::{self, Constraint, Layout, Rect}, style::{Color, Style, Stylize}, text::{self, Line, Span, Text}, widgets::{Block, Borders, List, Padding, Paragraph, Wrap}, Frame, Terminal
};
use std::{
    io::{Stdout, Write, stdout},
    thread,
    time::Duration,
};
mod befunge;
use befunge::*;

fn draw_space(frame: &mut Frame, state: &FungedState, area: Rect, offset: Position<u16>) {
    let mut text = Text::default();
    for y in offset.y..offset.y + area.height {
        let mut line = Line::default();
        for x in offset.x..offset.x + area.width {
            let mut span = Span::default();

            let char = char::from_u32(state.get(x.into(), y.into()).try_into().unwrap_or(0))
                .unwrap_or('�');
            span = if char.is_control() || char.is_whitespace() {
                span.content(" ")
            } else {
                span.content(char.to_string())
            };
            span = if x as u64 == state.position.x && y as u64 == state.position.y {
                span.style(Style::default().fg(Color::Black).bg(Color::Blue))
            } else {
                span.style(Style::default().fg(Color::White))
            };
            line.push_span(span);
        }
        text.push_line(line)
    }
    frame.render_widget(text, area);
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
        .wrap(Wrap {trim: false });

    let commands = Paragraph::new(Text::from(vec![
            Line::from(vec![Span::styled("^S", Style::new().blue()), Span::raw("tep")]), // Step
            Line::from(vec![Span::styled("^C", Style::new().blue()), Span::raw("ancel")]), // Cancel
            Line::from(vec![Span::styled("^R", Style::new().blue()), Span::raw("estart")]), // Restart
        ]))
        .block(block.title("commands:"))
        .style(Style::new().white())
        .wrap(Wrap {trim: false });

    let inner_layout = Layout::default()
        .direction(layout::Direction::Vertical)
        .constraints([Constraint::Min(10), Constraint::Length(10), Constraint::Length(4)])
        .split(Rect::new(0, 0, area.width, area.height));

    frame.render_widget(list, inner_layout[0]);
    frame.render_widget(output, inner_layout[1]);
    frame.render_widget(commands, inner_layout[2]);
}

fn main() {
    let mut cursorpos: Position<u64> = Position::new(0, 0);
    let mut posdirection: Direction = Direction::Right;
    let mut state: FungedState = FungedState::new();
    let mut terminal =
        Terminal::new(CrosstermBackend::new(stdout())).expect("failed to get ratatui terminal");

    terminal::enable_raw_mode().expect("failed to enable raw mode");
    stdout()
        .execute(terminal::EnterAlternateScreen)
        .expect("failed to enter alternate screen");

    'top: loop {
        //terminal.clear().expect("failed to clear screen");
        terminal
            .draw(|frame| {
                let size = frame.area();
                let layout = Layout::default()
                    .direction(layout::Direction::Horizontal)
                    .constraints([Constraint::Length(10), Constraint::Min(20)])
                    .split(Rect::new(0, 0, size.width, size.height));
                frame.area();
                draw_space(frame, &state, layout[1], Position::new(0, 0));
                draw_sidebar(frame, &state, layout[0]);

                frame.set_cursor_position(layout::Position::new(
                    cursorpos.x.wrapping_add(layout[1].x as u64)
                        .clamp(layout[1].x.into(), layout[1].width.into())
                        .try_into()
                        .unwrap(), // still has some weird behaviour on right edge but thats a problem for
                                   // future me :)
                    cursorpos.y.wrapping_add(layout[1].y as u64)
                        .clamp(layout[1].y.into(), layout[1].height.into())
                        .try_into()
                        .unwrap(),
                ));
            })
            .expect("failed to draw frame");

        while event::poll(Duration::ZERO).unwrap() {
            if let Event::Key(key) = event::read().expect("failed to read events") {
                match key.modifiers {
                    KeyModifiers::CONTROL => {
                        if let KeyCode::Char(key) = key.code {
                            match key {
                                'c' => break 'top,
                                's' => {
                                    // TODO: handle input
                                    let _ = state.do_step();
                                },
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
                }
            }
        }

        // we don't need more than 30 fps
        thread::sleep(Duration::from_millis((1.0 / 30.0 * 1000.0) as u64));
    }
    ratatui::restore();

    // double check
    stdout()
        .execute(terminal::LeaveAlternateScreen)
        .expect("failed to leave alternate screen");
    terminal::disable_raw_mode().expect("failed to disable raw mode");
}
