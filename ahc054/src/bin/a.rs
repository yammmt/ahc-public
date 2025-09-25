use proconio::marker::Chars;
use proconio::{input, source::line::LineSource};
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand::RngCore;
use std::collections::VecDeque;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};

// 2 s
const TIME_LIMIT_MS: u64 = 1900;

const DXY: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];
// 命名の方角: ゴールの位置が Visualizer 感覚でどこにあるか
const DXY_LT: [(isize, isize); 4] = [(1, 0), (0, 1), (-1, 0), (0, -1)];
const DXY_LB: [(isize, isize); 4] = [(-1, 0), (0, 1), (1, 0), (0, -1)];
const DXY_RT: [(isize, isize); 4] = [(1, 0), (0, -1), (-1, 0), (0, 1)];
const DXY_RB: [(isize, isize); 4] = [(-1, 0), (0, -1), (1, 0), (0, 1)];

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

    // TODO: 経路もたせながらの BFS は重い, BFS の経路復元なかったっけか
    let mut visited = vec![vec![false; n]; n];
    let mut que = VecDeque::new();
    que.push_back((sxy, vec![]));

    while let Some((cur_xy, cur_path)) = que.pop_front() {
        if visited[cur_xy.0][cur_xy.1] {
            continue;
        }

        visited[cur_xy.0][cur_xy.1] = true;
        let mut next_path = cur_path.clone();
        next_path.push(cur_xy);
        if cur_xy == gxy {
            return next_path;
        }

        for &(dx, dy) in &DXY {
            let nx = cur_xy.0.wrapping_add_signed(dx);
            let ny = cur_xy.1.wrapping_add_signed(dy);
            if nx >= n || ny >= n || has_tree[nx][ny] {
                continue;
            }

            let nxy = (nx, ny);
            que.push_back((nxy, next_path.clone()));
        }
    }

    unreachable!()
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

/// 盤面のスコアを良い感じに計算して返す
/// 小さいほうがよいスコア
#[inline(always)]
fn board_score<T>(
    sxy: (usize, usize),
    gxy: (usize, usize),
    has_tree: &Vec<Vec<bool>>,
    rng: &mut T,
) -> f64
where
    T: RngCore,
{
    let mut tries = [0, 0, 0, 0, 0];
    for i in 0..tries.len() {
        tries[i] = random_play_score(sxy, gxy, has_tree, rng);
    }
    tries.sort_unstable();

    -(tries[tries.len() / 2] as f64)
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

    let mut ht = has_tree.clone();
    ht[tx][ty] = true;
    could_goal_all(sxy, &ht)
}

/// goal を視認されないよう, 三方を囲む
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

/// 盤面全体に対し, X 状にトレントを配置する
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

/// 訪問順をランダムにして, ゲームを実行した結果のスコアを返す.
/// - ゴールマスの訪問順は, 必ず全マスの中間となるように固定する.
/// - 途中でトレントが追加されることは想定しない.
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
    let break_time = Duration::from_millis(TIME_LIMIT_MS);

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
    let mut has_tree = vec![vec![false; n]; n];
    for i in 0..n {
        for j in 0..n {
            has_tree[i][j] = bnn[i][j] == 'T';
        }
    }
    let mut ready_treants = vec![];

    let mut score_best = f64::MAX;
    let mut ht_best = has_tree.clone();
    let mut rt_best = ready_treants.clone();

    while start_time.elapsed() < break_time {
        // ゴールの三方に視界を防ぐためのトレントを立て,
        // "X" の形に適当にトレントを立て,
        // トレントの追加/削除をまとめて行った後に,
        // *評価関数* がよくなれば採用する
        for &dxy in &[DXY_LB, DXY_LT, DXY_RB, DXY_RT] {
            let mut ht_cur = has_tree.clone();
            let mut rt_cur = ready_treants.clone();
            add_treants_surrounding_goal(
                (0, n / 2),
                tij,
                &is_found,
                &mut ht_cur,
                &mut rt_cur,
                &dxy,
            );
            // X の形にトレントを置く
            add_treants_x((0, n / 2), tij, &is_found, &mut ht_cur, &mut rt_cur);

            // 雑乱択
            for _ in 0..n / 2 {
                let treant_x = rng.gen::<usize>() % n;
                let treant_y = rng.gen::<usize>() % n;
                if ht_cur[treant_x][treant_y] && bnn[treant_x][treant_y] != 'T' {
                    // 削除を試みる
                    // TODO: 遅い
                    let mut rm_i = 0;
                    for i in 0..rt_cur.len() {
                        if rt_cur[i] == (treant_x, treant_y) {
                            rm_i = i;
                            break;
                        }
                    }
                    rt_cur.remove(rm_i);
                    ht_cur[treant_x][treant_y] = false;
                } else if could_add_treant(
                    adventurer,
                    tij,
                    &is_found,
                    &ht_cur,
                    (treant_x, treant_y),
                ) {
                    rt_cur.push((treant_x, treant_y));
                    ht_cur[treant_x][treant_y] = true;
                }
            }

            let score_cur = board_score((0, n / 2), tij, &ht_cur, &mut rng);
            if score_cur < score_best {
                score_best = score_cur;
                rt_best = rt_cur;
                ht_best = ht_cur;
            }
        }

        // 初期囲い四方を試した後に更新する
        has_tree = ht_best.clone();
        ready_treants = rt_best.clone();
    }

    loop {
        input! {
            from &mut source,
            pij: (usize, usize),
            n: usize,
            xyn: [(usize, usize); n],
        }
        adventurer = pij;
        if adventurer == tij {
            break;
        }

        for (x, y) in xyn {
            is_found[x][y] = true;
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
