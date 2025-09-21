use proconio::marker::Chars;
use proconio::{input, source::line::LineSource};
#[allow(unused_imports)]
use rand::prelude::*;
#[allow(unused_imports)]
use rand::rngs::SmallRng;
#[allow(unused_imports)]
use rand::SeedableRng;
use std::collections::VecDeque;
use std::io::{stdout, Write};

// 2 s
#[allow(dead_code)]
const TIME_LIMIT_MS: usize = 1980;

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

fn main() {
    let stdin = std::io::stdin();
    let mut source = LineSource::new(stdin.lock());

    input! {
        from &mut source,
        n: usize,
        tij: (usize, usize),
        bnn: [Chars; n],
    }

    #[allow(unused_mut, unused_variables)]
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

    // ゴールの三方を塞ぐ
    let mut score_best = 0;
    let mut ht_best = has_tree.clone();
    let mut rt_best = ready_treants.clone();
    for &dxy in &[DXY_LB, DXY_LT, DXY_RB, DXY_RT] {
        let mut ht_cur = has_tree.clone();
        let mut rt_cur = ready_treants.clone();
        add_treants_surrounding_goal((0, n / 2), tij, &is_found, &mut ht_cur, &mut rt_cur, &dxy);

        let shortest_paths_cur = shortest_paths((0, n / 2), &ht_cur);
        let score_cur = shortest_paths_cur[tij.0][tij.1];
        if score_cur > score_best {
            score_best = score_cur;
            rt_best = rt_cur;
            ht_best = ht_cur;
        }
    }
    has_tree = ht_best;
    ready_treants = rt_best;

    // X の形にトレントを置く
    add_treants_x(
        (0, n / 2),
        tij,
        &is_found,
        &mut has_tree,
        &mut ready_treants,
    );

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
