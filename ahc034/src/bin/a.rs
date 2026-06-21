use proconio::fastout;
use proconio::input;
use std::collections::VecDeque;

const TURN_MAX: usize = 100_000;
const N: usize = 20;

#[derive(Debug, Clone, Copy)]
enum Operation {
    Pop(usize),
    Push(usize),
    Up,
    Down,
    Left,
    Right,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Pop(x) => write!(f, "+{x}"),
            Operation::Push(x) => write!(f, "-{x}"),
            Operation::Up => write!(f, "U"),
            Operation::Down => write!(f, "D"),
            Operation::Left => write!(f, "L"),
            Operation::Right => write!(f, "R"),
        }
    }
}

struct Board {
    load: usize,
    hnn: Vec<Vec<isize>>,
    pos: (usize, usize),
    cleared: usize,
    cost: usize,
    operations: Vec<Operation>,
}

impl Board {
    const DIR: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    fn work(&mut self, op: Operation) {
        self.operations.push(op);
        match op {
            Operation::Pop(x) => {
                let (i, j) = self.pos;
                if self.hnn[i][j] == x as isize {
                    self.cleared += 1;
                } else if self.hnn[i][j] == 0 {
                    self.cleared -= 1;
                }
                self.hnn[i][j] -= x as isize;
                self.load += x;
                self.cost += x as usize;
            }
            Operation::Push(x) => {
                let (i, j) = self.pos;
                if self.hnn[i][j] == -(x as isize) {
                    self.cleared += 1;
                } else if self.hnn[i][j] == 0 {
                    self.cleared -= 1;
                }
                self.hnn[i][j] += x as isize;
                self.load -= x;
                self.cost += x as usize;
            }
            Operation::Up => {
                self.pos.0 -= 1;
                self.cost += 100 + self.load;
            }
            Operation::Down => {
                self.pos.0 += 1;
                self.cost += 100 + self.load;
            }
            Operation::Left => {
                self.pos.1 -= 1;
                self.cost += 100 + self.load;
            }
            Operation::Right => {
                self.pos.1 += 1;
                self.cost += 100 + self.load;
            }
        }
    }

    fn nearest_negative_pos(&self, (bi, bj): (usize, usize)) -> (usize, usize) {
        let mut vdq = VecDeque::new();
        let mut visited = vec![vec![false; N]; N];
        vdq.push_back((bi, bj));
        while let Some((ci, cj)) = vdq.pop_front() {
            if visited[ci][cj] {
                continue;
            }

            visited[ci][cj] = true;

            if self.hnn[ci][cj] < 0 {
                return (ci, cj);
            }

            for &d in &Self::DIR {
                let ni = ci.wrapping_add_signed(d.0);
                let nj = cj.wrapping_add_signed(d.1);
                if ni >= N || nj >= N || visited[ni][nj] {
                    continue;
                }

                vdq.push_back((ni, nj));
            }
        }

        (0, 0)
    }

    fn nearest_positive_pos(&self, (bi, bj): (usize, usize)) -> (usize, usize) {
        let mut vdq = VecDeque::new();
        let mut visited = vec![vec![false; N]; N];
        vdq.push_back((bi, bj));
        while let Some((ci, cj)) = vdq.pop_front() {
            if visited[ci][cj] {
                continue;
            }

            visited[ci][cj] = true;

            if self.hnn[ci][cj] > 0 {
                return (ci, cj);
            }

            for &d in &Self::DIR {
                let ni = ci.wrapping_add_signed(d.0);
                let nj = cj.wrapping_add_signed(d.1);
                if ni >= N || nj >= N || visited[ni][nj] {
                    continue;
                }

                vdq.push_back((ni, nj));
            }
        }

        (0, 0)
    }

    fn island_sum(&self, (bi, bj): (usize, usize)) -> isize {
        let mut vdq = VecDeque::new();
        let mut visited = vec![vec![false; N]; N];
        let mut ret = 0;
        vdq.push_back((bi, bj));
        while let Some((ci, cj)) = vdq.pop_front() {
            if visited[ci][cj] {
                continue;
            }

            visited[ci][cj] = true;
            ret += self.hnn[ci][cj];

            for &d in &Self::DIR {
                let ni = ci.wrapping_add_signed(d.0);
                let nj = cj.wrapping_add_signed(d.1);
                if ni >= N
                    || nj >= N
                    || visited[ni][nj]
                    || !((self.hnn[ci][cj].is_positive() && self.hnn[ni][nj].is_positive())
                        || (self.hnn[ci][cj].is_negative() && self.hnn[ni][nj].is_negative()))
                {
                    continue;
                }

                vdq.push_back((ni, nj));
            }
        }
        ret
    }

