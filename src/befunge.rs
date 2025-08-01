use ahash::HashMap;
use rand::Rng;

// (hopefully) fully befunge93 compliant

pub struct Position<T> {
    x: T,
    y: T,
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
    pub map: HashMap<(u64, u64), u64>,
    pub is_string_mode: bool,
    pub position: Position<u64>,
    pub direction: Direction,
    pub stack: Vec<u64>,
    pub output: String,
    pub input: String,
    pub is_running: bool,
}

impl Default for FungedState {
    fn default() -> Self {
        Self::new()
    }
}

impl FungedState {
    pub fn new() -> Self {
        Self {
            map: HashMap::default(),
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
                self.setc(c as u64, r as u64, character);
            }
        }
    }

    pub fn print(&mut self, width: u64, height: u64) {
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

    pub fn get(&self, x: u64, y: u64) -> u64 {
        *self.map.get(&(x, y)).unwrap_or(&(b' ' as u64))
    }

    pub fn set(&mut self, x: u64, y: u64, v: u64) {
        self.map.insert((x, y), v);
    }

    pub fn setc(&mut self, x: u64, y: u64, v: char) {
        self.map.insert((x, y), v as u64);
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
                self.stack.push(character as u64)
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
                b'#' => match self.direction {
                    Direction::Up => self.position.y -= 1,
                    Direction::Down => self.position.y += 1,
                    Direction::Left => self.position.x -= 1,
                    Direction::Right => self.position.x += 1,
                },

                // Space manipulation
                // TODO: make temporary and add rollback function or similar
                // put (pop y,x,v, and put v at x,y)
                b'p' => {
                    let y = self.stack.pop().unwrap_or(0);
                    let x = self.stack.pop().unwrap_or(0);
                    let v = self.stack.pop().unwrap_or(0);

                    self.set(x, y, v);
                }
                // get (pop y,x and push value at x,y)
                b'g' => {
                    let y = self.stack.pop().unwrap_or(0);
                    let x = self.stack.pop().unwrap_or(0);

                    self.stack.push(self.get(x, y));
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
                    self.stack.push(self.input.chars().next().unwrap_or(0 as char) as u64);
                    self.input.clear();
                },

                // String mode
                b'"' => self.is_string_mode = true,

                // End program
                b'@' => self.is_running = false,

                // Digits
                b'0'..=b'9' => self.stack.push((op - b'0') as u64),

                _ => (),
            }
        }

        match self.direction {
            Direction::Up => self.position.y -= 1,
            Direction::Down => self.position.y += 1,
            Direction::Left => self.position.x -= 1,
            Direction::Right => self.position.x += 1,
        }

        NeedsInputType::None
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
        assert_eq!(state.stack, vec![b'r' as u64, b' ' as u64]);
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
}
