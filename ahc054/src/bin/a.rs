use proconio::marker::Chars;
use proconio::{input, source::line::LineSource};
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand::RngCore;
use std::collections::VecDeque;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};

#[allow(unused_macros)]
macro_rules! debug {
    ($($arg:tt)+) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)+);
        }
    };
}

// 2 s
const TIME_LIMIT_MS: u64 = 1800;
// 対話的動作部分のマージンを取る
// 初期盤面の評価関数があまりよくなく, 時間をかけすぎてもスコアが伸びない.
const TIME_LIMIT_BEFORE_INTERACTIVE_PART_MS: u64 = 1300;

const DXY: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
// 命名の方角: ゴールの位置が Visualizer 感覚でどこにあるか
#[allow(dead_code)]
const DXY_LT: [(isize, isize); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
#[allow(dead_code)]
const DXY_LB: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
#[allow(dead_code)]
const DXY_RT: [(isize, isize); 4] = [(1, 0), (0, -1), (-1, 0), (0, 1)];
#[allow(dead_code)]
const DXY_RB: [(isize, isize); 4] = [(-1, 0), (0, -1), (1, 0), (0, 1)];
#[allow(dead_code)]
const DXY_ALL: [[(isize, isize); 4]; 4] = [DXY_LB, DXY_LT, DXY_RB, DXY_RT];

// ([トレントを置く場所], [トレントを置かない場所])
// - 90 deg 回転を書いたほうが賢い
// - 構造体にすべき気配がする
#[rustfmt::skip]
const WHIRLPOOL_LT: ([(isize, isize); 15], [(isize, isize); 1]) = (
    [
        (-1, 0),
        (0, -3), (0, -1), (0, 1),
        (1, -3), (1, -1), (1, 2),
        (2, -3), (2, 0), (2, 2),
        (3, -2), (3, 2),
        (4, -1), (4, 0), (4, 1),
    ],
    [
        (-1, -1),
    ]
);
#[rustfmt::skip]
const WHIRLPOOL_LB: ([(isize, isize); 15], [(isize, isize); 1]) = (
    [
        (-2, 1), (-2, 2), (-2, 3),
        (-1, 0), (-1, 4),
        (0, -1), (0, 2), (0, 4),
        (1, 0), (1, 1), (1, 4),
        (2, 3),
        (3, 0), (3, 1), (3, 2),
    ],
    [
        (1, -1),
    ]
);
#[rustfmt::skip]
const WHIRLPOOL_RT: ([(isize, isize); 15], [(isize, isize); 1]) = (
    [
        (-3, -2), (-3, -1), (-3, 0),
        (-2, -3),
        (-1, -4), (-1, -1), (-1, 0),
        (0, -4), (0, -2), (0, 1),
        (1, -4), (1, 0),
        (2, -3), (2, -2), (2, -1),
    ],
    [
        (-1, 1),
    ]
);
#[rustfmt::skip]
const WHIRLPOOL_RB: ([(isize, isize); 15], [(isize, isize); 1]) = (
    [
        (-4, -1), (-4, 0), (-4, 1),
        (-3, -2), (-3, 2),
        (-2, -2), (-2, 0), (-2, 3),
        (-1, -2), (-1, 1), (-1, 3),
        (0, -1), (0, 1), (0, 3),
        (1, 0),
    ],
    [
        (1, 1),
    ]
);
const WHIRLPOOL_ALL: [([(isize, isize); 15], [(isize, isize); 1]); 4] =
    [WHIRLPOOL_LT, WHIRLPOOL_LB, WHIRLPOOL_RT, WHIRLPOOL_RB];

#[allow(dead_code)]
fn could_goal(sxy: (usize, usize), gxy: (usize, usize), has_tree: &Vec<Vec<bool>>) -> bool {
    let n = has_tree.len();
    let mut visited = vec![vec![false; n]; n];
    let mut que = VecDeque::new();
    que.push_back(sxy);

    while let Some(cur) = que.pop_front() {
        if visited[cur.0][cur.1] {
            continue;
        }

        visited[cur.0][cur.1] = true;
        for &(dx, dy) in &DXY {
            let nx = cur.0.wrapping_add_signed(dx);
            let ny = cur.1.wrapping_add_signed(dy);
            if nx >= n || ny >= n || has_tree[nx][ny] {
                continue;
            }

            let nxy = (nx, ny);
            if nxy == gxy {
                return true;
            }

            que.push_back(nxy);
        }
    }

    false
}

fn could_goal_all(sxy: (usize, usize), has_tree: &Vec<Vec<bool>>) -> bool {
    let n = has_tree.len();
    let mut visited = vec![vec![false; n]; n];
    let mut que = VecDeque::new();
    que.push_back(sxy);

    while let Some(cur) = que.pop_front() {
        if visited[cur.0][cur.1] {
            continue;
        }

        visited[cur.0][cur.1] = true;
        for &(dx, dy) in &DXY {
            let nx = cur.0.wrapping_add_signed(dx);
            let ny = cur.1.wrapping_add_signed(dy);
            if nx >= n || ny >= n || has_tree[nx][ny] {
                continue;
            }

            let nxy = (nx, ny);
            que.push_back(nxy);
        }
    }

    for i in 0..n {
        for j in 0..n {
            if !has_tree[i][j] && !visited[i][j] {
                return false;
            }
        }
    }

    true
}

/// 到達可能なマスにのみ `true` を入れたものを返す.
fn could_goal_each(sxy: (usize, usize), has_tree: &Vec<Vec<bool>>) -> Vec<Vec<bool>> {
    let n = has_tree.len();
    let mut visited = vec![vec![false; n]; n];
    let mut que = VecDeque::new();
    que.push_back(sxy);

    while let Some(cur) = que.pop_front() {
        if visited[cur.0][cur.1] {
            continue;
        }

        visited[cur.0][cur.1] = true;
        for &(dx, dy) in &DXY {
            let nx = cur.0.wrapping_add_signed(dx);
            let ny = cur.1.wrapping_add_signed(dy);
            if nx >= n || ny >= n || has_tree[nx][ny] {
                continue;
            }

            let nxy = (nx, ny);
            que.push_back(nxy);
        }
    }

    visited
}

/// 二点間の最短経路となる経路を一つ返す. 始点と終点も経路に含む.
fn shortest_path_2cells(
    sxy: (usize, usize),
    gxy: (usize, usize),
    has_tree: &Vec<Vec<bool>>,
) -> Vec<(usize, usize)> {
    let n = has_tree.len();

    // 経路もたせながらの BFS は重い, BFS の経路復元なかったっけか
    //       => どこから来たか, の直前一マスだけを覚えておいて最後に復元する
    let mut visited = vec![vec![false; n]; n];
    let mut comes_from = vec![vec![None; n]; n];
    let mut que = VecDeque::new();
    que.push_back(sxy);

    'bfs_loop: while let Some(cur_xy) = que.pop_front() {
        // visited 判定は queue 格納時
        visited[cur_xy.0][cur_xy.1] = true;

        for &(dx, dy) in &DXY {
            let nx = cur_xy.0.wrapping_add_signed(dx);
            let ny = cur_xy.1.wrapping_add_signed(dy);
            if nx >= n
                || ny >= n
                || has_tree[nx][ny]
                || comes_from[nx][ny].is_some()
                || visited[nx][ny]
            {
                continue;
            }

            let nxy = (nx, ny);
            comes_from[nx][ny] = Some(cur_xy);
            if (nx, ny) == gxy {
                break 'bfs_loop;
            }

            que.push_back(nxy);
        }
    }

    let mut ret = vec![gxy];
    let mut cur_xy = gxy;
    while let Some(prev_xy) = comes_from[cur_xy.0][cur_xy.1] {
        ret.push(prev_xy);
        cur_xy = prev_xy;
    }
    ret.reverse();
    ret
}

