use proconio::fastout;
use proconio::input;
use rand::rngs::SmallRng;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use std::time::{Duration, Instant};

const N: usize = 200;
const NN: usize = N * N;
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

struct ScoreManager {
    sum_v: Vec<isize>,
    sum_iv: Vec<isize>,
}

impl ScoreManager {
    fn new(path: &[(usize, usize)], ann: &Vec<Vec<usize>>) -> Self {
        let mut sm = Self {
            sum_v: vec![0; NN + 1],
            sum_iv: vec![0; NN + 1],
        };
        sm.update_all(path, ann);
        sm
    }

    fn update_all(&mut self, path: &[(usize, usize)], ann: &Vec<Vec<usize>>) {
        let mut cur_v = 0;
        let mut cur_iv = 0;
        for i in 0..NN {
            let val = ann[path[i].0][path[i].1] as isize;
            cur_v += val;
            cur_iv += i as isize * val;
            self.sum_v[i + 1] = cur_v;
            self.sum_iv[i + 1] = cur_iv;
        }
    }

    fn diff_reverse(&self, i: usize, j: usize) -> isize {
        let s1 = self.sum_v[j + 1] - self.sum_v[i];
        let s2 = self.sum_iv[j + 1] - self.sum_iv[i];
        (i + j) as isize * s1 - 2 * s2
    }

    fn diff_shift(&self, i: usize, j: usize, k: usize) -> isize {
        let s_val = self.sum_v[j + 1] - self.sum_v[i];
        let move_dist = if k < i {
            (k + 1) as isize - i as isize
        } else {
            k as isize - j as isize
        };
        let mut diff = s_val * move_dist;

        if k < i {
            let other_v = self.sum_v[i] - self.sum_v[k + 1];
            diff += (j - i + 1) as isize * other_v;
        } else {
            let other_v = self.sum_v[k + 1] - self.sum_v[j + 1];
            diff -= (j - i + 1) as isize * other_v;
        }
        diff
    }
}

#[inline]
fn is_adj(p1: (usize, usize), p2: (usize, usize)) -> bool {
    (p1.0 as isize - p2.0 as isize)
        .abs()
        .max((p1.1 as isize - p2.1 as isize).abs())
        <= 1
}

#[fastout]
fn main() {
    let start_time = Instant::now();
    let mut rng = SmallRng::seed_from_u64(0);
    input! { _n: usize, ann: [[usize; N]; N] }

    let mut cur_path = [(0, 0); NN];
    let mut path_order = [[0; N]; N];
    for i in 0..N {
        for j in 0..N {
            let jj = if i % 2 == 0 { j } else { N - j - 1 };
            let idx = i * N + j;
            cur_path[idx] = (i, jj);
            path_order[i][jj] = idx;
        }
    }

    let mut sm = ScoreManager::new(&cur_path, &ann);
    let mut cur_score = sm.sum_iv[NN] as usize;
    let mut ans_path = cur_path.clone();
    let mut ans_score = cur_score;

    while start_time.elapsed() < Duration::from_millis(TIME_LIMIT_MS) {
        let progress = start_time.elapsed().as_millis() as f64 / TIME_LIMIT_MS as f64;
        let temp = 100.0 * (1.0 - progress);

        if rng.random_bool(0.20) {
            // --- Reverse 遷移 ---
            let i = rng.gen_range(1..NN);
            let d = DIRS.choose(&mut rng).unwrap();
            let p_prev = cur_path[i - 1];
            let target = (
                p_prev.0.wrapping_add_signed(d.0),
                p_prev.1.wrapping_add_signed(d.1),
            );
            if target.0 >= N || target.1 >= N {
                continue;
            }
            let j = path_order[target.0][target.1];

            // 2-opt の対称性を厳密に処理
            let (start, end) = if i < j {
                (i, j)
            } else if j + 1 < i {
                (j + 1, i - 1)
            } else {
                continue;
            };

            if start < end && (end + 1 == NN || is_adj(cur_path[start], cur_path[end + 1])) {
                let diff = sm.diff_reverse(start, end);
                if diff > 0 || (temp > 0.0 && rng.random_bool((diff as f64 / temp).exp().min(1.0)))
                {
                    cur_path[start..=end].reverse();
                    for idx in start..=end {
                        let (r, c) = cur_path[idx];
                        path_order[r][c] = idx;
                    }
                    sm.update_all(&cur_path, &ann);
                    cur_score = (cur_score as isize + diff) as usize;
                }
            }
        } else {
            // --- Shift 遷移 ---
            let i = rng.gen_range(1..NN - 1);
            let j = (i + rng.gen_range(0..15)).min(NN - 2);

            // i に隣接する k を探すことで、有効な Shift を高確率で引き当てる
            let d = DIRS.choose(&mut rng).unwrap();
            let p_i = cur_path[i];
            let target = (
                p_i.0.wrapping_add_signed(d.0),
                p_i.1.wrapping_add_signed(d.1),
            );
            if target.0 >= N || target.1 >= N {
                continue;
            }
            let k = path_order[target.0][target.1];

            if k < i - 1 || k > j {
                if is_adj(cur_path[i - 1], cur_path[j + 1]) {
                    // is_adj(cur_path[k], cur_path[i]) は上記 target の取得により保証されている
                    if k + 1 == NN || is_adj(cur_path[j], cur_path[k + 1]) {
                        let diff = sm.diff_shift(i, j, k);
                        if diff > 0
                            || (temp > 0.0 && rng.random_bool((diff as f64 / temp).exp().min(1.0)))
                        {
                            let (range_l, range_r) = if k < i { (k + 1, j) } else { (i, k) };
                            if k < i {
                                cur_path[k + 1..=j].rotate_right(j - i + 1);
                            } else {
                                cur_path[i..=k].rotate_left(j - i + 1);
                            }
                            for idx in range_l..=range_r {
                                let (r, c) = cur_path[idx];
                                path_order[r][c] = idx;
                            }
                            sm.update_all(&cur_path, &ann);
                            cur_score = (cur_score as isize + diff) as usize;
                        }
                    }
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
