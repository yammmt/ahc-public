// 異常なほどの重実装になっているのだがどうして

use proconio::input;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;

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
const TURN_MAX: usize = 1000;
const TURN_WAIT_LONGEST: usize = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BoardStatus {
    Container(usize),
    Empty,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ContainerStatus {
    Free,
    Accepted(usize),
    BeingMoved(usize),
    Completed,
}

impl ContainerStatus {
    fn moved_by(&self) -> Option<usize> {
        match *self {
            ContainerStatus::Accepted(c) | ContainerStatus::BeingMoved(c) => Some(c),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CraneStatus {
    BigEmpty((usize, usize)),
    BigLift((usize, usize), usize),
    SmallEmpty((usize, usize)),
    SmallLift((usize, usize), usize),
    Removed,
}

impl CraneStatus {
    #[allow(dead_code)]
    fn pos(&self) -> Option<(usize, usize)> {
        match *self {
            CraneStatus::BigEmpty(p) => Some(p),
            CraneStatus::BigLift(p, _) => Some(p),
            CraneStatus::SmallEmpty(p) => Some(p),
            CraneStatus::SmallLift(p, _) => Some(p),
            CraneStatus::Removed => None,
        }
    }

    fn is_empty(&self) -> bool {
        match *self {
            CraneStatus::BigLift(..) | CraneStatus::SmallLift(..) => false,
            _ => true,
        }
    }

    #[allow(dead_code)]
    fn is_big(&self) -> bool {
        match *self {
            CraneStatus::BigEmpty(..) | CraneStatus::BigLift(..) => true,
            _ => false,
        }
    }

    fn is_removed(&self) -> bool {
        *self == CraneStatus::Removed
    }

    fn lifting_cid(&self) -> Option<usize> {
        match *self {
            CraneStatus::BigLift(_, c) | CraneStatus::SmallLift(_, c) => Some(c),
            _ => None,
        }
    }

    fn move_to(&self, pos: (usize, usize)) -> CraneStatus {
        match *self {
            CraneStatus::BigEmpty(_) => CraneStatus::BigEmpty(pos),
            CraneStatus::BigLift(_, c) => CraneStatus::BigLift(pos, c),
            CraneStatus::SmallEmpty(_) => CraneStatus::SmallEmpty(pos),
            CraneStatus::SmallLift(_, c) => CraneStatus::SmallLift(pos, c),
            CraneStatus::Removed => unreachable!(),
        }
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

fn main() {
    input! {
        n: usize,
        ann: [[usize; n]; n],
    }
    let mut rng = SmallRng::from_entropy();
    let random_move = |a: usize| {
        match a % 4 {
            0 => CraneMove::Up,
            1 => CraneMove::Down,
            2 => CraneMove::Left,
            3 => CraneMove::Right,
            // 4 => CraneMove::Wait,
            _ => unreachable!(),
        }
    };
    let mut random_move_array = [
        CraneMove::Up,
        CraneMove::Down,
        CraneMove::Left,
        CraneMove::Right,
    ];

    // 戦略
    // - とりあえず最初は適当に掃き出す
    // - 各クレーン, ターンごとに以下を繰り返す
    //     - 動作が定まっていれば, それに従う
    //         - クレーンが衝突し動けない場合には, 相手が待機中 or 駆動中を見る
    //             - 相手が待機中クレーンであれば, 適当に動かす
    //             - 後ろ二ターン見ても動けない小クレーンは, 荷物をおいて自爆する
    //     - 待機中クレーンであれば, ゴールまで運べるものがあれば掴みにいく
    //         - 優先度: 大クレーンであれば, 自身しか運べない > 距離小, 小さいクレーンは最短距離
    //         - 距離は往復分で考える, が, 復路は再計算する
    //         - ここで経路を一括でつぎ込む
    //     - 待機中クレーンでゴールまで運べるものがなければ, 左端マスを空ける
    //     - いずれも動けなければ, 待機のまま
    // 必要なもの:
    //     - 盤面の状態: Container(id) or Empty
    //     - 次に運ぶコンテナ
    //     - コンテナの状態: Free/Accepted/BeingMoved/Completed
    //     - クレーンの状態: Big/Small と Lift/Empty には座標付き, Removed
    //     - クレーンの予定された行動群: まんま `ans` でよい？でなく, 予定と確定とで分けるべき気がする
    //         - イレギュラー発生時には上書きされ得る, ターン数さほど多くないので間に合いそう
    //         - 拾いに行く/置きに行くで盤面変わる分考えると再計算入れたい, 一括ではない
    //     - クレーンの待機状態: 不要, "予定された今の行動が動く系で, 過去 m ターンの行動がすべて待機であれば", で取れる
    // TODO: 次にほしいやつも準備させた方が良い
    // FIXME: 0090 でたまに動けなくなって force drop する
    //        相手をどかす

    let mut ans = vec![vec![]; CRANE_NUM];
    // 行動予定は処理の都合で逆順に突っ込む
    let mut scheduled_moves = vec![vec![]; CRANE_NUM];
    let mut schedule_decided_turn = vec![0; CRANE_NUM];

    // 盤面管理
    let mut board = vec![vec![vec![BoardStatus::Empty; 5]; 5]; TURN_MAX];
    // 本来ここで定義すべきだが, 今は初手を 4x4 掃き出しで固定しているのでコメントアウト
    // let mut cranes = [
    //     CraneStatus::BigEmpty((0, 0)),
    //     CraneStatus::SmallEmpty((1, 0)),
    //     CraneStatus::SmallEmpty((2, 0)),
    //     CraneStatus::SmallEmpty((3, 0)),
    //     CraneStatus::SmallEmpty((4, 0)),
    // ];
    let mut containers = vec![ContainerStatus::Free; n * n];

    // 進捗管理
    let mut aidx = vec![0; n];
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

    // 最短移動経路をまとめて返す
    let min_move = |_turn_from: usize, move_from: (usize, usize), move_to: (usize, usize)| {
        let mut ret = vec![];

        if move_from.0 > move_to.0 {
            for _ in 0..move_from.0 - move_to.0 {
                ret.push(CraneMove::Up);
            }
        } else if move_from.0 < move_to.0 {
            for _ in 0..move_to.0 - move_from.0 {
                ret.push(CraneMove::Down);
            }
        }

        if move_from.1 > move_to.1 {
            for _ in 0..move_from.1 - move_to.1 {
                ret.push(CraneMove::Left);
            }
        } else if move_from.1 < move_to.1 {
            for _ in 0..move_to.1 - move_from.1 {
                ret.push(CraneMove::Right);
            }
        }

        ret
    };

    let could_move = |crane_id: usize,
                      move_from: (usize, usize),
                      mv: CraneMove,
                      board: &Vec<Vec<BoardStatus>>,
                      cranes: &[CraneStatus]| {
        let Some(my_pos) = cranes[crane_id].pos() else { unreachable!() };
        // 移動できる条件:
        //   - 移動先がグリッド外であると移動不可
        //   - 移動先に大小クレーンがいると移動不可
        //   - 小クレーンであれば, 自身が荷物持ち中かつ移動先に荷物がある場合は移動不可
        let next_pos = match mv {
            CraneMove::Up => (move_from.0.wrapping_add_signed(-1), move_from.1),
            CraneMove::Down => (move_from.0 + 1, move_from.1),
            CraneMove::Left => (move_from.0, move_from.1.wrapping_add_signed(-1)),
            CraneMove::Right => (move_from.0, move_from.1 + 1),
            _ => return true,
        };

        // グリッド外
        if next_pos.0 >= 5 || next_pos.1 >= 5 {
            return false;
        }

        // 他のクレーン
        for (i, _c) in cranes.iter().enumerate() {
            if i == crane_id {
                continue;
            }

            if let Some(other_crane_pos) = cranes[i].pos() {
                if next_pos == other_crane_pos {
                    return false;
                }
            }
        }

        // 小クレーン && 運送中 && 移動先にコンテナ
        if !cranes[crane_id].is_big()
            && cranes[crane_id].lifting_cid() != None
            && board[next_pos.0][next_pos.1] != BoardStatus::Empty
        {
            return false;
        }

        true
    };

    let could_drop = |pos: (usize, usize), board: &Vec<Vec<BoardStatus>>| {
        board[pos.0][pos.1] == BoardStatus::Empty
    };

    let next_pos = |pos_from: (usize, usize), mv: CraneMove| match mv {
        CraneMove::Up => (pos_from.0 - 1, pos_from.1),
        CraneMove::Down => (pos_from.0 + 1, pos_from.1),
        CraneMove::Left => (pos_from.0, pos_from.1 - 1),
        CraneMove::Right => (pos_from.0, pos_from.1 + 1),
        _ => unreachable!(),
    };

    // 小クレーン荷物持ち状態での最短経路をまとめて返す
    // 例えば周囲をコンテナに囲まれていると, 空ベクトルが帰る
    let min_move_small_lift = |crane_id: usize,
                               move_from: (usize, usize),
                               move_to: (usize, usize),
                               board: &Vec<Vec<BoardStatus>>,
                               cranes: &[CraneStatus]| {
        // BFS
        let dir = [
            CraneMove::Up,
            CraneMove::Down,
            CraneMove::Left,
            CraneMove::Right,
        ];
        let mut que = VecDeque::new();
        let n = board.len();
        let mut visited = vec![vec![false; n]; n];
        que.push_back((move_from, vec![]));
        while let Some((cur_pos, vpath)) = que.pop_front() {
            if visited[cur_pos.0][cur_pos.1] {
                continue;
            }

            if cur_pos == move_to {
                return vpath;
            }

            visited[cur_pos.0][cur_pos.1] = true;
            for &d in &dir {
                if !could_move(crane_id, cur_pos, d, board, cranes) {
                    // 移動先に元々自分がいた場合も false になるが, 目的上問題ないので放置
                    continue;
                }

                let np = next_pos(cur_pos, d);
                // debug!("[push] cur_pos: {:?}, d: {:?}, np: {:?}", cur_pos, d, np);
                let mut vv = vpath.clone();
                vv.push(d);
                que.push_back((np, vv));
            }
        }

        vec![]
    };

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
        CraneStatus::BigEmpty((0, 1)),
        CraneStatus::SmallEmpty((1, 1)),
        CraneStatus::SmallEmpty((2, 1)),
        CraneStatus::SmallEmpty((3, 1)),
        CraneStatus::SmallEmpty((4, 1)),
    ];

    let mut turn_cur = init_move.len();
    while turn_cur <= TURN_MAX && goal_want.iter().any(|g| g.is_some()) {
        debug!("\nturn: {turn_cur}");
        turn_cur += 1;
        // 盤面の状態は前回のもの
        for i in 0..n {
            for j in 0..n {
                board[turn_cur][i][j] = board[turn_cur - 1][i][j];
            }
        }

        // 流れてきたものを受け取る
        for i in 0..n {
            if board[turn_cur - 1][i][0] != BoardStatus::Empty
                || cranes
                    .iter()
                    .any(|&c| !c.is_empty() && c.pos().unwrap() == (i, 0))
                || aidx[i] >= n
            {
                // コンテナが存在するので受け取れない (lift 中含め)
                continue;
            }

            board[turn_cur][i][0] = BoardStatus::Container(ann[i][aidx[i]]);
            aidx[i] += 1;
        }

        // 回収されたものを消す
        for i in 0..n {
            // debug!("  board[][{i}][4] = {:?}", board[turn_cur][i][4]);
            if let BoardStatus::Container(c) = board[turn_cur][i][4] {
                goal_want[i] = if c % 5 == 4 {
                    None
                } else {
                    Some(goal_want[i].unwrap() + 1)
                };
                board[turn_cur][i][4] = BoardStatus::Empty;
                // containers[c] = ContainerStatus::Completed;
            }
        }
        debug!("  cranes:");
        for c in &cranes {
            debug!("    {:?}", c);
        }
        debug!("  containers:");
        for (i, c) in containers.iter().enumerate() {
            debug! {"    {i}: {:?}", c}
        }

        debug!("  goal_want: {:?}", goal_want);
        for i in 0..n {
            debug!("  {:?}", board[turn_cur][i]);
        }

        // クレーンを動かす
        // FIXME: Accepted が勝手にクリアされる
        for i in 0..CRANE_NUM {
            debug!("  ### crane {i}");
            if cranes[i].is_removed() {
                continue;
            }

            if i != 0 && i != 3 {
                // めんどいので小クレーンは最初に爆破する
                // TODO: 悪効率
                scheduled_moves[i].push(CraneMove::Remove);
            }

            let my_pos = cranes[i].pos().unwrap();
            debug!("  crane[{i}], scheduled_moves: {:?}", scheduled_moves[i]);

            if scheduled_moves[i].is_empty() {
                // 予定された動きがない場合, ここで動けるか再判定を入れる
                // TODO: クレーン大小で処理を分ける？最短経路だけなら変わらんが
                // (対象, 行動群)
                let mut candidates = vec![];
                if cranes[i].is_empty() {
                    // なにももっていないので拾いに行く
                    for gw_o in &goal_want {
                        // ゴールに持っていけるものがあれば優先する
                        let Some(cid_gw) = gw_o else { continue };

                        if containers[*cid_gw] != ContainerStatus::Free {
                            continue;
                        }

                        // FIXME: なぜか同じターンに同じものを狙いに行く
                        // でもここではないみたい
                        debug!(
                            "  candidate: containers[{cid_gw}], {:?}",
                            containers[*cid_gw]
                        );
                        for ii in 0..n {
                            for jj in 0..n {
                                if board[turn_cur][ii][jj] == BoardStatus::Container(*cid_gw) {
                                    let mm = min_move(i, my_pos, (ii, jj));
                                    candidates.push((*cid_gw, mm));
                                }
                            }
                        }

                        candidates.sort_unstable_by(|a, b| a.1.len().cmp(&b.1.len()));
                    }
                    debug!("    foo, {:?}", candidates);

                    if candidates.is_empty() {
                        // ゴールにもっていけるものがない
                        // とりあえず左端 ([*][0]) を開けて新しいコンテナを引き出す
                        for ii in 0..n {
                            let BoardStatus::Container(cid) = board[turn_cur][ii][0] else { continue };

                            if containers[cid] != ContainerStatus::Free {
                                continue;
                            }

                            let mm = min_move(i, my_pos, (ii, 0));
                            candidates.push((cid, mm));
                        }
                    }

                    // ゴール後には空になる
                    // assert!(!candidates.is_empty());
                    if candidates.is_empty() {
                        // FIXME: ここで小クレーンが一生動けなくなる
                        // FIXME: break しちゃうと ans 入らない
                        debug!("    bar random");
                        // scheduled_moves[i].push(random_move(rng.gen::<usize>()));
                        random_move_array.shuffle(&mut rng);
                        for mv in &random_move_array {
                            if could_move(i, my_pos, *mv, &board[turn_cur], &cranes) {
                                debug!("    mv: {:?}", mv);
                                ans[i].push(mv.to_ans());
                                let np = next_pos(my_pos, *mv);
                                cranes[i] = match cranes[i] {
                                    CraneStatus::BigEmpty(_) => CraneStatus::BigEmpty(np),
                                    CraneStatus::SmallEmpty(_) => CraneStatus::SmallEmpty(np),
                                    _ => unreachable!(),
                                };
                                scheduled_moves[i].clear();
                                // TODO: 何狙いかも Crane に持たせたほうがよいかも
                                for j in 0..n * n {
                                    if containers[j] == ContainerStatus::Accepted(i) {
                                        containers[j] = ContainerStatus::Free;
                                    }
                                }
                                debug!("break01");
                                break;
                            }
                        }

                        debug!("continue");
                        continue;
                    }
                    debug!("    {:?}", candidates);
                    if containers[candidates[0].0] != ContainerStatus::Free {
                        ans[i].push(CraneMove::Remove.to_ans());
                        scheduled_moves[i].clear();
                        cranes[i] = CraneStatus::Removed;
                        break;
                    }

                    candidates[0].1.push(CraneMove::Lift);
                    candidates[0].1.reverse();
                    scheduled_moves[i] = candidates[0].1.clone();
                    schedule_decided_turn[i] = turn_cur;
                    // ここで変えないと同じものを複数クレーンが狙いに行ってしまう
                    // FIXME: なぜか同じものを
                    containers[candidates[0].0] = ContainerStatus::Accepted(i);
                    debug!(
                        "  A, containers[{}], {:?}",
                        candidates[0].0, containers[candidates[0].0]
                    );
                } else {
                    // なにかをもっているので置きに行く
                    // TODO: 小クレーンは動ける範囲が限定されるので
                    let cid_lifting = cranes[i].lifting_cid().unwrap();
                    for gw_o in &goal_want {
                        // ゴールできるならゴールへ
                        let Some(cid_gw) = gw_o else { continue; };

                        if *cid_gw != cid_lifting {
                            continue;
                        }

                        if cranes[i].is_big() {
                            let mm = min_move(i, my_pos, (cid_lifting / 5, 4));
                            candidates.push((cid_lifting, mm));
                            break;
                        } else {
                            let mm = min_move_small_lift(
                                i,
                                my_pos,
                                (cid_lifting / 5, 4),
                                &board[turn_cur],
                                &cranes,
                            );
                            if !mm.is_empty() {
                                candidates.push((cid_lifting, mm));
                            }
                            break;
                        }
                    }

                    if candidates.is_empty() {
                        // ゴールに置けないので一時的に適当なところに置く
                        // - 左端を空けるため, [*][0] には置かない
                        // - 右端に置くと失点するので, [*][4] には置かない
                        // TODO: 一時置きの先はゴールに近いほうがよい
                        for ii in 0..n {
                            for jj in 1..n - 1 {
                                if board[turn_cur][ii][jj] == BoardStatus::Empty {
                                    let mm = if cranes[i].is_big() {
                                        min_move(i, my_pos, (ii, jj))
                                    } else {
                                        min_move_small_lift(
                                            i,
                                            my_pos,
                                            (ii, jj),
                                            &board[turn_cur],
                                            &cranes,
                                        )
                                    };
                                    if !mm.is_empty() {
                                        candidates.push((cid_lifting, mm));
                                    }
                                }
                            }
                        }
                    }

                    // FIXME: 初手 %5=0 組がすべ最奥にいる場合に詰む, 確率 1/50000 程度?
                    debug!("  candidates: {:?}", candidates);
                    if candidates.is_empty() {
                        candidates.push((cid_lifting, vec![]));
                    }
                    candidates.sort_unstable_by(|a, b| a.1.len().cmp(&b.1.len()));
                    candidates[0].1.push(CraneMove::Drop);
                    if candidates[0].1.len() == 1 {
                        // 連続して Lift/Drop させないため, 適当に動く
                        debug!("    add random x2");
                        let mv = random_move(rng.gen::<usize>());
                        for _ in 0..2 {
                            candidates[0].1.push(mv);
                        }
                    }
                    candidates[0].1.reverse();
                    scheduled_moves[i] = candidates[0].1.clone();
                    schedule_decided_turn[i] = turn_cur;
                    containers[candidates[0].0] = ContainerStatus::Accepted(i);
                    debug!("  B, containers[{}]", candidates[0].0);
                }
            }

            debug!("    schedule decided: {:?}", scheduled_moves[i]);
            if scheduled_moves[i].is_empty() {
                ans[i].push(CraneMove::Wait.to_ans());
                continue;
            }

            let cur_move = scheduled_moves[i].pop().unwrap();
            debug!("    [{i}] move_ideal: {:?}", cur_move);
            match cur_move {
                CraneMove::Lift => {
                    // FIXME:
                    let BoardStatus::Container(c) = board[turn_cur][my_pos.0][my_pos.1] else { unreachable!() };
                    ans[i].push(cur_move.to_ans());
                    board[turn_cur][my_pos.0][my_pos.1] = BoardStatus::Empty;
                    cranes[i] = match cranes[i] {
                        CraneStatus::BigEmpty(p) => CraneStatus::BigLift(p, c),
                        CraneStatus::SmallEmpty(p) => CraneStatus::SmallLift(p, c),
                        _ => unreachable!(),
                    };
                    containers[c] = ContainerStatus::BeingMoved(i);
                }
                CraneMove::Drop => {
                    let Some(c) = cranes[i].lifting_cid() else { unreachable!() };
                    if could_drop(my_pos, &board[turn_cur]) {
                        debug!("    force drop accepted");
                        ans[i].push(cur_move.to_ans());
                        // board は empty のまま
                        // 回収と目標更新処理はターン頭で処理する,
                        // 同じところに連投されるリスクがあるため (バグがなければないが)
                        cranes[i] = match cranes[i] {
                            CraneStatus::BigLift(p, _c) => CraneStatus::BigEmpty(p),
                            CraneStatus::SmallLift(p, _c) => CraneStatus::SmallEmpty(p),
                            _ => unreachable!(),
                        };
                        board[turn_cur][my_pos.0][my_pos.1] = BoardStatus::Container(c);
                        containers[c] = match my_pos.1 {
                            4 => ContainerStatus::Completed,
                            _ => ContainerStatus::Free,
                        };

                        if !cranes[i].is_big() && my_pos.1 != 4 {
                            debug!("  small crane, not goal");
                        }
                    } else {
                        debug!("    force drop rejected");
                        // 適当に動く
                        scheduled_moves[i].push(CraneMove::Drop);
                        random_move_array.shuffle(&mut rng);
                        for mv in &random_move_array {
                            if could_move(i, my_pos, *mv, &board[turn_cur], &cranes) {
                                ans[i].push(mv.to_ans());
                                let np = next_pos(my_pos, *mv);
                                cranes[i] = match cranes[i] {
                                    CraneStatus::BigEmpty(_) => CraneStatus::BigEmpty(np),
                                    CraneStatus::BigLift(_, c) => CraneStatus::BigLift(np, c),
                                    CraneStatus::SmallEmpty(_) => CraneStatus::SmallEmpty(np),
                                    CraneStatus::SmallLift(_, c) => CraneStatus::SmallLift(np, c),
                                    _ => unreachable!(),
                                };
                                scheduled_moves[i].clear();
                                // TODO: 何狙いかも Crane に持たせたほうがよいかも
                                for j in 0..n * n {
                                    if containers[j] == ContainerStatus::Accepted(i) {
                                        containers[j] = ContainerStatus::Free;
                                    }
                                }
                                break;
                            }
                        }
                    }
                }
                CraneMove::Up | CraneMove::Down | CraneMove::Left | CraneMove::Right => {
                    // 移動できなかった場合
                    // - 大クレーンなら問答無用で待つ
                    // - 小クレーンであれば, 荷物をおろして適当に動く
                    //    - 適当に散らさないと同じものを狙いに行くので
                    if !could_move(i, my_pos, cur_move, &board[turn_cur], &cranes) {
                        debug!("  could not move");
                        if cranes[i].is_big() {
                            ans[i].push(CraneMove::Wait.to_ans());
                            scheduled_moves[i].push(cur_move);
                        } else {
                            if let Some(cid) = cranes[i].lifting_cid() {
                                // 荷物を現在地に下ろす
                                if my_pos.1 == 4 {
                                    // 失点するので適当に動く
                                    random_move_array.shuffle(&mut rng);
                                    for mv in &random_move_array {
                                        if could_move(i, my_pos, *mv, &board[turn_cur], &cranes) {
                                            ans[i].push(mv.to_ans());
                                            let np = next_pos(my_pos, *mv);
                                            cranes[i] = match cranes[i] {
                                                CraneStatus::BigEmpty(_) => {
                                                    CraneStatus::BigEmpty(np)
                                                }
                                                CraneStatus::BigLift(_, c) => {
                                                    CraneStatus::BigLift(np, c)
                                                }
                                                CraneStatus::SmallEmpty(_) => {
                                                    CraneStatus::SmallEmpty(np)
                                                }
                                                CraneStatus::SmallLift(_, c) => {
                                                    CraneStatus::SmallLift(np, c)
                                                }
                                                _ => unreachable!(),
                                            };
                                            scheduled_moves[i].clear();
                                            // TODO: 何狙いかも Crane に持たせたほうがよいかも
                                            for j in 0..n * n {
                                                if containers[j] == ContainerStatus::Accepted(i) {
                                                    containers[j] = ContainerStatus::Free;
                                                }
                                            }
                                            break;
                                        }
                                    }
                                    // どうしても動けない
                                    assert!(scheduled_moves[i].is_empty());
                                } else {
                                    // FIXME: 列 4 だと辛い
                                    debug!("    force drop");
                                    ans[i].push(CraneMove::Drop.to_ans());
                                    cranes[i] = match cranes[i] {
                                        CraneStatus::BigLift(p, _) => CraneStatus::BigEmpty(p),
                                        CraneStatus::SmallLift(p, _) => CraneStatus::SmallEmpty(p),
                                        _ => unreachable!(),
                                    };
                                    board[turn_cur][my_pos.0][my_pos.1] =
                                        BoardStatus::Container(cid);
                                    containers[cid] = ContainerStatus::Free;
                                    scheduled_moves[i].clear();
                                }
                                // dbg
                                // debug!("  schedule remove");
                                // scheduled_moves[i].push(CraneMove::Remove);
                            } else {
                                // 荷物なし
                                // dbg
                                // debug!("  debug remove");
                                // ans[i].push(CraneMove::Wait.to_ans());
                                // scheduled_moves[i].push(CraneMove::Remove);
                                // 動けるところに適当に動く
                                random_move_array.shuffle(&mut rng);
                                for mv in &random_move_array {
                                    if could_move(i, my_pos, *mv, &board[turn_cur], &cranes) {
                                        ans[i].push(mv.to_ans());
                                        let np = next_pos(my_pos, *mv);
                                        cranes[i] = match cranes[i] {
                                            CraneStatus::BigEmpty(_) => CraneStatus::BigEmpty(np),
                                            CraneStatus::SmallEmpty(_) => {
                                                CraneStatus::SmallEmpty(np)
                                            }
                                            _ => unreachable!(),
                                        };
                                        scheduled_moves[i].clear();
                                        // TODO: 何狙いかも Crane に持たせたほうがよいかも
                                        for j in 0..n * n {
                                            if containers[j] == ContainerStatus::Accepted(i) {
                                                containers[j] = ContainerStatus::Free;
                                            }
                                        }
                                        break;
                                    }
                                }
                                // if !move_decided {
                                //     // しゃーなし
                                //     ans[i].push(CraneMove::Remove.to_ans());
                                // }
                            }
                        }
                    } else {
                        // 動ける
                        debug!("    can move");
                        ans[i].push(cur_move.to_ans());
                        let np = next_pos(my_pos, cur_move);
                        cranes[i] = cranes[i].move_to(np);
                    }
                }
                CraneMove::Wait => ans[i].push(CraneMove::Wait.to_ans()),
                CraneMove::Remove => {
                    ans[i].push(CraneMove::Remove.to_ans());
                    cranes[i] = CraneStatus::Removed;
                    // Accepted 状態のままだと動けなくなる
                    for container_id in 0..n * n {
                        if Some(i) == containers[container_id].moved_by() {
                            containers[container_id] = ContainerStatus::Free;
                        }
                    }
                }
            } // match
            if ans[i].len() == turn_cur {
                debug!("    ans: {:?}", ans[i][turn_cur - 1]);
            }
            debug!("    scheduled_moves[{i}]: {:?}", scheduled_moves[i]);
        } // crane
        debug!("  containers:");
        for (i, c) in containers.iter().enumerate() {
            debug! {"    {i}: {:?}", c}
        }
    } // turn

    // 一応
    assert!(containers.iter().all(|&c| c == ContainerStatus::Completed));

    for a in ans {
        println!("{}", a.iter().collect::<String>());
    }
}