#[allow(dead_code)]
fn shortest_paths(sxy: (usize, usize), has_tree: &Vec<Vec<bool>>) -> Vec<Vec<usize>> {
    let unvisited = usize::MAX / 2;
    let n = has_tree.len();
    let mut ret = vec![vec![unvisited; n]; n];
    let mut que = VecDeque::new();
    que.push_back((sxy, 0));

    while let Some((cur_xy, cur_cost)) = que.pop_front() {
        if ret[cur_xy.0][cur_xy.1] != unvisited {
            continue;
        }

        ret[cur_xy.0][cur_xy.1] = cur_cost;

        for &(dx, dy) in &DXY {
            let nx = cur_xy.0.wrapping_add_signed(dx);
            let ny = cur_xy.1.wrapping_add_signed(dy);
            if nx >= n || ny >= n || has_tree[nx][ny] {
                continue;
            }

            let nxy = (nx, ny);
            que.push_back((nxy, cur_cost + 1));
        }
    }

    ret
}

/// 各マスから見えるマスの総数を返す.
/// 見えるマスの数は冒険者に与える情報の量と対応しているだろうという考えによる.
/// のだが, 評価関数に使うとスコアが下がる...
#[allow(dead_code)]
fn visible_cells_num(has_tree: &Vec<Vec<bool>>) -> usize {
    let n = has_tree.len();
    let mut visible_num = vec![vec![0; n]; n];

    for i in 0..n {
        for j in 0..n {
            if has_tree[i][j] {
                continue;
            }

            for &(dx, dy) in &DXY {
                let mut a = 1;
                loop {
                    let cx = i.wrapping_add_signed(a * dx);
                    let cy = j.wrapping_add_signed(a * dy);
                    if cx >= n || cy >= n {
                        break;
                    } else if has_tree[cx][cy] {
                        // 木が見える場合も加算して終わる, 情報は得ているので
                        visible_num[i][j] += 1;
                        break;
                    }

                    visible_num[i][j] += 1;
                    a += 1;
                }
            }
        }
    }

    let mut ret = 0;
    for i in 0..n {
        for j in 0..n {
            ret += visible_num[i][j];
        }
    }

    ret
}

