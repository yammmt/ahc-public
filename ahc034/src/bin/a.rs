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

    // 現在地から最も近い「正のマス」を探す（島が分断されてもBFSで確実に見つける）
    fn nearest_positive_pos(&self, (bi, bj): (usize, usize)) -> Option<(usize, usize)> {
        let mut vdq = VecDeque::new();
        let mut visited = vec![vec![false; N]; N];
        vdq.push_back((bi, bj));
        while let Some((ci, cj)) = vdq.pop_front() {
            if visited[ci][cj] {
                continue;
            }
            visited[ci][cj] = true;

            if self.hnn[ci][cj] > 0 {
                return Some((ci, cj));
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
        None
    }

    // 現在地から最も近い「負のマス」を探す
    fn nearest_negative_pos(&self, (bi, bj): (usize, usize)) -> Option<(usize, usize)> {
        let mut vdq = VecDeque::new();
        let mut visited = vec![vec![false; N]; N];
        vdq.push_back((bi, bj));
        while let Some((ci, cj)) = vdq.pop_front() {
            if visited[ci][cj] {
                continue;
            }
            visited[ci][cj] = true;

            if self.hnn[ci][cj] < 0 {
                return Some((ci, cj));
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
        None
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
        // 1. 最寄りの正のマスを見つけて、そこへ移動して吸えるだけ吸う（これを積載が十分になるか正のマスがなくなるまで繰り返す）
        let mut collected = false;
        while board.operations.len() < TURN_MAX {
            if let Some(target_pos) = board.nearest_positive_pos(board.pos) {
                board.move_to(target_pos);
                let h = board.hnn[board.pos.0][board.pos.1];
                if h > 0 {
                    board.work(Operation::Pop(h as usize));
                    collected = true;
                }
                // 一定以上積み込んだら一旦下ろしに行く（抱えすぎによる移動コスト増大の防止）
                if board.load > 50 {
                    break;
                }
            } else {
                break; // 正のマスが完全になくなった
            }
        }

        // 2. 抱えた荷物があるなら、最寄りの負のマスへ届ける
        if board.load > 0 {
            while board.operations.len() < TURN_MAX && board.load > 0 {
                if let Some(target_pos) = board.nearest_negative_pos(board.pos) {
                    board.move_to(target_pos);
                    let h = board.hnn[board.pos.0][board.pos.1];
                    if h < 0 {
                        let dump = (h.abs() as usize).min(board.load);
                        board.work(Operation::Push(dump));
                    }
                } else {
                    break; // 負のマスが完全になくなった
                }
            }
        } else if !collected {
            // 正のマスも負のマスも変化させられなかった場合は無限ループ回避のため終了
            break;
        }
    }

    for a in board.operations {
        println!("{a}");
    }
}
