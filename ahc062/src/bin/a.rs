use proconio::fastout;
use proconio::input;
use std::time::{Duration, Instant};

// 固定
const N: usize = 200;

const TIME_LIMIT_MS: u64 = 2950;

#[inline]
fn twod_to_oned(i: usize, j: usize) -> usize {
    i * N + j
}

#[fastout]
fn main() {
    let start_time = Instant::now();

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
    for i in 0..N {
        for j in 0..N {
            let jj = if i % 2 == 0 { j } else { N - j - 1 };
            cur_path[twod_to_oned(i, j)] = (i, jj);
        }
    }

    let mut ans = cur_path.clone();
    while start_time.elapsed() < Duration::from_millis(TIME_LIMIT_MS) {
        break;
    }

    for i in 0..N * N {
        println!("{} {}", ans[i].0, ans[i].1);
    }
}
