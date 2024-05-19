use proconio::fastout;
use proconio::input;

// N 固定だから vec を回避すればちょっとだけ高速化できる
const CRANE_NUM: usize = 5;
const TURN_MAX: usize = 10000;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoardStatus {
    Container(usize),
    Empty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CraneStatus {
    BigEmpty(usize, usize),
    BigLift(usize, usize),
    SmallEmpty(usize, usize),
    SmallLift(usize, usize),
    Removed,
}

impl CraneStatus {
    #[allow(dead_code)]
    fn pos(&self) -> Option<(usize, usize)> {
        match *self {
            CraneStatus::BigEmpty(v0, v1) => Some((v0, v1)),
            CraneStatus::BigLift(v0, v1) => Some((v0, v1)),
            CraneStatus::SmallEmpty(v0, v1) => Some((v0, v1)),
            CraneStatus::SmallLift(v0, v1) => Some((v0, v1)),
            CraneStatus::Removed => None,
        }
    }

    #[allow(dead_code)]
    fn is_lift(&self) -> bool {
        matches!(
            self,
            CraneStatus::BigLift(_, _) | CraneStatus::SmallLift(_, _)
        )
    }
}

#[fastout]
fn main() {
    input! {
        n: usize,
        ann: [[usize; n]; n],
    }

    let mut ans = vec![vec![]; CRANE_NUM];

    // 盤面管理
    let mut board = vec![vec![BoardStatus::Empty; 5]; 5];
    let mut cranes = [
        CraneStatus::BigEmpty(0, 0),
        CraneStatus::SmallEmpty(1, 0),
        CraneStatus::SmallEmpty(2, 0),
        CraneStatus::SmallEmpty(3, 0),
        CraneStatus::SmallEmpty(4, 0),
    ];
    let mut _crane_alive = vec![true; CRANE_NUM];

    // 進捗管理
    let mut aidx = vec![0; n];
    let mut turn_cur = 0;
    let mut container_goal_num = 0;
    while turn_cur <= TURN_MAX && container_goal_num < n * n {
        turn_cur += 1;

        // 流れてきたものを受け取る
        for i in 0..n {
            if board[i][0] != BoardStatus::Empty || aidx[i] >= n {
                // コンテナが存在するので受け取れない
                // "コンテナを掴んだ状態のクレーン" は BoardStatus では区別しないので
                // これだけ見れば良い
                continue;
            }

            board[i][0] = BoardStatus::Container(ann[i][aidx[i]]);
            aidx[i] += 1;
        }

        // クレーンを動かす
        // 愚直に L->R に流すだけ
        for (i, c) in cranes.iter_mut().enumerate() {
            match *c {
                CraneStatus::BigEmpty(p0, p1) => match board[p0][p1] {
                    BoardStatus::Container(_) => {
                        ans[i].push('P');
                        *c = CraneStatus::BigLift(p0, p1);
                    }
                    BoardStatus::Empty => {
                        ans[i].push('L');
                        *c = CraneStatus::BigEmpty(p0, p1 - 1);
                    }
                },
                CraneStatus::BigLift(p0, p1) => {
                    if p1 == 4 {
                        ans[i].push('Q');
                        container_goal_num += 1;
                        *c = CraneStatus::BigEmpty(p0, p1);
                        board[p0][p1] = BoardStatus::Empty;
                    } else {
                        ans[i].push('R');
                        *c = CraneStatus::BigLift(p0, p1 + 1);
                        board[p0][p1 + 1] = board[p0][p1];
                        board[p0][p1] = BoardStatus::Empty;
                    }
                }
                CraneStatus::SmallEmpty(p0, p1) => match board[p0][p1] {
                    BoardStatus::Container(_) => {
                        ans[i].push('P');
                        *c = CraneStatus::SmallLift(p0, p1);
                    }
                    BoardStatus::Empty => {
                        ans[i].push('L');
                        *c = CraneStatus::SmallEmpty(p0, p1 - 1);
                    }
                },
                CraneStatus::SmallLift(p0, p1) => {
                    if p1 == 4 {
                        ans[i].push('Q');
                        container_goal_num += 1;
                        *c = CraneStatus::SmallEmpty(p0, p1);
                        board[p0][p1] = BoardStatus::Empty;
                    } else {
                        ans[i].push('R');
                        *c = CraneStatus::SmallLift(p0, p1 + 1);
                        board[p0][p1 + 1] = board[p0][p1];
                        board[p0][p1] = BoardStatus::Empty;
                    }
                }
                CraneStatus::Removed => {}
            }
        }
    }
    // 一応
    assert!(!(turn_cur == TURN_MAX && container_goal_num < n * n));

    for a in ans {
        println!("{}", a.iter().collect::<String>());
    }
}
