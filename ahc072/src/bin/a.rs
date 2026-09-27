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

    fn top_run_len(&self) -> usize {
        let Some(color) = self.last() else {
            return 0;
        };
        let mut count = 0;
        while count < self.len && self.colors[self.len - 1 - count] == color {
            count += 1;
        }
        count
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

type MovePriority = (bool, usize, usize, usize, usize);
type MoveCandidate = (MovePriority, Action, usize);

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

fn choose_target(board: &Board, distances: &[Vec<usize>]) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize, usize)> = None;
    for (cell, stack) in board.stacks.iter().enumerate() {
        if let Some(color) = stack.last() {
            let color = usize::from(color);
            let distance = distances[color][cell];
            if best.is_none_or(|(_, _, best_distance)| distance > best_distance) {
                best = Some((cell, color, distance));
            }
        }
    }
    best.map(|(cell, color, _)| (cell, color))
}

fn springboard_gain(board: &Board, cell: usize, moving: usize, distances: &[usize]) -> usize {
    let base_height = board.stacks[cell].len();
    let current_distance = distances[cell];
    let mut best_gain = 0;
    for direction in 0..DIRECTIONS.len() {
        let mut to = cell;
        for _ in 1..=base_height + 1 {
            let Some(next) = board.adjacent(to, direction) else {
                break;
            };
            to = next;
            if board.stacks[to].len() + moving <= MAX_HEIGHT {
                best_gain = best_gain.max(current_distance.saturating_sub(distances[to]));
            }
        }
    }
    best_gain.saturating_sub(1)
}

fn choose_move(
    board: &Board,
    cell: usize,
    color: usize,
    moving: usize,
    distances: &[usize],
) -> Option<(Action, usize)> {
    let height = board.stacks[cell].len();
    let k = height - moving;
    let current_distance = distances[cell];
    let mut best: Option<MoveCandidate> = None;

    for direction in 0..DIRECTIONS.len() {
        let mut to = cell;
        for length in 1..=k + 1 {
            let Some(next) = board.adjacent(to, direction) else {
                break;
            };
            to = next;
            let new_distance = distances[to];
            if new_distance >= current_distance || board.stacks[to].len() + moving > MAX_HEIGHT {
                continue;
            }
            let collected = if board.stacks[to].last() == Some(color as u8) {
                board.stacks[to].top_run_len()
            } else {
                0
            };
            let relay_gain = if board.stacks[to].len() > 0 && collected == 0 {
                springboard_gain(board, to, moving, distances)
            } else {
                0
            };
            let priority = (
                collected > 0,
                current_distance - new_distance,
                relay_gain,
                collected,
                length,
            );
            if best
                .as_ref()
                .is_none_or(|(best_priority, _, _)| priority > *best_priority)
            {
                best = Some((
                    priority,
                    Action {
                        from: cell,
                        k,
                        direction,
                        length,
                    },
                    to,
                ));
            }
        }
    }
    best.map(|(_, action, to)| (action, to))
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

    let mut actions = Vec::new();
    while actions.len() < MAX_OPERATIONS {
        let Some((mut cell, color)) = choose_target(&board, &distances) else {
            break;
        };
        while cell != nest_cells[color] && actions.len() < MAX_OPERATIONS {
            assert_eq!(board.stacks[cell].last(), Some(color as u8));
            let moving = board.stacks[cell].top_run_len();
            let (action, next) = choose_move(&board, cell, color, moving, &distances[color])
                .or_else(|| choose_move(&board, cell, color, 1, &distances[color]))
                .expect("A single slime can move toward its nest");
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
