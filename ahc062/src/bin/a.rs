use proconio::fastout;
use proconio::input;
use rand::rngs::SmallRng;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use std::time::{Duration, Instant};

const N: usize = 200;
const DIRS: [(isize, isize); 9] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 0),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

const TIME_LIMIT_MS: u64 = 2950;

#[inline]
fn twod_to_oned(i: usize, j: usize) -> usize {
    i * N + j
}

fn calc_score(path: &[(usize, usize)], ann: &Vec<Vec<usize>>) -> usize {
    path.iter()
        .enumerate()
        .map(|(idx, &(r, c))| idx * ann[r][c])
        .sum()
}

fn calc_reverse_score_diff(
    i: usize,
    j: usize,
    path: &[(usize, usize)],
    ann: &Vec<Vec<usize>>,
) -> isize {
    let mut ret = 0;
    let sum_idx = (i + j) as isize;
    for k in i..=j {
        let (r, c) = path[k];
        ret += (sum_idx - 2 * k as isize) * ann[r][c] as isize;
    }
    ret
}

/// 区間 [i, j] を k の直後に移動させる際のスコア差分
fn calc_shift_score_diff(
    i: usize,
    j: usize,
    k: usize,
    path: &[(usize, usize)],
    ann: &Vec<Vec<usize>>,
) -> isize {
    let mut section_sum = 0isize;
    for idx in i..=j {
        let (r, c) = path[idx];
        section_sum += ann[r][c] as isize;
    }

    let move_dist = if k < i {
        // 前方に移動：インデックスが減る
        (k + 1) as isize - i as isize
    } else {
        // 後方に移動：インデックスが増える
        k as isize - j as isize
    };

    // 移動する区間の変化
    let mut diff = section_sum * move_dist;

    // 押し出される側の区間の変化
    if k < i {
        // [k+1, i-1] が後ろにずれる
        let shift_len = (j - i + 1) as isize;
        for idx in k + 1..i {
            let (r, c) = path[idx];
            diff += shift_len * ann[r][c] as isize;
        }
    } else {
        // [j+1, k] が前にずれる
        let shift_len = (j - i + 1) as isize;
        for idx in j + 1..=k {
            let (r, c) = path[idx];
            diff -= shift_len * ann[r][c] as isize;
        }
    }
    diff
}

fn is_adj(p1: (usize, usize), p2: (usize, usize)) -> bool {
    (p1.0 as isize - p2.0 as isize)
        .abs()
        .max((p1.1 as isize - p2.1 as isize).abs())
        <= 1
}

fn could_reversed(i: usize, j: usize, paths: &[(usize, usize)]) -> bool {
    if i == 0 || j + 1 >= N * N {
        return false;
    }
    is_adj(paths[i - 1], paths[j]) && is_adj(paths[i], paths[j + 1])
}

fn could_shifted(i: usize, j: usize, k: usize, paths: &[(usize, usize)]) -> bool {
    if i == 0 || j + 1 >= N * N || (k >= i - 1 && k <= j) {
        return false;
    }
    // 3 箇所の繋ぎ変えをチェック
    if !is_adj(paths[i - 1], paths[j + 1]) {
        return false;
    }
    if k + 1 < N * N {
        is_adj(paths[k], paths[i]) && is_adj(paths[j], paths[k + 1])
    } else {
        // 末尾への移動
        is_adj(paths[k], paths[i])
    }
}

#[fastout]
fn main() {
    let start_time = Instant::now();
    let mut rng = SmallRng::seed_from_u64(0);
    input! { _n: usize, ann: [[usize; N]; N] }

    let mut cur_path = [(0, 0); N * N];
    let mut path_order = [[0; N]; N];
    for i in 0..N {
        for j in 0..N {
            let jj = if i % 2 == 0 { j } else { N - j - 1 };
            let idx = twod_to_oned(i, j);
            cur_path[idx] = (i, jj);
        }
    }
    for idx in 0..N * N {
        let (r, c) = cur_path[idx];
        path_order[r][c] = idx;
    }

    let mut cur_score = calc_score(&cur_path, &ann);
    let mut ans_path = cur_path.clone();
    let mut ans_score = cur_score;

    while start_time.elapsed() < Duration::from_millis(TIME_LIMIT_MS) {
        let progress = start_time.elapsed().as_millis() as f64 / TIME_LIMIT_MS as f64;
        let temp = 100.0 * (1.0 - progress);

        if rng.random_bool(0.20) {
            // --- Reverse 遷移 ---
            let i = rng.random_range(1..N * N);
            let d = DIRS.choose(&mut rng).unwrap();
            let pos_prev = cur_path[i - 1];
            let pos_j = (
                pos_prev.0.wrapping_add_signed(d.0),
                pos_prev.1.wrapping_add_signed(d.1),
            );
            if pos_j.0 >= N || pos_j.1 >= N {
                continue;
            }
            let j = path_order[pos_j.0][pos_j.1];
            let (start, end) = (i.min(j), i.max(j));

            if could_reversed(start, end, &cur_path) {
                let diff = calc_reverse_score_diff(start, end, &cur_path, &ann);
                if diff > 0 || rng.random_bool((diff as f64 / temp).exp().min(1.0)) {
                    cur_path[start..=end].reverse();
                    for idx in start..=end {
                        let (r, c) = cur_path[idx];
                        path_order[r][c] = idx;
                    }
                    cur_score = (cur_score as isize + diff) as usize;
                }
            }
        } else {
            // --- Shift 遷移 ---
            let i = rng.random_range(1..N * N - 1);
            // 短い区間の方が成功しやすい
            let j = (i + rng.random_range(0..10)).min(N * N - 2);
            let d = DIRS.choose(&mut rng).unwrap();
            let pos_k = (
                cur_path[i - 1].0.wrapping_add_signed(d.0),
                cur_path[i - 1].1.wrapping_add_signed(d.1),
            );
            if pos_k.0 >= N || pos_k.1 >= N {
                continue;
            }
            let k = path_order[pos_k.0][pos_k.1];

            if could_shifted(i, j, k, &cur_path) {
                let diff = calc_shift_score_diff(i, j, k, &cur_path, &ann);
                if diff > 0 || rng.random_bool((diff as f64 / temp).exp().min(1.0)) {
                    let move_range = if k < i { k + 1..=j } else { i..=k };
                    if k < i {
                        cur_path[k + 1..=j].rotate_right(j - i + 1);
                    } else {
                        cur_path[i..=k].rotate_left(j - i + 1);
                    }
                    for idx in move_range {
                        let (r, c) = cur_path[idx];
                        path_order[r][c] = idx;
                    }
                    cur_score = (cur_score as isize + diff) as usize;
                }
            }
        }

        if cur_score > ans_score {
            ans_score = cur_score;
            ans_path = cur_path.clone();
        }
    }

    for (r, c) in ans_path {
        println!("{} {}", r, c);
    }
}
