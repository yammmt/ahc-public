// AI 課金実装コンテスト...

use proconio::fastout;
use proconio::input;

#[allow(unused)]
const TURN_MAX: usize = 100_000;
const N: usize = 20;

#[derive(Debug, Clone, Copy)]
enum Operation {
    /// ダンプカーに積み込む, 現在位置に対しての pop
    Pop(usize),
    /// ダンプカーから下ろす, 現在位置に対しての push
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
    #[allow(unused)]
    const DIR: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

    /// 指定位置に 1 マスだけ移動する.マス間の距離が 1 でなければ assert する.
    fn move_to(&mut self, (r_nxt, c_nxt): (usize, usize)) {
        let (r_cur, c_cur) = self.pos;
        let r_diff = r_nxt as isize - r_cur as isize;
        let c_diff = c_nxt as isize - c_cur as isize;
        assert_eq!(r_diff.abs() + c_diff.abs(), 1);

        if r_diff > 0 {
            self.work(Operation::Down);
        } else if r_diff < 0 {
            self.work(Operation::Up);
        } else if c_diff > 0 {
            self.work(Operation::Right);
        } else if c_diff < 0 {
            self.work(Operation::Left);
        } else {
            unreachable!("move_to failed");
        }
    }

    /// ダンプカーに積み込む, 現在位置に対しての pop
    fn manual_pop(&mut self, amount: usize) {
        self.work(Operation::Pop(amount));
    }

    /// ダンプカーから下ろす, 現在位置に対しての push
    fn manual_push(&mut self, amount: usize) {
        assert!(amount >= self.load);

        self.work(Operation::Push(amount));
    }

    /// 現在位置を平らにする.
    /// 現在位置を埋める場合は平らにならない可能性があるが, 平らにならなかった場合にも
    /// 特に通知を行わない.
    fn flatten(&mut self) {
        let (r, c) = self.pos;
        let v = self.hnn[r][c];
        if v > 0 {
            self.work(Operation::Pop(v.unsigned_abs()));
        } else if v < 0 && self.load > 0 {
            self.work(Operation::Push(v.unsigned_abs().min(self.load)));
        }
    }

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
                self.cost += x;
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
                self.cost += x;
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

    // 解説放送のルールベースを手実装

    // 左右往復
    // 列で必要な積み込み量を先に記憶しておき, 分割位置マスで前借りする
    // 分割位置到達時に積荷をすべて下ろす
    // 最後に左下 -> 左上で辻褄をあわせる

    let storage_j = N / 2;

    // 始点まで動く
    for _ in 0..storage_j {
        board.work(Operation::Right);
    }

    // 左半分
    for i in 0..N / 2 {
        // 往復の積載量計算
        let mut h_total = 0;
        let mut h_total_min = 0;
        for j in (0..storage_j).rev() {
            h_total += board.hnn[2 * i][j];
            h_total_min = h_total_min.min(h_total);
        }
        for j in 0..storage_j {
            h_total += board.hnn[2 * i + 1][j];
            h_total_min = h_total_min.min(h_total);
        }

        // 右 -> 左
        let mut routes = vec![];
        for j in (0..storage_j).rev() {
            routes.push((2 * i, j));
        }
        // 降りて左 -> 右
        for j in 0..=storage_j {
            routes.push((2 * i + 1, j));
        }

        for (idx, &(r_nxt, c_nxt)) in routes.iter().enumerate() {
            if idx == 0 {
                if h_total_min < 0 {
                    board.manual_pop(h_total_min.unsigned_abs());
                }
            } else {
                board.flatten();
            }
            board.move_to((r_nxt, c_nxt));
        }
        if board.load > 0 {
            board.manual_push(board.load);
        }

        // 最終行でなければ, 降りる
        if 2 * i + 2 < N {
            board.move_to((2 * i + 2, storage_j));
        }
    }

    // 右半分
    for i in (0..N / 2).rev() {
        // 往復の積載量計算
        let mut h_total = 0;
        let mut h_total_min = 0;
        for j in 1..storage_j {
            h_total += board.hnn[2 * i + 1][storage_j + j];
            h_total_min = h_total_min.min(h_total);
        }
        for j in (1..storage_j).rev() {
            h_total += board.hnn[2 * i][storage_j + j];
            h_total_min = h_total_min.min(h_total);
        }

        let mut routes = vec![];
        // 左 -> 右
        for j in 1..storage_j {
            routes.push((2 * i + 1, storage_j + j));
        }
        // 上がって右 -> 左
        for j in (0..storage_j).rev() {
            routes.push((2 * i, storage_j + j));
        }

        for (idx, &(r_nxt, c_nxt)) in routes.iter().enumerate() {
            if idx == 0 {
                if h_total_min < 0 {
                    board.manual_pop(h_total_min.unsigned_abs());
                }
            } else {
                board.flatten();
            }
            board.move_to((r_nxt, c_nxt));
        }
        if board.load > 0 {
            board.manual_push(board.load);
        }

        // 最終行 (行 0) でなければ, 上がる
        if i != 0 {
            board.move_to((2 * i - 1, storage_j));
        }
    }

    // 上 -> 下
    let mut routes = vec![];
    for i in 1..N {
        routes.push((i, storage_j));
    }
    for &(r_nxt, c_nxt) in routes.iter() {
        board.flatten();
        board.move_to((r_nxt, c_nxt));
    }
    board.flatten();

    // 下 -> 上
    let mut routes = vec![];
    for i in (0..N - 1).rev() {
        routes.push((i, storage_j));
    }
    for &(r_nxt, c_nxt) in routes.iter() {
        board.flatten();
        if board.cleared == N * N {
            break;
        }

        board.move_to((r_nxt, c_nxt));
    }
    if board.cleared != N * N {
        board.flatten();
    }

    for a in board.operations {
        println!("{a}");
    }
}
