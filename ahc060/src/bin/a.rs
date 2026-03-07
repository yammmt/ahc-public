use proconio::input;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::time::{Duration, Instant};

const T_MAX: usize = 10000;
// 2 s
const TIME_LIMIT_MS: u64 = 1980;

const COLOR_CHANGE_PCT: u64 = 1;

#[derive(Default)]
struct Pos {
    cur: usize,
    prev: usize,
}

fn main() {
    let start_time = Instant::now();
    let break_time = Duration::from_millis(TIME_LIMIT_MS);
    let mut rng = SmallRng::seed_from_u64(1);

    input! {
        n: usize,
        m: usize,
        k: usize,
        _t: usize,
        abm: [(usize, usize); m],
        _xyn: [(usize, usize); n],
    }

    let mut edges = vec![vec![]; n];
    for (a, b) in abm {
        edges[a].push(b);
        edges[b].push(a);
    }
    let edges = edges;

    let mut ans_score = 0;
    let mut ans_moves = Vec::with_capacity(T_MAX);
    while start_time.elapsed() < break_time {
        // TODO: ループごとに領域確保入るので遅いはず
        let mut is_red = vec![false; n];
        let mut cur_score = 0;
        let mut cur_moves = Vec::with_capacity(T_MAX);
        let mut icecream_delivered = vec![HashSet::new(); k];

        let mut cur_icecream = vec![];
        let mut cur_pos = Pos::default();
        while cur_moves.len() < T_MAX {
            let mut next_pos = cur_pos.prev;
            while next_pos == cur_pos.prev {
                next_pos = edges[cur_pos.cur][rng.random_range(0..edges[cur_pos.cur].len())];
            }

            cur_moves.push(next_pos as isize);
            if next_pos < k {
                // 納品
                cur_score -= icecream_delivered[next_pos].len();
                icecream_delivered[next_pos].insert(cur_icecream.clone());
                cur_score += icecream_delivered[next_pos].len();
                cur_icecream.clear();
            } else {
                // 収穫
                cur_icecream.push(is_red[next_pos]);

                if !is_red[next_pos]
                    && cur_moves.len() < T_MAX - 1
                    && rng.random_range(1..=100) <= COLOR_CHANGE_PCT
                {
                    cur_moves.push(-1);
                    is_red[next_pos] = true;
                }
            }

            cur_pos.prev = cur_pos.cur;
            cur_pos.cur = next_pos;
        }

        if cur_score > ans_score {
            ans_score = cur_score;
            ans_moves = cur_moves;
        }
    }

    ans_moves.iter().for_each(|a| println!("{a}"));
}
