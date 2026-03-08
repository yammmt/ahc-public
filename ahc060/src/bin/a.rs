use proconio::input;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const T_MAX: usize = 10000;

// 2 s
// BFS 部分が遅すぎるのであまり回せない
const TIME_LIMIT_MS: u64 = 1930;

// 色編が貪欲探索失敗時のみになっているので, 変えてやったほうが次回の探索でうまくいき易いっぽい
// が, 後半にネタ切れとなるリスクがある
const COLOR_CHANGE_PCT: u64 = 7;
const PATH_SEARCH_MAX_LEN: usize = 8;

#[derive(Clone, Copy, Debug, Default)]
struct Pos {
    cur: usize,
    prev: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
struct IceCream {
    taste: u128,
    len: usize,
}

impl IceCream {
    fn add_white(&mut self) {
        debug_assert!(self.len < 127);
        self.len += 1;
    }

    fn add_red(&mut self) {
        debug_assert!(self.len < 127);
        self.taste |= 1 << self.len;
        self.len += 1;
    }

    fn clear(&mut self) {
        self.taste = 0;
        self.len = 0;
    }
}

#[derive(Debug, Default)]
struct State {
    // 変わるもの
    pos: Pos,
    icecream: IceCream,
    delivered: Vec<HashSet<IceCream>>,
    is_red: Vec<bool>,
    score: usize,
    // 変わらないもの
    k: usize,
    edges: Vec<Vec<usize>>,
}

impl State {
    fn apply_move(&mut self, vnext: usize) {
        debug_assert!(vnext != self.pos.prev);
        self.pos.prev = self.pos.cur;
        self.pos.cur = vnext;

        if self.pos.cur < self.k {
            // 店到着
            self.score -= self.delivered[self.pos.cur].len();
            self.delivered[self.pos.cur].insert(self.icecream);
            self.score += self.delivered[self.pos.cur].len();

            self.icecream.clear();
        } else {
            // アイスクリーム回収
            if self.is_red[self.pos.cur] {
                self.icecream.add_red();
            } else {
                self.icecream.add_white();
            }
        }
    }

    fn change_color(&mut self, v: usize) {
        self.is_red[v] = true;
    }

    fn score_raise_path(&self, max_depth: usize) -> Option<Vec<Pos>> {
        let mut que = VecDeque::new();
        // (経路, アイスクリーム)
        que.push_back((vec![self.pos], self.icecream));
        while let Some((vpos, cur_icecream)) = que.pop_front() {
            let cur_pos = *vpos.last().unwrap();

            // 店は現れないとする

            for &next_pos in &self.edges[cur_pos.cur] {
                if next_pos == cur_pos.prev {
                    continue;
                }

                if next_pos < self.k {
                    // 納品判定
                    if !self.delivered[next_pos].contains(&cur_icecream) {
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
                    if self.is_red[next_pos] {
                        next_icecream.add_red();
                    } else {
                        next_icecream.add_white();
                    }

                    que.push_back((v, next_icecream));
                }
            }
        }

        None
    }

    fn random_next_pos(&self, rng: &mut SmallRng) -> Pos {
        // 納品済みのアイスは可能な限り納品しない
        let candidates: Vec<_> = self.edges[self.pos.cur]
            .iter()
            .copied()
            .filter(|&v| {
                if v == self.pos.prev {
                    return false;
                }
                if v < self.k && self.delivered[v].contains(&self.icecream) {
                    return false;
                }
                true
            })
            .collect();
        let mut next_pos = self.pos.prev;
        while next_pos == self.pos.prev {
            next_pos = if !candidates.is_empty() {
                candidates[rng.random_range(0..candidates.len())]
            } else {
                self.edges[self.pos.cur][rng.random_range(0..self.edges[self.pos.cur].len())]
            };
        }

        Pos {
            cur: next_pos,
            prev: self.pos.cur,
        }
    }
}

// FIXME: 使う
#[allow(dead_code)]
fn does_score_raise(
    icecream: &IceCream,
    delivered: &[HashSet<IceCream>],
    cur_v: Pos,
    next_v: usize,
    k: usize,
) -> bool {
    cur_v.prev != next_v && next_v < k && !delivered[next_v].contains(icecream)
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
        let mut state = State {
            pos: Pos::default(),
            icecream: IceCream::default(),
            delivered: vec![HashSet::new(); n],
            is_red: vec![false; n],
            score: 0,
            k,
            edges: edges.clone(),
        };
        let mut cur_moves = Vec::with_capacity(T_MAX);

        while cur_moves.len() < T_MAX {
            // 納品してスコアが増えるなら納品する
            if let Some(vpath) =
                state.score_raise_path(PATH_SEARCH_MAX_LEN.min(T_MAX - cur_moves.len()))
            {
                for &v in vpath.iter().skip(1) {
                    cur_moves.push(v.cur as isize);
                    state.apply_move(v.cur);
                }

                continue;
            }

            let next_pos = state.random_next_pos(&mut rng);
            cur_moves.push(next_pos.cur as isize);
            state.apply_move(next_pos.cur);
            if next_pos.cur >= state.k
                && !state.is_red[next_pos.cur]
                && cur_moves.len() < T_MAX - 1
                && rng.random_range(1..=100) <= COLOR_CHANGE_PCT
            {
                cur_moves.push(-1);
                state.change_color(next_pos.cur);
            }
        }

        if state.score > ans_score {
            ans_score = state.score;
            ans_moves = cur_moves;
        }
    }

    ans_moves.iter().for_each(|a| println!("{a}"));
}
