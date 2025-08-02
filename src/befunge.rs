use ahash::HashMap;
use rand::Rng;

// (hopefully) fully befunge93 compliant

#[derive(Clone, Debug)]
pub struct Position<T> {
    pub x: T,
    pub y: T,
}

pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub enum NeedsInputType {
    None,
    Character,
    Decimal,
}

impl<T> Position<T> {
    pub fn new(x: T, y: T) -> Self {
        Position { x, y }
    }
}

pub struct FungedState {
    pub map: HashMap<(u16, u16), i64>,
    pub put_map: HashMap<(u16, u16), i64>,
    pub is_string_mode: bool,
    pub position: Position<u16>,
    pub direction: Direction,
    pub stack: Vec<i64>,
    pub output: String,
    pub input: String,
    pub is_running: bool,
}

impl Default for FungedState {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl FungedState {
    pub fn new() -> Self {
        Self {
            map: HashMap::default(),
            put_map: HashMap::default(),
            is_string_mode: false,
            position: Position::new(0, 0),
            direction: Direction::Right,
            stack: Vec::new(),
            output: String::new(),
            input: String::new(),
            is_running: false,
        }
    }

    pub fn map_from_string(&mut self, string: &str) {
        for (r, line) in string.lines().enumerate() {
            for (c, character) in line.chars().enumerate() {
                self.setc(c as u16, r as u16, character);
            }
        }
    }

    pub fn print(&mut self, width: u16, height: u16) {
        for y in 0..height {
            for x in 0..width {
                print!("{}", char::from_u32(self.get(x, y)
                .try_into()
                .unwrap_or(b' ' as u32)).unwrap()
                );
            }
            println!();
        }
    }

    pub fn get(&self, x: u16, y: u16) -> i64 {
        *self.put_map.get(&(x, y)).unwrap_or(
            self.map.get(&(x, y)).unwrap_or(&(b' ' as i64)))
    }

    pub fn set(&mut self, x: u16, y: u16, v: i64) {
        self.map.insert((x, y), v);
    }

    pub fn setc(&mut self, x: u16, y: u16, v: char) {
        self.map.insert((x, y), v as i64);
    }

    pub fn restart(&mut self) {
        self.position = Position::new(0, 0);
        self.direction = Direction::Right;
        self.is_string_mode = false;
        self.is_running = false;
        self.stack.clear();
        self.output.clear();
        self.input.clear();
        self.put_map.clear();
    }