/// 訪問できるセル数を返す.
#[allow(dead_code)]
fn can_visit_cells_num(sxy: (usize, usize), has_tree: &Vec<Vec<bool>>) -> usize {
    let n = has_tree.len();
    let mut visited = vec![vec![false; n]; n];
    let mut que = VecDeque::new();
    que.push_back(sxy);

    while let Some(cur_xy) = que.pop_front() {
        if visited[cur_xy.0][cur_xy.1] {
            continue;
        }

        visited[cur_xy.0][cur_xy.1] = true;

        for &(dx, dy) in &DXY {
            let nx = cur_xy.0.wrapping_add_signed(dx);
            let ny = cur_xy.1.wrapping_add_signed(dy);
            if nx >= n || ny >= n || has_tree[nx][ny] {
                continue;
            }

            let nxy = (nx, ny);
            que.push_back(nxy);
        }
    }

    let mut ret = 0;
    for i in 0..n {
        for j in 0..n {
            if visited[i][j] {
                ret += 1;
            }
        }
    }

    ret
}

/// トレント追加前後での, 訪問できるセル数の変化 (減った数) を返す.
#[allow(dead_code)]
fn cannot_visit_cells_num_after_adding(
    sxy: (usize, usize),
    has_tree: &Vec<Vec<bool>>,
    treant_xy: (usize, usize),
) -> usize {
    // board_score に入れるとあまり効果が見られず
    let can_visit_before = can_visit_cells_num(sxy, has_tree);
    let mut ht = has_tree.clone();
    ht[treant_xy.0][treant_xy.1] = true;
    let can_visit_after = can_visit_cells_num(sxy, &ht);
    can_visit_before - can_visit_after
}

