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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
enum CraneMove {
    Lift,
    Drop,
    Up,
    Down,
    Left,
    Right,
    Wait,
    Remove,
}

impl CraneMove {
    fn to_ans(&self) -> char {
        match self {
            CraneMove::Lift => 'P',
            CraneMove::Drop => 'Q',
            CraneMove::Up => 'U',
            CraneMove::Down => 'D',
            CraneMove::Left => 'L',
            CraneMove::Right => 'R',
            CraneMove::Wait => '.',
            CraneMove::Remove => 'B',
        }
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
    let mut board = vec![vec![vec![BoardStatus::Empty; 5]; 5]; TURN_MAX];

    // 進捗管理
    let mut aidx = vec![0; n];
    let mut container_goal_num = 0;
    let mut goal_want = [Some(0), Some(5), Some(10), Some(15), Some(20)];

    // 大クレーンが運ぶ数字を五択
    // 大クレーン最短経路中に小クレーンも左端を空ける or 次の数字を準備するをする
    // 大クレーンゴール時に盤面のスコアを出す
    // 大クレーンをなるべく使わない方がよい気がするな
    // 一気に全部吐き出すと小クレーンの経路が大きく制限されてしまう
    // 初期に吐き出すパスを偶数 or 奇数行にすれば, 必ず 0 を引ける？
    // 一ターンずつ操作を決定するより, 一つのクレーンを決める => 余った経路でうまく残りのクレーンを動かす,
    // とした方がトータルでは賢いみたい

    // 0-origin 偶数行目に idx-0 があれば, 0 列目登場時点で小さいレーン 1 or 3 をゴールまで導く
    // 偶数レーンは四列目埋まるまでは何も考えず掃き出す

    // 四列掃き出し後に最もゴールに近いものを運ぶ
    // 0000 を手計算してもどうにも大クレーン依存が強く前半ほぼほぼ動けない
    // とはいえども後半はそこそこ改善できる
    //     => 一旦実装する？
    // なんいせよ全手の盤面を保存しないとだめっぽい
    // 最終的な盤面の状態を記憶しておき, 遡って動かす

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
        board[init_move.len()][i][3] = BoardStatus::Container(ann[i][0]);
        board[init_move.len()][i][2] = BoardStatus::Container(ann[i][1]);
        board[init_move.len()][i][1] = BoardStatus::Container(ann[i][2]);
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
        for i in 0..n {
            for j in 0..n {
                board[turn_cur][i][j] = board[turn_cur - 1][i][j];
            }
        }
        for i in 0..n {
            debug!("  {:?}", board[turn_cur][i]);
        }

        // 流れてきたものを受け取る
        for i in 0..n {
            if board[turn_cur - 1][i][0] != BoardStatus::Empty || aidx[i] >= n {
                // コンテナが存在するので受け取れない
                // "コンテナを掴んだ状態のクレーン" は BoardStatus では区別しないので
                // これだけ見れば良い
                continue;
            }

            board[turn_cur][i][0] = BoardStatus::Container(ann[i][aidx[i]]);
            aidx[i] += 1;
        }

        // クレーンを動かす
        for (i, c) in cranes.iter_mut().enumerate() {
            if i != 0 && *c != CraneStatus::Removed {
                // めんどいので小クレーンは最初に爆破する
                // TODO: 小クレーンは右端がどかせるならどかす, を初期状態だけでもすべき
                ans[i].push(CraneMove::Remove.to_ans());
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
                            if board[turn_cur][p0][p1] == BoardStatus::Container(cid) {
                                debug!("p0: {p0}, p1: {p1}, cid: {cid}");
                                // 現在位置があっていれば pick
                                ans[i].push(CraneMove::Lift.to_ans());
                                *c = CraneStatus::BigLift(p0, p1);

                                crane_strategies[i] = if goal_want[cid / 5] == Some(cid) {
                                    // そのままゴールへ
                                    CraneStrategy::Move(cid, (cid / 5, 4))
                                } else {
                                    // 適当な空きマスに置いて, コンテナを流してもらう
                                    // TODO: 一時置きの先はゴールに近いほうがよい
                                    let mut tmp_goal = (0, 0);
                                    let mut tmp_goal_dist = usize::MAX / 2;
                                    for ii in 0..n {
                                        for jj in 0..n-1 {
                                            if board[turn_cur][ii][jj] == BoardStatus::Empty {
                                                let dist_x = p0.max(ii) - p0.min(ii);
                                                let dist_y = p1.max(jj) - p1.min(jj);
                                                if dist_x + dist_y <= tmp_goal_dist {
                                                    tmp_goal = (ii, jj);
                                                    tmp_goal_dist = dist_x + dist_y;
                                                }
                                            }
                                        }
                                    }
                                    CraneStrategy::Move(cid, (tmp_goal.0, tmp_goal.1))
                                };

                                // TODO: 大クレーン以外を考えるとエンバグするかも
                                board[turn_cur][p0][p1] = BoardStatus::Empty;
                            } else {
                                // 近い側に動く
                                // TODO: 目標位置は都度検索すべきでないのでは？
                                let mut cid_i = 0;
                                let mut cid_j = 0;
                                for ii in 0..n {
                                    for jj in 0..n {
                                        if board[turn_cur][ii][jj] == BoardStatus::Container(cid) {
                                            cid_i = ii;
                                            cid_j = jj;
                                        }
                                    }
                                }
                                if p0 > cid_i {
                                    ans[i].push(CraneMove::Up.to_ans());
                                    *c = CraneStatus::BigEmpty(p0 - 1, p1);
                                } else if p0 < cid_i {
                                    ans[i].push(CraneMove::Down.to_ans());
                                    *c = CraneStatus::BigEmpty(p0 + 1, p1);
                                } else if p1 > cid_j {
                                    ans[i].push(CraneMove::Left.to_ans());
                                    *c = CraneStatus::BigEmpty(p0, p1 - 1);
                                } else {
                                    ans[i].push(CraneMove::Right.to_ans());
                                    *c = CraneStatus::BigEmpty(p0, p1 + 1);
                                }
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                CraneStrategy::Move(cid, (goal_i, goal_j)) => {
                    match *c {
                        CraneStatus::BigLift(p0, p1) => {
                            if p0 == goal_i && p1 == goal_j {
                                // 現在位置があっていれば下ろす
                                ans[i].push(CraneMove::Drop.to_ans());
                                *c = CraneStatus::BigEmpty(p0, p1);
                                crane_strategies[i] = CraneStrategy::Wait;
                                if p1 == 4 {
                                    container_goal_num += 1;
                                    goal_want[goal_i] = if cid % 5 == 4 {
                                        None
                                    } else {
                                        // FIXME: 転倒を許可する場合に詰む
                                        Some(goal_want[goal_i].unwrap() + 1)
                                    };
                                } else {
                                    board[turn_cur][p0][p1] = BoardStatus::Container(cid);
                                }
                            } else {
                                debug!("  {p0},{p1} -> {goal_i},{goal_j}");
                                // 近い側に動く
                                // TODO: 近い側に動く操作は共通化したいが
                                if p0 > goal_i {
                                    ans[i].push(CraneMove::Up.to_ans());
                                    *c = CraneStatus::BigLift(p0 - 1, p1);
                                } else if p0 < goal_i {
                                    ans[i].push(CraneMove::Down.to_ans());
                                    *c = CraneStatus::BigLift(p0 + 1, p1);
                                } else if p1 > goal_j {
                                    ans[i].push(CraneMove::Left.to_ans());
                                    *c = CraneStatus::BigLift(p0, p1 - 1);
                                } else {
                                    ans[i].push(CraneMove::Right.to_ans());
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
                            if let BoardStatus::Container(cid) = board[turn_cur][ii][jj] {
                                for want in &goal_want {
                                    if let Some(w) = want {
                                        if cid == *w {
                                            // 転倒数ペナルティが発生しない
                                            next_cid = Some(cid);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if next_cid.is_none() {
                        // 置いてもらうために一時置きする
                        for ii in 0..n {
                            if aidx[ii] == n {
                                continue;
                            }

                            for want in &goal_want {
                                if let Some(w) = want {
                                    if ann[ii][aidx[ii]] == *w {
                                        // これを置きたい
                                        assert!(board[turn_cur][ii][0] != BoardStatus::Empty);

                                        // 適当に周囲の空きマスにもっていかせる
                                        // この書き方だと何度か上書きされ得るが,
                                        // 実際の性能にはあまり効かないはず
                                        if let BoardStatus::Container(cid) = board[turn_cur][ii][0]
                                        {
                                            next_cid = Some(cid);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    // FIXME: 奥に 0, 5, ... が固まっていると動けなくなる
                    //       このときは初手が今と同じ処理ではどうしようもない
                    //       でも発生率 1/50000 くらいであり, 無視してもサンプル 100 入力は通る
                    if next_cid.is_none() {
                    }

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