    pub fn do_step(&mut self) -> NeedsInputType {
        if self.is_string_mode {
            let character: u32 = self
                .get(self.position.x, self.position.y)
                .try_into()
                .unwrap_or(u32::MAX);

            if character == b'"' as u32 {
                self.is_string_mode = false
            } else {
                self.stack.push(character as i64)
            }

        } else {
            let op: u8 = self
                .get(self.position.x, self.position.y)
                .try_into()
                .unwrap_or(b' ');

            match op {
                // space is no-op
                b' ' => (),

                // direction operations
                b'^' => self.direction = Direction::Up,
                b'v' => self.direction = Direction::Down,
                b'<' => self.direction = Direction::Left,
                b'>' => self.direction = Direction::Right,

                // arithmetic
                b'+' => {
                    let a = self.stack.pop().unwrap_or(0);
                    let b = self.stack.pop().unwrap_or(0);
                    self.stack.push(a + b);
                }
                b'-' => {
                    let a = self.stack.pop().unwrap_or(0);
                    let b = self.stack.pop().unwrap_or(0);
                    self.stack.push(b - a);
                }
                b'*' => {
                    let a = self.stack.pop().unwrap_or(0);
                    let b = self.stack.pop().unwrap_or(0);
                    self.stack.push(a * b);
                }
                b'/' => {
                    let a = self.stack.pop().unwrap_or(0);
                    let b = self.stack.pop().unwrap_or(0);
                    self.stack.push(b / a);
                }
                b'%' => {
                    let a = self.stack.pop().unwrap_or(0);
                    let b = self.stack.pop().unwrap_or(0);
                    self.stack.push(b % a);
                }

                // Logical operators
                // not
                b'!' => {
                    if self.stack.pop().unwrap_or(0) == 0 {
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }
                // greater than
                b'`' => {
                    let a = self.stack.pop().unwrap_or(0);
                    let b = self.stack.pop().unwrap_or(0);
                    if b > a {
                        self.stack.push(1);
                    } else {
                        self.stack.push(0);
                    }
                }

                // If statements
                // horizontal
                b'_' => {
                    if self.stack.pop().unwrap_or(0) == 0 {
                        self.direction = Direction::Right
                    } else {
                        self.direction = Direction::Left
                    }
                }
                // vertical
                b'|' => {
                    if self.stack.pop().unwrap_or(0) == 0 {
                        self.direction = Direction::Down
                    } else {
                        self.direction = Direction::Up
                    }
                }

                // Random
                b'?' => {
                    let mut rand = rand::rng();

                    self.direction = match rand.random_range(0..4) {
                        0 => Direction::Up,
                        1 => Direction::Down,
                        2 => Direction::Left,
                        3 => Direction::Right,
                        _ => panic!("this wont ever happen"),
                    }
                }

                // Stack manipulation
                // duplicate top
                b':' => {
                    let a = self.stack.pop().unwrap_or(0);
                    self.stack.push(a);
                    self.stack.push(a);
                }
                // swap two top
                b'\\' => {
                    let a = self.stack.pop().unwrap_or(0);
                    let b = self.stack.pop().unwrap_or(0);

                    self.stack.push(a);
                    self.stack.push(b);
                }
                // pop top
                b'$' => {
                    self.stack.pop();
                }

                // Bridge (skip next cell)
                b'#' => self.step_forward(),

                // Space manipulation
                // TODO: make temporary and add rollback function or similar
                // put (pop y,x,v, and put v at x,y)
                b'p' => {
                    let y = self.stack.pop().unwrap_or(0);
                    let x = self.stack.pop().unwrap_or(0);
                    let v = self.stack.pop().unwrap_or(0);

                    // dont worry, should never panic (aslong as clamp works)
                    self.put_map.insert((x.clamp(0, u16::MAX.into()).try_into().unwrap(), y.clamp(0, u16::MAX.into()).try_into().unwrap()), v);
                }
                // get (pop y,x and push value at x,y)
                b'g' => {
                    let y = self.stack.pop().unwrap_or(0);
                    let x = self.stack.pop().unwrap_or(0);

                    self.stack.push(self.get(x.clamp(0, u16::MAX.into()).try_into().unwrap(), y.clamp(0, u16::MAX.into()).try_into().unwrap()));
                }

                // Output
                // as integer (followed by space)
                b'.' => {
                    self.output
                        .push_str(&self.stack.pop().unwrap_or(0).to_string());
                    self.output.push(' ');
                }
                // as char
                b',' => {
                    let a: char = char::from_u32(
                        self.stack.pop().unwrap_or(0).try_into().unwrap_or(u32::MAX),
                    )
                    .unwrap_or('�');
                    self.output.push(a);
                }

                // Input
                // get decimal
                b'&' => {
                    if self.input.is_empty() {
                        return NeedsInputType::Decimal
                    }
                    self.stack.push(self.input.parse().unwrap_or(0));
                    self.input.clear();
                },
                // get character
                b'~' => {
                    if self.input.is_empty() {
                        return NeedsInputType::Character
                    }
                    self.stack.push(self.input.chars().next().unwrap_or(0 as char) as i64);
                    self.input.clear();
                },

                // String mode
                b'"' => self.is_string_mode = true,

                // End program
                b'@' => self.is_running = false,

                // Digits
                b'0'..=b'9' => self.stack.push((op - b'0') as i64),

                _ => (),
            }
        }

        self.step_forward();

        NeedsInputType::None
    }

    fn step_forward(&mut self) {
        match self.direction {
            Direction::Up => self.position.y = self.position.y.wrapping_sub(1),
            Direction::Down => self.position.y = self.position.y.wrapping_add(1),
            Direction::Left => self.position.x = self.position.x.wrapping_sub(1),
            Direction::Right => self.position.x = self.position.x.wrapping_add(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn do_n_steps(state: &mut FungedState, n: u16) {
        state.is_running = true;
        for _ in 0..n {
            if !state.is_running {
                return;
            }
            state.do_step();
        }
    }

    pub fn run_until_completion(state: &mut FungedState) {
        state.is_running = true;
        loop {
            if !state.is_running {
                return;
            }

            state.do_step();
        }
    }

    #[test]
    fn basic_push_and_move() {
        let mut state = FungedState::new();

        state.map_from_string(
            "v \n\
             >0123456789");

        do_n_steps(&mut state, 12);

        assert_eq!(state.stack, vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
        assert_eq!(state.position.x, 11);
        assert_eq!(state.position.y, 1);
    }

    #[test]
    fn if_statements() {
        let mut state = FungedState::new();

        // setup
        //  v
        //2 1 4
        //|0_0|
        //1   3
        state.map_from_string(
               "  v  \n\
                @   @\n\
                2 1 4\n\
                |0_0|\n\
                1   3\n\
                @   @");

        run_until_completion(&mut state);
        assert_eq!(state.stack, vec![1]);

        state.position = Position::new(0, 0);
        state.direction = Direction::Right;
        state.stack.clear();
        state.setc(1, 3, '1');
        run_until_completion(&mut state);
        assert_eq!(state.stack, vec![2]);

        state.position = Position::new(0, 0);
        state.direction = Direction::Right;
        state.stack.clear();
        state.setc(2, 2, '0');
        run_until_completion(&mut state);
        assert_eq!(state.stack, vec![3]);

        state.position = Position::new(0, 0);
        state.direction = Direction::Right;
        state.stack.clear();
        state.setc(3, 3, '1');
        run_until_completion(&mut state);
        assert_eq!(state.stack, vec![4]);
    }

    #[test]
    fn operators() {
        let mut state = FungedState::new();

        state.map_from_string("21`12`!0!!@");

        run_until_completion(&mut state);
        assert_eq!(state.stack, vec![1, 1, 0]);
    }

    #[test]
    fn arithmetic() {
        let mut state = FungedState::new();

        state.map_from_string("27*3+2-62/95%@");

        run_until_completion(&mut state);
        assert_eq!(state.stack, vec![15, 3, 4]);
    }

    #[test]
    fn bridge() {
        let mut state = FungedState::new();

        state.map_from_string("0#@1#2# @");

        run_until_completion(&mut state);
        assert_eq!(state.stack, vec![0, 1]);
    }

    #[test]
    fn stack_manipulation() {
        let mut state = FungedState::new();

        state.map_from_string(":1\\$:@");

        run_until_completion(&mut state);
        assert_eq!(state.stack, vec![0, 1, 1]);
    }

    #[test]
    fn print_string() {
        let mut state = FungedState::new();
 
        state.map_from_string("\"v,8g\\\",,,,,@");

        run_until_completion(&mut state);
        assert_eq!(state.stack, Vec::new());
        assert_eq!(state.output, String::from("\\g8,v"));
    }

    #[test]
    fn print_integer() {
        let mut state = FungedState::new();

        state.map_from_string("\" \"98....@");

        run_until_completion(&mut state);
        assert_eq!(state.stack, Vec::new());
        assert_eq!(state.output, String::from("8 9 32 0 "));
    }

    #[test]
    fn space_manipulation() {
        let mut state = FungedState::new();

        state.map_from_string("\"r\"97p97g96g@");

        run_until_completion(&mut state);
        assert_eq!(state.stack, vec![b'r' as i64, b' ' as i64]);
        assert_eq!(state.get(9, 7), b'r' as i64);
    }

    #[test]
    fn read_input() {
        let mut state = FungedState::new();

        state.map_from_string("~&@");

        state.input = String::from("aa");
        state.do_step();
        assert_eq!(state.input, String::new());
        state.input = String::from("571");
        run_until_completion(&mut state);


    }

    #[test]
    fn wrapping() {
        let mut state = FungedState::new();
        state.setc(0, 0, '^');
        state.setc(0, u16::MAX, '<');
        state.setc(u16::MAX, u16::MAX, 'v');
        state.setc(u16::MAX, 0, '>');

        state.do_step();
        assert_eq!(state.position.x, 0);
        assert_eq!(state.position.y, u16::MAX);
        state.do_step();
        assert_eq!(state.position.x, u16::MAX);
        assert_eq!(state.position.y, u16::MAX);
        state.do_step();
        assert_eq!(state.position.x, u16::MAX);
        assert_eq!(state.position.y, 0);
        state.do_step();
        assert_eq!(state.position.x, 0);
        assert_eq!(state.position.y, 0);
    }
}