/// 盤面のスコアを良い感じに計算して返す
/// 小さいほうがよいスコア
#[inline(always)]
#[allow(dead_code)]
fn board_score<T>(
    sxy: (usize, usize),
    gxy: (usize, usize),
    has_tree: &Vec<Vec<bool>>,
    _rng: &mut T,
) -> f64
where
    T: RngCore,
{
    let n = has_tree.len();
    let shortest_paths = shortest_paths(sxy, &has_tree);
    // 長いほどよい
    let s2g_len = shortest_paths[gxy.0][gxy.1] as f64;
    // 小さいほどよい
    let goal_hub= cannot_visit_cells_num_after_adding(sxy, has_tree, gxy) as f64;
     -s2g_len * 0.8 + goal_hub * 0.1
}

fn could_add_treant(
    sxy: (usize, usize),
    gxy: (usize, usize),
    is_found: &Vec<Vec<bool>>,
    has_tree: &Vec<Vec<bool>>,
    treant_xy: (usize, usize),
) -> bool {
    let n = has_tree.len();
    let (tx, ty) = treant_xy;
    if tx >= n || ty >= n || is_found[tx][ty] || has_tree[tx][ty] || treant_xy == gxy {
        return false;
    }

    for &(dx, dy) in &WHIRLPOOL_LT.1 {
        let nx = gxy.0.wrapping_add_signed(dx);
        let ny = gxy.1.wrapping_add_signed(dy);
        if nx < n && ny < n && (nx, ny) == treant_xy {
            return false;
        }
    }

    let mut ht = has_tree.clone();
    ht[tx][ty] = true;
    could_goal_all(sxy, &ht)
}

/// トレントを足せるか判定する. 到達不能なセルを作る可能性がある.
fn could_add_treant_harshly(
    sxy: (usize, usize),
    gxy: (usize, usize),
    is_found: &Vec<Vec<bool>>,
    has_tree: &Vec<Vec<bool>>,
    treant_xy: (usize, usize),
) -> bool {
    let n = has_tree.len();
    let (tx, ty) = treant_xy;
    if tx >= n || ty >= n || is_found[tx][ty] || has_tree[tx][ty] || treant_xy == gxy {
        return false;
    }

    let mut ht = has_tree.clone();
    ht[tx][ty] = true;
    could_goal(sxy, gxy, &ht)
}

/// goal を視認されないよう, 三方を囲む
#[allow(dead_code)]
fn add_treants_surrounding_goal(
    sxy: (usize, usize),
    gxy: (usize, usize),
    is_found: &Vec<Vec<bool>>,
    has_tree: &mut Vec<Vec<bool>>,
    ready_treants: &mut Vec<(usize, usize)>,
    dxy: &[(isize, isize)],
) {
    #[allow(unused_variables)]
    let n = has_tree.len();

    for &(dx, dy) in dxy {
        let tx = gxy.0.wrapping_add_signed(dx);
        let ty = gxy.1.wrapping_add_signed(dy);
        if could_add_treant(sxy, gxy, is_found, has_tree, (tx, ty)) {
            ready_treants.push((tx, ty));
            has_tree[tx][ty] = true;
        } else {
            // 囲めなかった部分に対し, 一マス空けて視界を遮る木を立てたい
            let tx = tx.wrapping_add_signed(dx);
            let ty = ty.wrapping_add_signed(dy);
            if could_add_treant(sxy, gxy, is_found, has_tree, (tx, ty)) {
                ready_treants.push((tx, ty));
                has_tree[tx][ty] = true;
            }
        }
    }
}

