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
        for j in (0..storage_j + 1).rev() {
            if j == storage_j {
                if h_total_min < 0 {
                    board.work(Operation::Pop(h_total_min.unsigned_abs()));
                }
            } else if board.hnn[2 * i][j] > 0 {
                board.work(Operation::Pop(board.hnn[2 * i][j].unsigned_abs()));
            } else if board.hnn[2 * i][j] < 0 {
                board.work(Operation::Push(board.hnn[2 * i][j].unsigned_abs()));
            }

            if j == 0 {
                // 左端一マス降りる
                board.work(Operation::Down);
            } else {
                board.work(Operation::Left);
            }
        }

        // 左 -> 右
        for j in 0..(storage_j + 1) {
            if j == storage_j {
                if board.load > 0 {
                    board.work(Operation::Push(board.load));
                }
                if i == N / 2 - 1 {
                    break;
                }
            } else if board.hnn[2 * i + 1][j] > 0 {
                board.work(Operation::Pop(board.hnn[2 * i + 1][j].unsigned_abs()));
            } else if board.hnn[2 * i + 1][j] < 0 {
                board.work(Operation::Push(board.hnn[2 * i + 1][j].unsigned_abs()));
            }

            if j == storage_j && i != N / 2 - 1 {
                board.work(Operation::Down);
            } else {
                board.work(Operation::Right);
            }
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

        // 左 -> 右
        for j in 0..storage_j {
            let j_real = storage_j + j;
            if j == 0 {
                if h_total_min < 0 {
                    board.work(Operation::Pop(h_total_min.unsigned_abs()));
                }
            } else if board.hnn[2 * i + 1][j_real] > 0 {
                board.work(Operation::Pop(board.hnn[2 * i + 1][j_real].unsigned_abs()));
            } else if board.hnn[2 * i + 1][j_real] < 0 {
                board.work(Operation::Push(board.hnn[2 * i + 1][j_real].unsigned_abs()));
            }

            if j_real == N - 1 {
                board.work(Operation::Up);
            } else {
                board.work(Operation::Right);
            }
        }

        // 右 -> 左
        for j in (0..storage_j).rev() {
            let j_real = storage_j + j;
            if j == 0 {
                if board.load > 0 {
                    board.work(Operation::Push(board.load));
                }
                if i == 0 {
                    break;
                }
            } else if board.hnn[2 * i][j_real] > 0 {
                board.work(Operation::Pop(board.hnn[2 * i][j_real].unsigned_abs()));
            } else if board.hnn[2 * i][j_real] < 0 {
                board.work(Operation::Push(board.hnn[2 * i][j_real].unsigned_abs()));
            }

            if j == 0 {
                board.work(Operation::Up);
            } else {
                board.work(Operation::Left);
            }
        }
    }

    // 上 -> 下
    for i in 0..N {
        if board.hnn[i][storage_j] > 0 {
            board.work(Operation::Pop(board.hnn[i][storage_j] as usize));
        } else if board.hnn[i][storage_j] < 0 && board.load > 0 {
            board.work(Operation::Push(
                board.hnn[i][storage_j].unsigned_abs().min(board.load),
            ));
        }

        if i != N - 1 {
            board.work(Operation::Down)
        }
    }

    // 下 -> 上
    for i in (0..N).rev() {
        if board.hnn[i][storage_j] < 0 {
            board.work(Operation::Push(board.hnn[i][storage_j].unsigned_abs()));
        }

        if board.cleared < N * N {
            board.work(Operation::Up);
        }
    }

    for a in board.operations {
        println!("{a}");
    }
}
