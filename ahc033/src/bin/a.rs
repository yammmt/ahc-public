// 異常なほどの重実装になっているのだがどうして

use proconio::input;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            println!($($arg)*);
        }
    };
}

// N 固定だから vec を回避すればちょっとだけ高速化できる
const GRID_SIZE: usize = 5;
const CRANE_NUM: usize = 5;
const CONTAINER_NUM: usize = 25;
// サンプルケース見る限りでは, 答えは最大でも 230 かそこらには収まる
const TURN_MAX: usize = 350;
// TODO: 提出時は伸ばそう
const RUN_TIME_MAX_MS: u64 = if cfg!(debug_assertions) { 500 } else { 2970 };
// const RUN_TIME_MAX_MS: u64 = if cfg!(debug_assertions) { 500 } else { 1000 };

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
    let start_time = Instant::now();
    let break_time = Duration::from_millis(RUN_TIME_MAX_MS);

    input! {
        _n: usize,
        ann: [[usize; GRID_SIZE]; GRID_SIZE],
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

    // 乱択を時間いっぱい繰り返すであればこのくらいの発生率のバグは消さなくて良いよね

    let mut ans_final: Vec<Vec<char>> = vec![vec![]; CRANE_NUM];

    // 一気に全部吐き出すと小クレーンの経路が大きく制限されてしまう
    // 初期に吐き出すパスを偶数 or 奇数行にすれば, 必ず 0 を引ける？
    // 一ターンずつ操作を決定するより, 一つのクレーンを決める => 余った経路でうまく残りのクレーンを遡って動かす,
    // とした方がトータルでは賢いみたい

    // 移動
    let next_pos = |pos_from: (usize, usize), mv: CraneMove| match mv {
        CraneMove::Up => (pos_from.0 - 1, pos_from.1),
        CraneMove::Down => (pos_from.0 + 1, pos_from.1),
        CraneMove::Left => (pos_from.0, pos_from.1 - 1),
        CraneMove::Right => (pos_from.0, pos_from.1 + 1),
        _ => unreachable!(),
    };

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
                      cranes_now: &[CraneStatus],
                      cranes_prev: &[CraneStatus]| {
        // 移動できる条件:
        //   - 移動先がグリッド外であると移動不可
        //   - 移動先に大小クレーンがいると移動不可
        //   - すれ違う動きになる場合は移動不可
        //   - 小クレーンであれば, 自身が荷物持ち中かつ移動先に荷物がある場合は移動不可
        let np = match mv {
            CraneMove::Up => (move_from.0.wrapping_add_signed(-1), move_from.1),
            CraneMove::Down => (move_from.0 + 1, move_from.1),
            CraneMove::Left => (move_from.0, move_from.1.wrapping_add_signed(-1)),
            CraneMove::Right => (move_from.0, move_from.1 + 1),
            _ => return true,
        };

        // グリッド外
        if np.0 >= GRID_SIZE || np.1 >= GRID_SIZE {
            return false;
        }

        // 他のクレーン
        for i in 0..CRANE_NUM {
            if i == crane_id {
                continue;
            }

            if let Some(other_crane_pos) = cranes_now[i].pos() {
                if np == other_crane_pos {
                    return false;
                }
            }
            // すれ違い: 自身の移動先 == 相手の過去位置 && 自身の過去位置 == 相手の現在地
            if let Some(other_crane_pos_prev) = cranes_prev[i].pos() {
                if np == other_crane_pos_prev && Some(move_from) == cranes_now[i].pos() {
                    return false;
                }
            }
        }

        // 小クレーン && 運送中 && 移動先にコンテナ
        if !cranes_now[crane_id].is_big()
            && cranes_now[crane_id].lifting_cid() != None
            && board[np.0][np.1] != BoardStatus::Empty
        {
            return false;
        }

        true
    };

    let could_drop = |pos: (usize, usize), board: &Vec<Vec<BoardStatus>>| {
        board[pos.0][pos.1] == BoardStatus::Empty
    };

    // 小クレーン荷物持ち状態での最短経路をまとめて返す
    // 例えば周囲をコンテナに囲まれていると, 空ベクトルが帰る
    let min_move_small_lift = |crane_id: usize,
                               move_from: (usize, usize),
                               move_to: (usize, usize),
                               board: &Vec<Vec<BoardStatus>>,
                               cranes: &[CraneStatus],
                               cranes_prev: &[CraneStatus]| {
        // BFS
        let dir = [
            CraneMove::Up,
            CraneMove::Down,
            CraneMove::Left,
            CraneMove::Right,
        ];
        let mut que = VecDeque::new();
        let mut visited = vec![vec![false; GRID_SIZE]; GRID_SIZE];
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
                if !could_move(crane_id, cur_pos, d, board, cranes, cranes_prev) {
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

    let mut do_random_move = |my_crane_id: usize,
                              my_pos: (usize, usize),
                              ans: &mut Vec<char>,
                              scheduled_moves: &mut Vec<CraneMove>,
                              cranes: &mut [CraneStatus],
                              cranes_prev: &[CraneStatus],
                              board: &mut Vec<Vec<BoardStatus>>,
                              containers: &mut [ContainerStatus],
                              rng: &mut SmallRng| {
        random_move_array.shuffle(rng);
        for mv in &random_move_array {
            if could_move(my_crane_id, my_pos, *mv, board, cranes, cranes_prev) {
                debug!("  decided random move: {:?}", mv);
                ans.push(mv.to_ans());
                let np = next_pos(my_pos, *mv);
                cranes[my_crane_id] = match cranes[my_crane_id] {
                    CraneStatus::BigEmpty(_) => CraneStatus::BigEmpty(np),
                    CraneStatus::BigLift(_, c) => CraneStatus::BigLift(np, c),
                    CraneStatus::SmallEmpty(_) => CraneStatus::SmallEmpty(np),
                    CraneStatus::SmallLift(_, c) => CraneStatus::SmallLift(np, c),
                    _ => unreachable!(),
                };
                scheduled_moves.clear();
                for j in 0..CONTAINER_NUM {
                    if containers[j] == ContainerStatus::Accepted(my_crane_id) {
                        containers[j] = ContainerStatus::Free;
                    }
                }
                break;
            }
        }
    };

    // ゴールまでの距離が変化することを考慮した, パスの評価値を返す
    // 通行不能状態は考慮しない
    let path_cost = |
        container_id: usize,
        move_from: (usize, usize),
        vpath: &[CraneMove]
    | {
        let goal_i = container_id % 5;
        let mut pos = move_from;
        for &mv in vpath {
            pos = next_pos(pos, mv);
        }
        let diff_i = goal_i.max(pos.0) - goal_i.min(pos.0);
        let diff_j = 4 - pos.1;
        vpath.len() + diff_i + diff_j
    };

    while start_time.elapsed() < break_time {
        let mut ans = vec![vec![]; CRANE_NUM];
        // 行動予定は処理の都合で逆順に突っ込む
        let mut scheduled_moves = vec![vec![]; CRANE_NUM];
        let mut schedule_decided_turn = vec![0; CRANE_NUM];

        // 盤面管理
        let mut board = vec![vec![vec![BoardStatus::Empty; GRID_SIZE]; GRID_SIZE]; TURN_MAX];
        let mut cranes = [
            CraneStatus::BigEmpty((0, 0)),
            CraneStatus::SmallEmpty((1, 0)),
            CraneStatus::SmallEmpty((2, 0)),
            CraneStatus::SmallEmpty((3, 0)),
            CraneStatus::SmallEmpty((4, 0)),
        ];
        let mut containers = vec![ContainerStatus::Free; CONTAINER_NUM];

        // 進捗管理
        let mut aidx = vec![0; GRID_SIZE];
        let mut goal_want = [Some(0), Some(5), Some(10), Some(15), Some(20)];

        // 初手は三列目まで全行掃き出す or 同じく偶数行目だけ掃き出すの二択
        // 後者は小クレーンが動き易い盤面
        // 前者だけ or 後者だけより, 両方混ぜた方が手元で見る限りはよいスコアだったので
        let mut init_move = vec![];
        if rng.gen::<usize>() % 2 == 0 {
            // 三列目まで全部出す
            init_move = "PRRRQLLLPRRQLLPRQ".chars().collect::<Vec<char>>();
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
            cranes = [
                CraneStatus::BigEmpty((0, 1)),
                CraneStatus::SmallEmpty((1, 1)),
                CraneStatus::SmallEmpty((2, 1)),
                CraneStatus::SmallEmpty((3, 1)),
                CraneStatus::SmallEmpty((4, 1)),
            ];
        } else {
            // E 字をスケジュールだけ
            for i in 0..CRANE_NUM {
                if i % 2 == 0 {
                    let mut mv = vec![
                        CraneMove::Lift,
                        CraneMove::Right,
                        CraneMove::Right,
                        CraneMove::Right,
                        CraneMove::Drop,
                        CraneMove::Left,
                        CraneMove::Left,
                        CraneMove::Left,
                        CraneMove::Lift,
                        CraneMove::Right,
                        CraneMove::Right,
                        CraneMove::Drop,
                        CraneMove::Left,
                        CraneMove::Left,
                        CraneMove::Lift,
                    ];
                    mv.reverse();
                    scheduled_moves[i].append(&mut mv);
                }
            }
        }

        // 大クレーンは爆破しない
        // 小クレーンをある程度使ったほうがスコアが上がりそうだから,
        // 必ず三つ以上を使うようにする
        let mut is_removed_first = match rng.gen::<usize>() % 3 {
            0 => vec![false, false, true, true],
            1 => vec![false, false, false, true],
            2 => vec![false, false, false, false],
            _ => unreachable!(),
        };
        is_removed_first.shuffle(&mut rng);
        is_removed_first.push(false);
        is_removed_first.reverse();

        let mut turn_cur = init_move.len();
        'turn_loop: while turn_cur < TURN_MAX - 1 && goal_want.iter().any(|g| g.is_some()) {
            debug!("\nturn: {turn_cur}");
            turn_cur += 1;
            // 盤面の状態は前回のもの
            for i in 0..GRID_SIZE {
                for j in 0..GRID_SIZE {
                    board[turn_cur][i][j] = board[turn_cur - 1][i][j];
                }
            }

            // 流れてきたものを受け取る
            for i in 0..GRID_SIZE {
                // "5" 埋め込みだが実害なく *短い* 定義名が浮かばなかったので放置
                if board[turn_cur - 1][i][0] != BoardStatus::Empty
                    || cranes
                        .iter()
                        .any(|&c| !c.is_empty() && c.pos().unwrap() == (i, 0))
                    || aidx[i] >= 5
                {
                    // コンテナが存在するので受け取れない (lift 中含め)
                    continue;
                }

                board[turn_cur][i][0] = BoardStatus::Container(ann[i][aidx[i]]);
                aidx[i] += 1;
            }

            // 回収されたものを消す
            for i in 0..GRID_SIZE {
                // debug!("  board[][{i}][4] = {:?}", board[turn_cur][i][4]);
                if let BoardStatus::Container(c) = board[turn_cur][i][4] {
                    goal_want[i] = if c % 5 == 4 {
                        None
                    } else {
                        // 意にそぐわぬものがきたら, 探索失敗として今のループを諦める
                        if goal_want[i].is_none() {
                            // 最終ターンに選択を誤った場合には, ここで break しても
                            // ゴール判定が通り, スコアが 10,000 点悪化してしまう.
                            // これの対策として, ゴールを無効にする.
                            // 本来はすべてのゴールを操作すべきだが, 高速化のため割愛
                            containers[0] = ContainerStatus::Free;
                            break 'turn_loop;
                        } else {
                            let gw = goal_want[i].unwrap();
                            if c != gw {
                                break 'turn_loop;
                            }

                            Some(gw + 1)
                        }
                    };
                    board[turn_cur][i][4] = BoardStatus::Empty;
                }
            }
            if containers.iter().all(|&c| c == ContainerStatus::Completed) {
                break;
            }

            // 移動前のクレーン状態を控える
            let cranes_prev = cranes.clone();

            debug!("  cranes:");
            for c in &cranes {
                debug!("    {:?}", c);
            }
            debug!("  containers:");
            for (i, c) in containers.iter().enumerate() {
                debug! {"    {i}: {:?}", c}
            }

            debug!("  goal_want: {:?}", goal_want);
            for i in 0..GRID_SIZE {
                debug!("  {:?}", board[turn_cur][i]);
            }

            // クレーンを動かす
            for i in 0..CRANE_NUM {
                debug!("  ### crane {i}");
                if cranes[i].is_removed() {
                    continue;
                }

                // assert_eq!(ans[i].len(), turn_cur - 1);
                // TODO: 暫定処置
                while ans[i].len() < turn_cur - 1 {
                    debug!("    [ERROR] invalid ans length");
                    ans[i].push(CraneMove::Wait.to_ans());
                }

                if is_removed_first[i] {
                    // TODO: 最初に爆破するのではなく後で爆破したほうがよい可能性がある
                    // 後半使い道のなくなったやつとか
                    scheduled_moves[i].push(CraneMove::Remove);
                }

                let my_pos = cranes[i].pos().unwrap();
                debug!("  crane[{i}], scheduled_moves: {:?}", scheduled_moves[i]);

                if scheduled_moves[i].is_empty() {
                    // 予定された動きがない場合, ここで動けるか再判定を入れる
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

                            debug!(
                                "  candidate: containers[{cid_gw}], {:?}",
                                containers[*cid_gw]
                            );
                            for ii in 0..GRID_SIZE {
                                for jj in 0..GRID_SIZE {
                                    if board[turn_cur][ii][jj] == BoardStatus::Container(*cid_gw) {
                                        let mm = min_move(i, my_pos, (ii, jj));
                                        candidates.push((*cid_gw, mm));
                                    }
                                }
                            }
                        }
                        debug!("    foo, {:?}", candidates);

                        if candidates.is_empty() {
                            // ゴールにもっていけるものがない
                            // とりあえず左端 ([*][0]) を開けて新しいコンテナを引き出す
                            for ii in 0..GRID_SIZE {
                                let BoardStatus::Container(cid) = board[turn_cur][ii][0] else { continue };

                                if containers[cid] != ContainerStatus::Free {
                                    continue;
                                }

                                let mm = min_move(i, my_pos, (ii, 0));
                                candidates.push((cid, mm));
                            }
                        }

                        if candidates.is_empty() {
                            // 左端をどかせるわけでもゴールにもっていけるものがあるわけでもない
                            // 次に必要となるものを拾いに行ってみる
                            for gw_o in &goal_want {
                                let Some(cid_gw) = gw_o else { continue };

                                let cid_gw_next = cid_gw + 1;
                                if cid_gw_next % 5 == 0 {
                                    // ゴールにもっていけるものが残り一つだから黙る
                                    continue;
                                }

                                if containers[cid_gw_next] != ContainerStatus::Free {
                                    continue;
                                }

                                for ii in 0..GRID_SIZE {
                                    for jj in 0..GRID_SIZE {
                                        if board[turn_cur][ii][jj]
                                            == BoardStatus::Container(*cid_gw)
                                        {
                                            let mm = min_move(i, my_pos, (ii, jj));
                                            candidates.push((*cid_gw, mm));
                                        }
                                    }
                                }
                            }
                        }

                        // ゴール後には空になる
                        if candidates.is_empty() {
                            // FIXME: 中途半端な `continue` で動けなくなる？
                            debug!("    bar random");
                            do_random_move(
                                i,
                                my_pos,
                                &mut ans[i],
                                &mut scheduled_moves[i],
                                &mut cranes,
                                &cranes_prev,
                                &mut board[turn_cur],
                                &mut containers,
                                &mut rng,
                            );
                            debug!("continue");
                            continue;
                        }

                        candidates.sort_unstable_by(|a, b| a.1.len().cmp(&b.1.len()));
                        debug!("    {:?}", candidates);
                        if containers[candidates[0].0] != ContainerStatus::Free {
                            if cranes[i].is_big() {
                                ans[i].push(CraneMove::Wait.to_ans());
                                scheduled_moves[i].clear();
                            } else {
                                ans[i].push(CraneMove::Remove.to_ans());
                                scheduled_moves[i].clear();
                                cranes[i] = CraneStatus::Removed;
                            }
                            break;
                        }

                        candidates[0].1.push(CraneMove::Lift);
                        candidates[0].1.reverse();
                        scheduled_moves[i] = candidates[0].1.clone();
                        schedule_decided_turn[i] = turn_cur;
                        // ここで変えないと同じものを複数クレーンが狙いに行ってしまう
                        containers[candidates[0].0] = ContainerStatus::Accepted(i);
                        debug!(
                            "  A, containers[{}], {:?}",
                            candidates[0].0, containers[candidates[0].0]
                        );
                    } else {
                        // なにかをもっているので置きに行く
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
                                    &cranes_prev,
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
                            // "適当なところ" の評価値は sort 時に考慮する
                            for ii in 0..GRID_SIZE {
                                for jj in 0..GRID_SIZE - 1 {
                                    if jj == 0 && aidx[i] < GRID_SIZE {
                                        // 出切っていなければ置かない
                                        continue;
                                    }

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
                                                &cranes_prev,
                                            )
                                        };
                                        if !mm.is_empty() {
                                            candidates.push((cid_lifting, mm));
                                        }
                                    }
                                }
                            }
                        }

                        debug!("  candidates: {:?}", candidates);
                        if candidates.is_empty() {
                            candidates.push((cid_lifting, vec![]));
                        }
                        // 単純な手数に加え, 移動長に移動先がゴールから遠くなる分の評価を乗せる
                        candidates.sort_unstable_by(|a, b| {
                            let a_cost = path_cost(a.0, my_pos, &a.1);
                            let b_cost = path_cost(b.0, my_pos, &b.1);
                            a_cost.cmp(&b_cost)
                        });
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
                        let BoardStatus::Container(c) = board[turn_cur][my_pos.0][my_pos.1] else { break 'turn_loop };
                        ans[i].push(cur_move.to_ans());
                        board[turn_cur][my_pos.0][my_pos.1] = BoardStatus::Empty;
                        cranes[i] = match cranes[i] {
                            CraneStatus::BigEmpty(p) => CraneStatus::BigLift(p, c),
                            CraneStatus::SmallEmpty(p) => CraneStatus::SmallLift(p, c),
                            _ => break 'turn_loop,
                        };
                        containers[c] = ContainerStatus::BeingMoved(i);
                    }
                    CraneMove::Drop => {
                        let Some(c) = cranes[i].lifting_cid() else { break 'turn_loop };
                        if could_drop(my_pos, &board[turn_cur]) {
                            debug!("    force drop accepted");
                            ans[i].push(cur_move.to_ans());
                            // board は empty のまま
                            // 回収と目標更新処理はターン頭で処理する,
                            // 同じところに連投されるリスクがあるため (バグがなければないが)
                            cranes[i] = match cranes[i] {
                                CraneStatus::BigLift(p, _c) => CraneStatus::BigEmpty(p),
                                CraneStatus::SmallLift(p, _c) => CraneStatus::SmallEmpty(p),
                                _ => break 'turn_loop,
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
                            do_random_move(
                                i,
                                my_pos,
                                &mut ans[i],
                                &mut scheduled_moves[i],
                                &mut cranes,
                                &cranes_prev,
                                &mut board[turn_cur],
                                &mut containers,
                                &mut rng,
                            );
                            scheduled_moves[i].push(CraneMove::Drop);
                        }
                    }
                    CraneMove::Up | CraneMove::Down | CraneMove::Left | CraneMove::Right => {
                        // 移動できなかった場合
                        // - 大クレーンなら問答無用で待つ
                        // - 小クレーンであれば, 荷物をおろして適当に動く
                        //    - 適当に散らさないと同じものを狙いに行くので
                        //    - 動けなければ待つ
                        if !could_move(i, my_pos, cur_move, &board[turn_cur], &cranes, &cranes_prev)
                        {
                            debug!("  could not move");
                            if cranes[i].is_big() {
                                ans[i].push(CraneMove::Wait.to_ans());
                                scheduled_moves[i].push(cur_move);
                            } else if let Some(cid) = cranes[i].lifting_cid() {
                                // 荷物を現在地に下ろす
                                if my_pos.1 == 4 {
                                    // 失点するので適当に動く
                                    do_random_move(
                                        i,
                                        my_pos,
                                        &mut ans[i],
                                        &mut scheduled_moves[i],
                                        &mut cranes,
                                        &cranes_prev,
                                        &mut board[turn_cur],
                                        &mut containers,
                                        &mut rng,
                                    );
                                    // どうしても動けない
                                    if scheduled_moves[i].is_empty() {
                                        break 'turn_loop;
                                    }
                                } else {
                                    // FIXME: 列 4 だと辛い
                                    debug!("    force drop");
                                    ans[i].push(CraneMove::Drop.to_ans());
                                    cranes[i] = match cranes[i] {
                                        CraneStatus::BigLift(p, _) => CraneStatus::BigEmpty(p),
                                        CraneStatus::SmallLift(p, _) => CraneStatus::SmallEmpty(p),
                                        _ => break 'turn_loop,
                                    };
                                    board[turn_cur][my_pos.0][my_pos.1] =
                                        BoardStatus::Container(cid);
                                    containers[cid] = ContainerStatus::Free;
                                    scheduled_moves[i].clear();
                                }
                                // debug!("  schedule remove");
                            } else {
                                // 荷物なし
                                // debug!("  debug remove");
                                // 動けるところに適当に動く
                                do_random_move(
                                    i,
                                    my_pos,
                                    &mut ans[i],
                                    &mut scheduled_moves[i],
                                    &mut cranes,
                                    &cranes_prev,
                                    &mut board[turn_cur],
                                    &mut containers,
                                    &mut rng,
                                );
                                if ans[i].len() < turn_cur {
                                    // 動ける方法がなかった
                                    ans[i].push(CraneMove::Wait.to_ans());
                                }
                            }
                        } else {
                            // 動ける
                            debug!("    can move: {:?}", cur_move);
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
                        for container_id in 0..CONTAINER_NUM {
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

        // 探索失敗時は破棄
        let mut is_valid_ans = true;
        for &c in &containers {
            if c != ContainerStatus::Completed {
                is_valid_ans = false;
                break;
            }
        }
        if !is_valid_ans {
            continue;
        }

        let max_ans_len = ans.iter().map(|a| a.len()).max().unwrap();
        let max_ans_final_len = ans_final.iter().map(|a| a.len()).max().unwrap_or(0);
        if max_ans_final_len == 0 || max_ans_len < max_ans_final_len {
            for i in 0..CRANE_NUM {
                ans_final[i] = ans[i].clone();
            }
        }
    } // loop

    if ans_final[0].is_empty() {
        // 答えが見つからなかった場合 (最奥に %5=0) は, 左から右に受け流すだけにする
        // エラー回避分くらいの点数はもらえる
        for _ in 0..CRANE_NUM {
            println!("PRRRRQLLLLPRRRRQLLLLPRRRRQLLLLPRRRRQLLLLPRRRRQ");
        }
    } else {
        for a in ans_final {
            println!("{}", a.iter().collect::<String>());
        }
    }
}