/// 渦巻状にトレントを配置する
/// これができれば, 花が目的地とならない限りは花の発見を阻止できる
/// 拝借元: https://x.com/dj_maeda3/status/1972604871515017621
///        G の上の T が視認されてないと, 渦巻右下にいる場合に
///        探索優先度 (上下左右) の都合で G マス経由される
fn add_treants_whirlpool(
    sxy: (usize, usize),
    gxy: (usize, usize),
    is_found: &Vec<Vec<bool>>,
    has_tree: &mut Vec<Vec<bool>>,
    ready_treants: &mut Vec<(usize, usize)>,
) {
    for whirlpool in &WHIRLPOOL_ALL {
        let mut rt = ready_treants.clone();
        let mut ht = has_tree.clone();
        let mut passed = true;

        for &(dx, dy) in &whirlpool.0 {
            let nx = gxy.0.wrapping_add_signed(dx);
            let ny = gxy.1.wrapping_add_signed(dy);
            if could_add_treant(sxy, gxy, is_found, &ht, (nx, ny)) {
                rt.push((nx, ny));
                ht[nx][ny] = true;
            } else {
                passed = false;
                break;
            }
        }

        if passed {
            *ready_treants = rt;
            *has_tree = ht;
            // TODO: 採用した渦の形を返さねば, トレントを置けない場所が正しく判定できない
            return;
        }
    }
}

/// 盤面全体に対し, X 状にトレントを配置する
#[allow(dead_code)]
fn add_treants_x(
    sxy: (usize, usize),
    gxy: (usize, usize),
    is_found: &Vec<Vec<bool>>,
    has_tree: &mut Vec<Vec<bool>>,
    ready_treants: &mut Vec<(usize, usize)>,
) {
    const X_DIAG_LEN: usize = 4;
    let n = has_tree.len();

    let mut i = 1;
    while i < n {
        let mut j = 1;
        while j < n {
            // 左上から右下
            for k in 0..X_DIAG_LEN {
                let ni = i + k;
                let nj = j + k;
                if ni != n - 1
                    && nj != n - 1
                    && could_add_treant(sxy, gxy, is_found, has_tree, (ni, nj))
                {
                    ready_treants.push((ni, nj));
                    has_tree[ni][nj] = true;
                }
            }

            // 右上から左下
            for k in 0..X_DIAG_LEN {
                let ni = i + k;
                let nj = j + X_DIAG_LEN - k;
                if ni < n {
                    if ni != n - 1
                        && nj != n - 1
                        && could_add_treant(sxy, gxy, is_found, has_tree, (ni, nj))
                    {
                        ready_treants.push((ni, nj));
                        has_tree[ni][nj] = true;
                    }
                }
            }

            j += X_DIAG_LEN + 1;
        }
        i += X_DIAG_LEN + 1;
    }
}

/// ジグザグな形にトレントを配置するよう試みる
/// 配置は左上を基準としてサイズ 4 固定
#[allow(dead_code)]
fn add_treants_zigzag(
    sxy: (usize, usize),
    gxy: (usize, usize),
    is_found: &Vec<Vec<bool>>,
    has_tree: &mut Vec<Vec<bool>>,
    ready_treants: &mut Vec<(usize, usize)>,
    begin_lt: (usize, usize),
) {
    let n = has_tree.len();

    // "X" を埋める
    // ooXo
    // Xooo
    // oXoX
    // oXoo
    let add_cells_diff = [(0, 2), (1, 0), (2, 1), (2, 3), (3, 1)];

    for (dx, dy) in add_cells_diff {
        let treant_xy = (
            begin_lt.0.wrapping_add_signed(dx),
            begin_lt.1.wrapping_add_signed(dy),
        );
        if could_add_treant(sxy, gxy, is_found, has_tree, treant_xy) {
            ready_treants.push(treant_xy);
            has_tree[treant_xy.0][treant_xy.1] = true;
        }
    }
    return;

    // 外周部でない or 中央の列を含むブロックでない
    // if !(begin_lt.0 != 0 && begin_lt.1 != 0) || begin_lt.1 == n / 4 / 2 * n {
    //     let treant_xy = (
    //         begin_lt.0 + 2,
    //         begin_lt.1 + 1,
    //     );
    //     if could_add_treant(sxy, gxy, is_found, has_tree, treant_xy) {
    //         ready_treants.push(treant_xy);
    //         has_tree[treant_xy.0][treant_xy.1] = true;
    //     }
    // }
}

