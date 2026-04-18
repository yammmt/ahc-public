use proconio::fastout;
use proconio::input;
use rand::rngs::SmallRng;
use rand::seq::IndexedRandom;
use rand::{Rng, SeedableRng};
use std::time::{Duration, Instant};

// 固定
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

/// 区間 [i, j] でのスコアを計算する
fn calc_score(i: usize, j: usize, path: &[(usize, usize)], ann: &Vec<Vec<usize>>) -> usize {
    let mut ret = 0;
    for a in i..=j {
        let ii = path[a].0;
        let jj = path[a].1;
        ret += a * ann[ii][jj];
    }
    ret
}

/// 区間 [i, j] を反転した場合のスコア差分を計算する
/// 値が正であれば, スコアは改善する
fn calc_reverse_score_diff(
    i: usize,
    j: usize,
    path: &[(usize, usize)],
    ann: &Vec<Vec<usize>>,
) -> isize {
    let mut ret = 0;

    for k in i..=j {
        let (r, c) = path[k];
        let val = ann[r][c] as isize;

        let old_idx = k as isize;
        let new_idx = (i + j - k) as isize;

        // (新しい位置 - 元の位置) * マスの値 = そのマスのスコア変化
        ret += (new_idx - old_idx) * val;
    }
    ret
}

/// i から j までの経路を反転できるなら true
fn could_reversed(i: usize, j: usize, paths: &[(usize, usize)]) -> bool {
    // (i-1) -> i -> (i+1) -> ... -> j -> (j+1)
    // (i-1) -> j -> (j-1) -> ... -> i -> (j+1)

    // i-1 と j が隣接か
    if i > 0 {
        let dist = (paths[i - 1].0 as isize - paths[j].0 as isize)
            .abs()
            .max((paths[i - 1].1 as isize - paths[j].1 as isize).abs());
        if dist > 1 {
            return false;
        }
    }

    // i と j+1 が隣接か
    if j + 1 < N * N {
        let dist = (paths[i].0 as isize - paths[j + 1].0 as isize)
            .abs()
            .max((paths[i].1 as isize - paths[j + 1].1 as isize).abs());
        if dist > 1 {
            return false;
        }
    }

    true
}

#[fastout]
fn main() {
    let start_time = Instant::now();
    let mut rng = SmallRng::seed_from_u64(0);

    input! {
        _n: usize,
        ann: [[usize; N]; N],
    }

    // なるべく ann 昇順でマスを回る
    // すべてのマスを回らなくてはならない
    // 移動範囲に制限があるので, いい感じに一筆書きを保つようにしなければ
    // 初期経路作って時間いっぱい繋ぎ変える, では解法として楽しくない…
    // 初期経路の最初と最後のマスを固定…ではなんか深みのある探索が必要になりそうで, 時間割けばできるけど嫌

    let mut cur_path = [(0, 0); N * N];
    let mut path_order = [[0; N]; N];
    for i in 0..N {
        for j in 0..N {
            let jj = if i % 2 == 0 { j } else { N - j - 1 };
            cur_path[twod_to_oned(i, j)] = (i, jj);
            path_order[i][j] = i * N + j;
        }
    }
    let mut cur_score = calc_score(0, N * N - 1, &cur_path, &ann);

    let mut ans_path = cur_path.clone();
    let mut ans_score = cur_score;
    while start_time.elapsed() < Duration::from_millis(TIME_LIMIT_MS) {
        // 隣接マスを選択
        // i ではなく i-1 に対する隣接を見る
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

        // i と j の大小関係を整理
        let (start, end) = if i < j { (i, j) } else { (j, i) };

        if !could_reversed(start, end, &cur_path) {
            continue;
        }

        let score_diff = calc_reverse_score_diff(start, end, &cur_path, &ann);

        if score_diff > 0 {
            cur_path[start..=end].reverse();
            for k in start..=end {
                let (r, c) = cur_path[k];
                path_order[r][c] = k;
            }
            cur_score = (cur_score as isize + score_diff) as usize;

            if cur_score > ans_score {
                ans_path = cur_path.clone();
                ans_score = cur_score;
            }
        }
    }

    for i in 0..N * N {
        println!("{} {}", ans_path[i].0, ans_path[i].1);
    }
}
