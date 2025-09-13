use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::io::{stdout, Write};
use std::time::{Duration, Instant};

// 固定
const N: usize = 500;
const M: usize = 50;
const L: i64 = 1_000_000_000_000_000 - 2 * 1_000_000_000_000;
const U: i64 = 1_000_000_000_000_000 + 2 * 1_000_000_000_000;

// swap 一回であれば差分を取ったほうが高速だが絶対値に注意
fn calc_score(an: &[i64], bm: &[i64], distributed_to: &[usize]) -> i64 {
    let mut mountains = vec![0; M];
    for i in 0..N {
        if distributed_to[i] == 0 {
            continue;
        }

        mountains[distributed_to[i] - 1] += an[i];
    }

    let mut ret = 0;
    for i in 0..M {
        ret += (bm[i] - mountains[i]).abs();
    }

    ret
}

fn _calc_mountains(an: &[i64], distributed_to: &[usize]) -> Vec<i64> {
    let mut ret = vec![0; M];
    for i in 0..N {
        if distributed_to[i] == 0 {
            continue;
        }

        ret[distributed_to[i] - 1] += an[i];
    }
    ret
}

fn get_line() -> String {
    let mut s = String::new();
    std::io::stdin().read_line(&mut s).ok();
    s.trim().to_string()
}

fn main() {
    // 2000 ms
    const RUN_TIME_MAX_MS: u64 = 1930;

    let start_time = Instant::now();
    let break_time = Duration::from_millis(RUN_TIME_MAX_MS);
    let mut rng = SmallRng::from_entropy();

    let _nmlu: Vec<i64> = get_line()
        .split_whitespace()
        .map(|tok| tok.parse::<i64>().expect("failed to parse i64"))
        .collect();

    // 完璧に振り分けられると 1e8 点が得られる
    // A_i が 500 個で振り分け先が 50 個だから, 全探索すると 500^50
    // [L, R] の数を 500 個作って近い順に採用, が実装楽そうだが工夫がない
    // それで 1 領域に 1 要素を与える初期解を作っておいて, swap と残りの add を
    // 山登りやら焼きなまし, である程度は取れそう
    // この操作はカードの初期配布先を 0 にしておくと実質 swap だけ
    // A_i は決め打ちだがどうやるとよいのやら？初期解を全部 (L+U)/2 にしてやるとか
    // 工夫が弱い...

    // 初期値が上四桁 0998, 0999, 1000, 1001 の四通りとして
    // 微調整用の値を足す

    let mut an = vec![(L + U) / 2; N];
    // 初期値用
    for i in 0..20 {
        an[i] = 998_000_000_000_000;
    }
    for i in 20..40 {
        an[i] = 999_000_000_000_000;
    }
    for i in 40..60 {
        an[i] = 1_000_000_000_000_000;
    }
    for i in 60..80 {
        an[i] = 1_001_000_000_000_000;
    }
    // 微調整用
    // floor((10^12-10^9)/420)
    let d = 2_380_952_380;
    for i in 80..N {
        an[i] = (1_000_000_000 + d * (i - 80)) as i64;
    }

    let an_line = an
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    println!("{an_line}");
    stdout().flush().unwrap();

    let bm: Vec<i64> = get_line()
        .split_whitespace()
        .map(|tok| tok.parse::<i64>().expect("failed to parse i64"))
        .collect();
    let mut ans = vec![0; N];
    for i in 0..M {
        // 雑に半分を入れておく
        ans[i] = i + 1;
    }
    let mut score = calc_score(&an, &bm, &ans);
    // let mut mountains = calc_mountains(&an, &ans);

    while start_time.elapsed() < break_time {
        let mut ans_cur = ans.clone();
        let a_i = rng.gen::<usize>() % N;

        if rng.gen::<usize>() % 2 == 0 {
            // 加算
            let distribution_i = rng.gen::<usize>() % (M + 1);
            ans_cur[a_i] = distribution_i;
            let score_cur = calc_score(&an, &bm, &ans_cur);
            // println!("a[{a_i}] -> {distribution_i}, score: {score} -> {score_cur}");
            stdout().flush().unwrap();
            if score_cur < score {
                ans = ans_cur;
                score = score_cur;
            }
        } else {
            // 交換
            let distribution_i = ans[a_i];
            let a_j = rng.gen::<usize>() % N;
            let distribution_j = ans[a_j];
            ans_cur[a_i] = distribution_j;
            ans_cur[a_j] = distribution_i;
            let score_cur = calc_score(&an, &bm, &ans_cur);
            stdout().flush().unwrap();
            if score_cur < score {
                ans = ans_cur;
                score = score_cur;
            }
        }
    }

    // 解の出力
    let ans_line = ans
        .iter()
        .map(|x| x.to_string())
        .collect::<Vec<_>>()
        .join(" ");
    println!("{ans_line}");
    stdout().flush().unwrap();
}