/// 正方形領域に対し, 固定数の木をランダムに配置する
fn add_treants_square<T>(
    sxy: (usize, usize),
    gxy: (usize, usize),
    is_found: &Vec<Vec<bool>>,
    has_tree: &mut Vec<Vec<bool>>,
    ready_treants: &mut Vec<(usize, usize)>,
    rng: &mut T,
    begin_lt: (usize, usize),
) where
    T: RngCore,
{
    let n = has_tree.len();
    if begin_lt.0 + 1 >= n || begin_lt.1 + 1 >= n {
        return;
    }

    let square_len = if begin_lt.0 + 3 < n && begin_lt.1 + 3 < n {
        4
    } else {
        2
    };
    let want_trees_num = square_len * square_len / 4;

    let mut trees_num = 0;
    for i in 0..square_len {
        for j in 0..square_len {
            if has_tree[begin_lt.0 + i][begin_lt.1 + j] {
                trees_num += 1;
            }
        }
    }

    // TODO: shuffle して順にあたった方が高速では
    let mut loop_count = 0;
    while trees_num < want_trees_num {
        let dx = rng.gen::<usize>() % square_len;
        let dy = rng.gen::<usize>() % square_len;
        let nx = begin_lt.0 + dx;
        let ny = begin_lt.1 + dy;
        if could_add_treant(sxy, gxy, is_found, has_tree, (nx, ny)) {
            ready_treants.push((nx, ny));
            has_tree[nx][ny] = true;
            trees_num += 1;
        }
        loop_count += 1;
        // TODO: 追加できない場合も事前に判別できるのでは
        if loop_count > square_len * square_len + square_len {
            return;
        }
    }
}

/// 訪問順をランダムにして, ゲームを実行した結果のスコアを返す.
/// - ゴールマスの訪問順は, 必ず全マスの中間となるように固定する.
/// - 途中でトレントが追加されることは想定しない.
#[allow(dead_code)]
fn random_play_score<T>(
    sxy: (usize, usize),
    gxy: (usize, usize),
    has_tree: &Vec<Vec<bool>>,
    rng: &mut T,
) -> usize
where
    T: RngCore,
{
    let n = has_tree.len();

    let mut is_found = vec![vec![false; n]; n];
    let mut saw_tree = vec![vec![false; n]; n];
    let mut goal_orders = Vec::with_capacity(n * n);
    for i in 0..n {
        for j in 0..n {
            goal_orders.push((i, j));
        }
    }
    goal_orders.shuffle(rng);

    let mut goal_i = 0;
    let mut cur_goal: Option<(usize, usize)> = None;
    let mut gxy_appeared = false;
    let mut cur_pos = sxy;
    is_found[sxy.0][sxy.1] = true;
    let mut turn = 0;

    while cur_pos != gxy {
        // 確認済みマスへの追加
        for &(dx, dy) in &DXY {
            for i in 1..n {
                let cx = cur_pos.0.wrapping_add_signed(i as isize * dx);
                let cy = cur_pos.1.wrapping_add_signed(i as isize * dy);
                if cx >= n || cy >= n {
                    break;
                } else if has_tree[cx][cy] {
                    saw_tree[cx][cy] = true;
                    is_found[cx][cy] = true;
                    break;
                }

                is_found[cx][cy] = true;
            }
        }

        // 目的地が確認できていればクリア
        if let Some((cgx, cgy)) = cur_goal {
            // gxy との一致は取っても取らなくてもどうせ後で代入される
            if is_found[cgx][cgy] {
                cur_goal = None;
            }
        }

        // 伝説の花が確認済みであれば, 目的地に設定
        if is_found[gxy.0][gxy.1] {
            cur_goal = Some(gxy);
        }

        let could_be_goal = could_goal_each(cur_pos, &saw_tree);
        // 目的地が到達不可であればクリア
        // 全マスに対して到着判定取って, 後から使い回す
        if let Some((cgx, cgy)) = cur_goal {
            if !could_be_goal[cgx][cgy] {
                cur_goal = None;
            }
        }

        // 目的地の設定, ゴール出現位置は操作する
        // HACK: 難読な気がする
        if cur_goal.is_none() {
            // TODO: goal_orders 長くて嫌
            while is_found[goal_orders[goal_i].0][goal_orders[goal_i].1]
                || !could_be_goal[goal_orders[goal_i].0][goal_orders[goal_i].1]
            {
                goal_i += 1;

                if goal_orders[goal_i] == gxy {
                    gxy_appeared = true;
                    goal_i += 1;
                }

                if (gxy_appeared && goal_i >= n * n / 2 + 1)
                    || (!gxy_appeared && goal_i >= n * n / 2)
                {
                    // 正しいゴールの登場順は中間で固定
                    cur_goal = Some(gxy);
                    break;
                }
            }

            if cur_goal.is_none() {
                cur_goal = Some(goal_orders[goal_i]);
            }
        }

        // 目的地到達までの最短経路を算出し, 動く
        // HACK: 目的地変わるか否かで全探索するの遅くないか
        //       少なくとも木が確認できた場合以外には更新不要なはず
        if let Some(cxy) = cur_goal {
            let p = shortest_path_2cells(cur_pos, cxy, &saw_tree);
            cur_pos = p[1];
        }

        turn += 1;
    }

    turn
}

