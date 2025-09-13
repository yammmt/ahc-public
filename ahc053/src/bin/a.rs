// use itertools::Itertools;
// use petgraph::unionfind::UnionFind;
use proconio::fastout;
use proconio::input;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
// use std::collections::BinaryHeap;
// use std::collections::BTreeSet;
// use std::collections::HashSet;
// use std::collections::HashMap;
// use std::collections::VecDeque;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};

// 固定
const N: usize = 500;
const M: usize = 50;
const L: i64 = 1_000_000_000_000_000 - 2 * 1_000_000_000_000;
const U: i64 = 1_000_000_000_000_000 + 2 * 1_000_000_000_000;

#[fastout]
fn main() {
    // 2000 ms
    const RUN_TIME_MAX_MS: u64 = 1930;

    let start_time = Instant::now();
    let break_time = Duration::from_millis(RUN_TIME_MAX_MS);
    let mut rng = SmallRng::from_entropy();

    input! {
        _n: usize,
        _m: usize,
        _l: usize,
        _r: usize,
    }
    // 完璧に振り分けられると 1e8 点が得られる
    // A_i が 500 個で振り分け先が 50 個だから, 全探索すると 500^50
    // [L, R] の数を 500 個作って近い順に採用, が実装楽そうだが工夫がない
    // それで 1 領域に 1 要素を与える初期解を作っておいて, swap と残りの add を
    // 山登りやら焼きなまし, である程度は取れそう
    // この操作はカードの初期配布先を 0 にしておくと実質 swap だけ
    // A_i は決め打ちだがどうやるとよいのやら？初期解を全部 (L+U)/2 にしてやるとか
    // 工夫が弱い...

    let mut an = vec![(L + U) / 2; N];
    let nm4 = (N - M) / 4;
    let uml4 = (U - L) / 4;
    for i in M..M + nm4 {
        // 1/8
        an[i] = an[i] - (uml4 + uml4 / 2);
    }
    for i in M + nm4..M + 2 * nm4 {
        // 1/4
        an[i] = an[i] - uml4;
    }
    for i in M + 2 * nm4..M + 3 * nm4 {
        // 3/4
        an[i] = an[i] + uml4;
    }
    for i in M + 3 * nm4..N {
        // 7/8
        an[i] = an[i] + uml4 + uml4 / 2;
    }
    for (i, a) in an.iter().enumerate() {
        print!("{a}");
        if i == N - 1 {
            println!();
        } else {
            print!(" ");
        }
    }
    stdout().flush().unwrap();

    input! {
        bm: [i64; M],
    }
    let mut ans = vec![0; N];
    for i in 0..M {
        // 雑に半分を入れておく
        ans[i] = i + 1;
    }

    // TODO:
    while start_time.elapsed() < break_time {
        break;
    }

    // 解の出力
    for (i, a) in ans.iter().enumerate() {
        print!("{a}");
        if i == N - 1 {
            println!();
        } else {
            print!(" ");
        }
    }
}
