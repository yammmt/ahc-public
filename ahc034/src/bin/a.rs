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

    // 左から右に進む -> 右到達したら下, を繰り返す
    // 列で必要な積み込み量を先に記憶しておき, 左端マスで前借りする
    // 左端到達時に積荷をすべて下ろす
    // 最後に左下 -> 左上で辻褄をあわせる

    // 0-origin の偶数番目の左端到着時に行って帰ってくる際の回収量を算出する
    // 回収量が負ならその分を前借りして掘る
    // 着数番目到着時には下ろす

    for i in 0..N / 2 {
        // 左端から一往復する際の積載量計算
        let mut h_total = 0;
        let mut h_total_min = 0;
        for j in 1..N {
            h_total += board.hnn[2 * i][j];
            h_total_min = h_total_min.min(h_total);
        }
        for j in (0..N).rev() {
            h_total += board.hnn[2 * i + 1][j];
            h_total_min = h_total_min.min(h_total);
        }
        h_total = h_total_min;

        // 左 -> 右
        for j in 0..N {
            if j == 0 {
                if h_total < 0 {
                    board.work(Operation::Pop(h_total.unsigned_abs()));
                }
            } else if board.hnn[2 * i][j] > 0 {
                board.work(Operation::Pop(board.hnn[2 * i][j].unsigned_abs()));
            } else if board.hnn[2 * i][j] < 0 {
                board.work(Operation::Push(board.hnn[2 * i][j].unsigned_abs()));
            }

            if j == N - 1 {
                // 右端一マス降りる
                board.work(Operation::Down);
            } else {
                board.work(Operation::Right);
            }
        }

        // 右 -> 左
        for j in (0..N).rev() {
            if j == 0 {
                if board.load > 0 {
                    board.work(Operation::Push(board.load));
                }
            } else if board.hnn[2 * i + 1][j] > 0 {
                board.work(Operation::Pop(board.hnn[2 * i + 1][j].unsigned_abs()));
            } else if board.hnn[2 * i + 1][j] < 0 {
                board.work(Operation::Push(board.hnn[2 * i + 1][j].unsigned_abs()));
            }

            if j == 0 && i != N / 2 - 1 {
                board.work(Operation::Down);
            } else if j != 0 {
                board.work(Operation::Left);
            }
        }
    }

    // 最後には上に向かって辻褄
    for i in (0..N).rev() {
        if board.hnn[i][0] > 0 {
            board.work(Operation::Pop(board.hnn[i][0] as usize));
        } else if board.hnn[i][0] < 0 && board.load > 0 {
            board.work(Operation::Push(
                board.hnn[i][0].unsigned_abs().min(board.load),
            ));
        }

        if i != 0 {
            board.work(Operation::Up);
        }
    }

    // 下に向かって終わり
    for i in 0..N {
        if board.hnn[i][0] < 0 {
            board.work(Operation::Push(board.hnn[i][0].unsigned_abs()));
        }

        if board.cleared < N * N {
            board.work(Operation::Down);
        }
    }

    for a in board.operations {
        println!("{a}");
    }
}