fn main() {
    let start_time = Instant::now();
    let break_time_before_interactive_part =
        Duration::from_millis(TIME_LIMIT_BEFORE_INTERACTIVE_PART_MS);
    let break_time_finally = Duration::from_millis(TIME_LIMIT_MS);

    let stdin = std::io::stdin();
    let mut source = LineSource::new(stdin.lock());

    input! {
        from &mut source,
        n: usize,
        tij: (usize, usize),
        bnn: [Chars; n],
    }

    let mut rng = SmallRng::from_entropy();
    #[allow(unused_assignments)]
    let mut adventurer = (0, n / 2);
    let mut is_found = vec![vec![false; n]; n];
    // 冒険者の初期配置
    is_found[0][n / 2] = true;
    // 伝説の花マスも置けないが, 考えなくともよい？
    let mut default_tree_num = 0;
    let mut has_tree = vec![vec![false; n]; n];
    for i in 0..n {
        for j in 0..n {
            if bnn[i][j] == 'T' {
                default_tree_num += 1;
                has_tree[i][j] = true;
            }
        }
    }
    let mut ready_treants = vec![];

    let mut score_best = f64::MAX;

    // 初期配置の X 状は動的な阻止と合わせると逆効果っぽいのでしない

    let mut tries = 0;
    while start_time.elapsed() < break_time_before_interactive_part {
        tries += 1;

        // トレントの追加/削除をまとめて行った後に, *評価関数* がよくなれば採用する
        let mut rt_cur = ready_treants.clone();
        let mut ht_cur = has_tree.clone();
        add_treants_whirlpool(
            (0, n / 2),
            tij,
            &is_found,
            &mut has_tree,
            &mut ready_treants,
        );
        break;
    }

    // ゴールから方向転換一度で行けるマスをマークする
    // このラインを進まれるとゴールが発見されてしまう
    let mut is_danger_cell = vec![vec![false; n]; n];
    for &(dxa, dya) in &DXY {
        for &(dxb, dyb) in &DXY {
            if (dxa == dxb && dya == dyb) || (dxa == -dxb && dya == -dyb) {
                // 同じ方向か反対方向は見ても意味がないので
                continue;
            }

            for a in 1..n {
                let ax = tij.0.wrapping_add_signed(a as isize * dxa);
                let ay = tij.1.wrapping_add_signed(a as isize * dya);
                if ax >= n || ay >= n || has_tree[ax][ay] {
                    break;
                }

                for b in 0..n {
                    let abx = ax.wrapping_add_signed(b as isize * dxb);
                    let aby = ay.wrapping_add_signed(b as isize * dyb);
                    if abx >= n || aby >= n || has_tree[abx][aby] {
                        break;
                    }

                    is_danger_cell[abx][aby] = true;
                }
            }
        }
    }
    let mut danger_cells = vec![];
    for i in 0..n {
        for j in 0..n {
            if is_danger_cell[i][j] {
                danger_cells.push((i, j));
            }
        }
    }

    let mut turn = 0;
    let mut adventure_moves = vec![];
    loop {
        if start_time.elapsed() >= break_time_finally {
            // TLE 回避
            println!("-1");
            return;
        }

        input! {
            from &mut source,
            pij: (usize, usize),
            n_turn: usize,
            xyn: [(usize, usize); n_turn],
        }
        if turn != 0 {
            adventure_moves.push((
                pij.0 as isize - adventurer.0 as isize,
                pij.1 as isize - adventurer.0 as isize,
            ));
        }
        turn += 1;
        adventurer = pij;
        if adventurer == tij {
            break;
        }

        for (x, y) in xyn {
            is_found[x][y] = true;
        }

        // let goal_diff = (tij.0 as isize - adventurer.0 as isize, tij.1 as isize - adventurer.1 as isize);
        for &(dx, dy) in &DXY {
            // TODO: 2 固定もどうなのか, 斜めに置いたほうがよい
            for i in 1..5 {
                let cx = adventurer.0.wrapping_add_signed(i as isize * dx);
                let cy = adventurer.1.wrapping_add_signed(i as isize * dy);
                if cx >= n || cy >= n || is_found[cx][cy] || has_tree[cx][cy] {
                    break;
                } else if i >= 2 {
                    if could_add_treant(adventurer, tij, &is_found, &has_tree, (cx, cy)) {
                        ready_treants.push((cx, cy));
                        has_tree[cx][cy] = true;
                        break;
                    }
                }
            }
        }

        // 冒険者が次の手で花を視認できるマスに出てきそうな場合, 妨害したい
        // 冒険者と危険マスの距離が 1 であった場合, 冒険者から最も近い危険マスを防ぐ
        for &(dx, dy) in &DXY {
            // 冒険者が次に出現し得る位置
            let nax = adventurer.0.wrapping_add_signed(dx);
            let nay = adventurer.1.wrapping_add_signed(dy);
            if nax >= n || nay >= n || !is_danger_cell[nax][nay] || has_tree[nax][nay] {
                continue;
            }

            // 危険マスを基準に, 花の方向に最も近い未確認マスを塞ぎたい
            // 危険マスからゴールまでの最短経路をもらって, 逆順に防ぐ
            // TODO: 遅い
            let mut na2goal = shortest_path_2cells((nax, nay), tij, &has_tree);
            // 始点は除く
            na2goal.pop();
            na2goal.reverse();
            // ゴールは除く
            na2goal.pop();
            na2goal.reverse();
            for (i, &c) in na2goal.iter().enumerate() {
                if !is_found[c.0][c.1] {
                    // TODO: 到達不可なマスを生み出してでも目先の遠回りを優先した方が賢い？
                    if i < 2 && could_add_treant_harshly((nax, nay), tij, &is_found, &has_tree, c) {
                        ready_treants.push(c);
                        has_tree[c.0][c.1] = true;
                        break;
                    } else if i < n / 4
                        && could_add_treant(adventurer, tij, &is_found, &has_tree, c)
                    {
                        ready_treants.push(c);
                        has_tree[c.0][c.1] = true;
                        break;
                    }
                }
            }

            // TODO: 危険でなくなったマスをマークすべき, 高速化の余地もある
        }

        // 初期以外でうまく使えず…
        print!("{}", ready_treants.len());
        if !ready_treants.is_empty() {
            for &(x, y) in &ready_treants {
                print!(" {x} {y}");
            }
            ready_treants.clear();
        }
        println!();
        stdout().flush().unwrap();
    }
}
