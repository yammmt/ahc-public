use proconio::input;
use proconio::marker::Bytes;
use std::collections::VecDeque;
use std::fmt::Write;

const DIRECTIONS: [(isize, isize, char); 4] =
    [(-1, 0, 'U'), (1, 0, 'D'), (0, -1, 'L'), (0, 1, 'R')];
const MAX_OPERATIONS: usize = 100_000;
const MAX_HEIGHT: usize = 8;

#[derive(Clone, Copy, Default)]
struct Stack {
    colors: [u8; MAX_HEIGHT], // Bottom to top in colors[..len].
    len: usize,
}

impl Stack {
    fn len(&self) -> usize {
        self.len
    }

    fn last(&self) -> Option<u8> {
        (self.len > 0).then(|| self.colors[self.len - 1])
    }

    fn push(&mut self, color: u8) {
        assert!(self.len < MAX_HEIGHT);
        self.colors[self.len] = color;
        self.len += 1;
    }

    fn pop(&mut self) -> Option<u8> {
        if self.len == 0 {
            return None;
        }
        self.len -= 1;
        Some(self.colors[self.len])
    }
}

struct Board {
    n: usize,
    walls: Vec<bool>,
    nests: Vec<Option<u8>>,
    stacks: Vec<Stack>,
}

impl Board {
    fn adjacent(&self, cell: usize, direction: usize) -> Option<usize> {
        let (di, dj, _) = DIRECTIONS[direction];
        let i = (cell / self.n).checked_add_signed(di)?;
        let j = (cell % self.n).checked_add_signed(dj)?;
        if i >= self.n || j >= self.n {
            return None;
        }
        let next = i * self.n + j;
        (!self.walls[next]).then_some(next)
    }

    fn return_home(&mut self, cell: usize) {
        if let Some(color) = self.nests[cell] {
            while self.stacks[cell].last() == Some(color) {
                self.stacks[cell].pop();
            }
        }
    }

    fn apply(&mut self, action: Action) {
        let height = self.stacks[action.from].len();
        assert!(action.k < height);
        assert!((1..=action.k + 1).contains(&action.length));

        let mut to = action.from;
        for _ in 0..action.length {
            to = self.adjacent(to, action.direction).unwrap();
        }
        let jumping_count = height - action.k;
        assert!(self.stacks[to].len() + jumping_count <= MAX_HEIGHT);

        let mut jumping = [0; MAX_HEIGHT];
        for (index, slot) in jumping.iter_mut().take(jumping_count).enumerate() {
            *slot = self.stacks[action.from].colors[height - 1 - index];
        }
        self.stacks[action.from].len = action.k;
        for &color in jumping.iter().take(jumping_count) {
            self.stacks[to].push(color);
        }
        self.return_home(action.from);
        self.return_home(to);
    }
}

#[derive(Clone, Copy)]
struct Action {
    from: usize,
    k: usize,
    direction: usize,
    length: usize,
}

#[derive(Clone, Copy)]
struct Slime {
    cell: usize,
    color: usize,
}

fn distances_from(board: &Board, start: usize) -> Vec<usize> {
    let mut distances = vec![usize::MAX; board.n * board.n];
    let mut queue = VecDeque::new();
    distances[start] = 0;
    queue.push_back(start);

    while let Some(cell) = queue.pop_front() {
        for direction in 0..DIRECTIONS.len() {
            let Some(next) = board.adjacent(cell, direction) else {
                continue;
            };
            if distances[next] == usize::MAX {
                distances[next] = distances[cell] + 1;
                queue.push_back(next);
            }
        }
    }
    distances
}

fn choose_step(board: &Board, cell: usize, distances: &[usize]) -> (usize, usize) {
    let current = distances[cell];
    for direction in 0..DIRECTIONS.len() {
        if let Some(next) = board.adjacent(cell, direction)
            && distances[next] < current
        {
            return (next, direction);
        }
    }
    unreachable!("Every floor cell is connected to its nest")
}

fn main() {
    input! {
        n: usize,
        k: usize,
        rows: [Bytes; n],
    }

    let mut board = Board {
        n,
        walls: vec![false; n * n],
        nests: vec![None; n * n],
        stacks: vec![Stack::default(); n * n],
    };
    let mut nest_cells = vec![usize::MAX; k];
    let mut slimes = Vec::new();

    for (i, row) in rows.iter().enumerate() {
        for (j, &symbol) in row.iter().enumerate() {
            let cell = i * n + j;
            match symbol {
                b'#' => board.walls[cell] = true,
                b'A'..=b'L' => {
                    let color = usize::from(symbol - b'A');
                    board.nests[cell] = Some(color as u8);
                    nest_cells[color] = cell;
                }
                b'a'..=b'l' => {
                    let color = usize::from(symbol - b'a');
                    board.stacks[cell].push(color as u8);
                    slimes.push(Slime { cell, color });
                }
                b'.' => {}
                _ => unreachable!("Unexpected input cell"),
            }
        }
    }

    let distances: Vec<_> = nest_cells
        .iter()
        .map(|&nest| distances_from(&board, nest))
        .collect();
    slimes.sort_by_key(|slime| std::cmp::Reverse(distances[slime.color][slime.cell]));

    let mut actions = Vec::new();
    'slimes: for slime in slimes {
        let mut cell = slime.cell;
        while cell != nest_cells[slime.color] {
            if actions.len() == MAX_OPERATIONS {
                break 'slimes;
            }
            assert_eq!(board.stacks[cell].last(), Some(slime.color as u8));
            let (next, direction) = choose_step(&board, cell, &distances[slime.color]);
            let action = Action {
                from: cell,
                k: board.stacks[cell].len() - 1,
                direction,
                length: 1,
            };
            board.apply(action);
            actions.push(action);
            cell = next;
        }
    }

    let mut output = String::new();
    for action in actions {
        let _ = writeln!(
            output,
            "{} {} {} {} {}",
            action.from / n,
            action.from % n,
            action.k,
            DIRECTIONS[action.direction].2,
            action.length,
        );
    }
    print!("{output}");
}
