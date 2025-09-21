use proconio::marker::Chars;
use proconio::{input, source::line::LineSource};
use rand::prelude::*;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use std::collections::VecDeque;
use std::io::{stdout, Write};

// 2 s
#[allow(dead_code)]
const TIME_LIMIT_MS: usize = 1980;

const DXY: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

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

fn could_add_treant(
    sxy: (usize, usize),
    gxy: (usize, usize),
    is_found: &Vec<Vec<bool>>,
    has_tree: &Vec<Vec<bool>>,
    treant_xy: (usize, usize),
) -> bool {
    let n = has_tree.len();
    let (tx, ty) = treant_xy;
    if tx >= n || ty >= n || is_found[tx][ty] || has_tree[tx][ty] {
        return false;
    }

    let mut ht = has_tree.clone();
    ht[tx][ty] = true;
    could_goal(sxy, gxy, &ht)
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

    let mut rng = SmallRng::from_entropy();
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

    // goal を視認されないよう, 三方を囲む
    // TODO: ランダム性をもたせる意味とは
    let mut dxy = DXY.clone();
    dxy.shuffle(&mut rng);
    for (dx, dy) in dxy {
        let tx = tij.0.wrapping_add_signed(dx);
        let ty = tij.1.wrapping_add_signed(dy);
        if could_add_treant(adventurer, tij, &is_found, &has_tree, (tx, ty)) {
            ready_treants.push((tx, ty));
            has_tree[tx][ty] = true;
        } else {
            // 囲めなかった部分に対し, 一マス空けて視界を遮る木を立てたい
            let tx = tx.wrapping_add_signed(dx);
            let ty = ty.wrapping_add_signed(dy);
            if could_add_treant(adventurer, tij, &is_found, &has_tree, (tx, ty)) {
                ready_treants.push((tx, ty));
                has_tree[tx][ty] = true;
            }
        }
    }

    // 列単位で適当に間引く (置く -> 置かない -> 置かない, をループ)
    for i in 0..n {
        for j in 0..n {
            let nj = j + (i % 4);
            if j % 5 == 0 && nj < n && !has_tree[i][nj] && (i, nj) != tij {
                if could_add_treant(adventurer, tij, &is_found, &has_tree, (i, nj)) {
                    ready_treants.push((i, nj));
                    has_tree[i][nj] = true;
                }
            }
        }
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