    fn move_to(&mut self, (ei, ej): (usize, usize)) {
        let (diff_i, diff_j) = (
            ei as isize - self.pos.0 as isize,
            ej as isize - self.pos.1 as isize,
        );
        if diff_i > 0 {
            for _ in 0..diff_i {
                self.work(Operation::Down);
            }
        } else if diff_i < 0 {
            for _ in 0..diff_i.abs() {
                self.work(Operation::Up);
            }
        }
        if diff_j > 0 {
            for _ in 0..diff_j {
                self.work(Operation::Right);
            }
        } else if diff_j < 0 {
            for _ in 0..diff_j.abs() {
                self.work(Operation::Left);
            }
        }
    }

    fn pop_island_paths(&self, (bi, bj): (usize, usize), load_max: usize) -> Vec<(usize, usize)> {
        let mut ret = vec![];
        let mut load_cur = 0;
        let mut stack = VecDeque::new();
        let mut visited = vec![vec![false; N]; N];

        stack.push_back(((bi, bj), false));
        while let Some(((ci, cj), finished)) = stack.pop_back() {
            if finished {
                ret.push((ci, cj));
                continue;
            }

            if visited[ci][cj] {
                continue;
            }

            visited[ci][cj] = true;
            ret.push((ci, cj));
            load_cur += self.hnn[ci][cj].abs();
            if load_cur >= load_max as isize {
                return ret;
            }

            stack.push_back(((ci, cj), true));

            for &d in &Self::DIR {
                let ni = ci.wrapping_add_signed(d.0);
                let nj = cj.wrapping_add_signed(d.1);
                if ni >= N
                    || nj >= N
                    || visited[ni][nj]
                    || !((self.hnn[ci][cj].is_positive() && self.hnn[ni][nj].is_positive())
                        || (self.hnn[ci][cj].is_negative() && self.hnn[ni][nj].is_negative()))
                {
                    continue;
                }

                stack.push_back(((ni, nj), false));
            }
        }

        ret
    }
}

#[fastout]
fn main() {
    input! {
        _n: usize,
        hnn: [[isize; N]; N],
    }

    let mut cleared = 0;
    for r in &hnn {
        for &v in r {
            if v == 0 {
                cleared += 1;
            }
        }
    }

    let mut board = Board {
        load: 0,
        hnn,
        pos: (0, 0),
        cleared,
        cost: 0,
        operations: vec![],
    };

    while board.operations.len() < TURN_MAX && board.cleared < N * N {
        // 1. 最寄りの正の島を探し、入口へ移動
        let pos_start = board.nearest_positive_pos(board.pos);
        if board.hnn[pos_start.0][pos_start.1] <= 0 {
            break; // 正のマスが枯渇
        }
        board.move_to(pos_start);

        // 2. 正の島を巡回し、限界（今回はusize::MAX）まで積む
        let load_path = board.pop_island_paths(board.pos, usize::MAX);
        for p in load_path {
            if board.operations.len() >= TURN_MAX {
                break;
            }
            board.move_to(p);
            let h = board.hnn[board.pos.0][board.pos.1];
            if h > 0 {
                board.work(Operation::Pop(h as usize));
            }
        }

        if board.load == 0 {
            break; // 積み込み失敗時の無限ループ防止
        }

        // 3. 最寄りの負の島を探し、入口へ移動
        let neg_start = board.nearest_negative_pos(board.pos);
        if board.hnn[neg_start.0][neg_start.1] >= 0 {
            break; // 負のマスが枯渇
        }
        board.move_to(neg_start);

        // 4. 負の島を巡回し、持っている積載量分だけ下ろす
        let dump_path = board.pop_island_paths(board.pos, board.load);
        for p in dump_path {
            if board.operations.len() >= TURN_MAX || board.load == 0 {
                break;
            }
            board.move_to(p);
            let h = board.hnn[board.pos.0][board.pos.1];
            if h < 0 {
                let dump = (h.abs() as usize).min(board.load);
                board.work(Operation::Push(dump));
            }
        }
    }

    for a in board.operations {
        println!("{a}");
    }
}
