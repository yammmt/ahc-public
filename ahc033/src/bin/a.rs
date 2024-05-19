use proconio::fastout;
use proconio::input;

macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            println!($($arg)*);
        }
    };
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CraneStrategy {
    // container ID
    Pick(usize),
    // (container ID, goal (i, j))
    Move(usize, (usize, usize)),
    Wait,
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

    // 進捗管理
    let mut aidx = vec![0; n];
    let mut container_goal_num = 0;

    // とりあえず 3 列目までの 20 個を全部出して, 残りは %5 が小さいものから順に突っ込む
    // 搬出先と順番さえあっていれば, 大クレーンで一つずつ誘導するとして,
    // 高々往復 20 ターンを 25 回繰り返すだけだから 500 点を下回るくらいに落ち着くはず

    // 三列目まで全部出す
    // TODO: 固定操作を置き換えられれば, 列 1 はこの時点で sorted にできる
    let init_move = "PRRRQLLLPRRQLLPRQ".chars().collect::<Vec<char>>();
    for i in 0..5 {
        for c in &init_move {
            ans[i].push(*c);
        }
        board[i][3] = BoardStatus::Container(ann[i][0]);
        board[i][2] = BoardStatus::Container(ann[i][1]);
        board[i][1] = BoardStatus::Container(ann[i][2]);
        aidx[i] = 3;
    }
    // 前処理のため開始位置が固定
    let mut cranes = [
        CraneStatus::BigEmpty(0, 1),
        CraneStatus::SmallEmpty(1, 1),
        CraneStatus::SmallEmpty(2, 1),
        CraneStatus::SmallEmpty(3, 1),
        CraneStatus::SmallEmpty(4, 1),
    ];

    let mut turn_cur = init_move.len();
    let mut crane_strategies = vec![CraneStrategy::Wait; CRANE_NUM];

    while turn_cur <= TURN_MAX && container_goal_num < n * n {
        debug!("turn: {turn_cur}");
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
        for (i, c) in cranes.iter_mut().enumerate() {
            if i != 0 && *c != CraneStatus::Removed {
                // めんどいので小クレーンは最初に爆破する
                // TODO: 小クレーンは右端がどかせるならどかす, を初期状態だけでもすべき
                ans[i].push('B');
                *c = CraneStatus::Removed;
                crane_strategies[i] = CraneStrategy::Removed;
            }

            // debug
            if i != 0 {
                continue;
            }

            debug!("  strategy: {:?}", crane_strategies[i]);
            debug!("  c: {:?}", c);
            match crane_strategies[i] {
                CraneStrategy::Pick(cid) => {
                    match *c {
                        CraneStatus::BigEmpty(p0, p1) => {
                            if board[p0][p1] == BoardStatus::Container(cid) {
                                debug!("p0: {p0}, p1: {p1}, cid: {cid}");
                                // 現在位置があっていれば pick
                                ans[i].push('P');
                                *c = CraneStatus::BigLift(p0, p1);
                                crane_strategies[i] = CraneStrategy::Move(cid, (cid / 5, 4));
                                // TODO: 大クレーン以外を考えるとエンバグするかも
                                board[p0][p1] = BoardStatus::Empty;
                            } else {
                                // 近い側に動く
                                // TODO: 目標位置は都度検索すべきでないのでは？
                                let mut cid_i = 0;
                                let mut cid_j = 0;
                                for ii in 0..n {
                                    for jj in 0..n {
                                        if board[ii][jj] == BoardStatus::Container(cid) {
                                            cid_i = ii;
                                            cid_j = jj;
                                        }
                                    }
                                }
                                if p0 > cid_i {
                                    ans[i].push('U');
                                    *c = CraneStatus::BigEmpty(p0 - 1, p1);
                                } else if p0 < cid_i {
                                    ans[i].push('D');
                                    *c = CraneStatus::BigEmpty(p0 + 1, p1);
                                } else if p1 > cid_j {
                                    ans[i].push('L');
                                    *c = CraneStatus::BigEmpty(p0, p1 - 1);
                                } else {
                                    ans[i].push('R');
                                    *c = CraneStatus::BigEmpty(p0, p1 + 1);
                                }
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                CraneStrategy::Move(_cid, (goal_i, goal_j)) => {
                    match *c {
                        CraneStatus::BigLift(p0, p1) => {
                            if p0 == goal_i && p1 == goal_j {
                                // 現在位置があっていれば下ろす
                                ans[i].push('Q');
                                *c = CraneStatus::BigEmpty(p0, p1);
                                crane_strategies[i] = CraneStrategy::Wait;
                                if p1 == 4 {
                                    container_goal_num += 1;
                                }
                            } else {
                                debug!("  {p0},{p1} -> {goal_i},{goal_j}");
                                // 近い側に動く
                                // TODO: 近い側に動く操作は共通化したいが
                                if p0 > goal_i {
                                    ans[i].push('U');
                                    *c = CraneStatus::BigLift(p0 - 1, p1);
                                } else if p0 < goal_i {
                                    ans[i].push('D');
                                    *c = CraneStatus::BigLift(p0 + 1, p1);
                                } else if p1 > goal_j {
                                    ans[i].push('L');
                                    *c = CraneStatus::BigLift(p0, p1 - 1);
                                } else {
                                    ans[i].push('R');
                                    *c = CraneStatus::BigLift(p0, p1 + 1);
                                }
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                CraneStrategy::Wait => {
                    // 次に拾うものを決める
                    // 盤面に %5 が 0 のやつがなければ 1 のやつで妥協するので 100 点ペナルティ
                    // TODO: 空きマスに一時置きするとペナルティを回避できる場合がある
                    // TODO: %5 最小というよりペナルティ量最小を選びたい
                    let mut next_cid = None;
                    for ii in 0..n {
                        for jj in 0..n {
                            if let BoardStatus::Container(cid) = board[ii][jj] {
                                if next_cid.is_none() || cid % 5 < next_cid.unwrap() % 5 {
                                    next_cid = Some(cid);
                                }
                            }
                        }
                    }
                    assert!(next_cid.is_some());

                    crane_strategies[i] = CraneStrategy::Pick(next_cid.unwrap());
                }
                CraneStrategy::Removed => {}
            }
        }
    }
    // 一応
    assert!(!(turn_cur == TURN_MAX && container_goal_num < n * n));

    for a in ans {
        println!("{}", a.iter().collect::<String>());
    }
}
