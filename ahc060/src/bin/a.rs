use proconio::input;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const T_MAX: usize = 10000;
// 2 s
const TIME_LIMIT_MS: u64 = 1800;

const COLOR_CHANGE_PCT: u64 = 1;
const PATH_SEARCH_MAX_LEN: usize = 8;

#[derive(Clone, Copy, Default)]
struct Pos {
    cur: usize,
    prev: usize,
}

// FIXME: 使う
#[allow(dead_code)]
fn does_score_raise(
    icecream: &[bool],
    delivered: &[HashSet<Vec<bool>>],
    cur_v: Pos,
    next_v: usize,
    k: usize,
) -> bool {
    cur_v.prev != next_v && next_v < k && !delivered[next_v].contains(icecream)
}

fn score_raise_path(
    icecream: &[bool],
    delivered: &[HashSet<Vec<bool>>],
    is_red: &[bool],
    max_depth: usize,
    edges: &[Vec<usize>],
    begin_pos: Pos,
    k: usize,
) -> Option<Vec<Pos>> {
    let mut que = VecDeque::new();
    // (経路, アイスクリーム)
    que.push_back((vec![begin_pos], icecream.to_vec()));
    while let Some((vpos, cur_icecream)) = que.pop_front() {
        let cur_pos = *vpos.last().unwrap();

        // 店は現れないとする

        for &next_pos in &edges[cur_pos.cur] {
            if next_pos == cur_pos.prev {
                continue;
            }

            if next_pos < k {
                // 納品判定
                if !delivered[next_pos].contains(&cur_icecream) {
                    let mut ret = vpos.clone();
                    ret.push(Pos {
                        cur: next_pos,
                        prev: cur_pos.cur,
                    });
                    return Some(ret);
                }
            } else {
                if vpos.len() == max_depth {
                    continue;
                }

                let mut v = vpos.clone();
                v.push(Pos {
                    cur: next_pos,
                    prev: cur_pos.cur,
                });
                let mut next_icecream = cur_icecream.clone();
                next_icecream.push(is_red[next_pos]);
                que.push_back((v, next_icecream));
            }
        }
    }

    None
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
            // 納品してスコアが増えるなら納品する
            if let Some(vpath) = score_raise_path(
                &cur_icecream,
                &icecream_delivered,
                &is_red,
                PATH_SEARCH_MAX_LEN.min(T_MAX - cur_moves.len()),
                &edges,
                cur_pos,
                k,
            ) {
                for &v in vpath.iter().skip(1) {
                    cur_moves.push(v.cur as isize);
                    if v.cur < k {
                        // 納品パスだから店は最後に一度しか現れない
                        icecream_delivered[v.cur].insert(cur_icecream.clone());
                        cur_icecream.clear();
                    } else {
                        cur_icecream.push(is_red[v.cur]);
                    }
                }
                cur_score += 1;
                cur_pos = *vpath.last().unwrap();

                continue;
            }

            // 納品済みのアイスを可能な限り納品しない, を実現する仕組み
            let candidates: Vec<_> = edges[cur_pos.cur]
                .iter()
                .copied()
                .filter(|&v| {
                    if v == cur_pos.prev {
                        return false;
                    }
                    if v < k && icecream_delivered[v].contains(&cur_icecream) {
                        return false;
                    }
                    true
                })
                .collect();
            let mut next_pos = cur_pos.prev;
            while next_pos == cur_pos.prev {
                next_pos = if !candidates.is_empty() {
                    candidates[rng.random_range(0..candidates.len())]
                } else {
                    edges[cur_pos.cur][rng.random_range(0..edges[cur_pos.cur].len())]
                };
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
