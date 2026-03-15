use proconio::{input, source::line::LineSource};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;
use std::io::{Write, stdout};
use std::time::{Duration, Instant};

#[allow(unused_macros)]
macro_rules! debug {
    ($($arg:tt)+) => {
        if cfg!(debug_assertions) {
            eprintln!($($arg)+);
        }
    };
}

const CELL_NO_OWNER: isize = -1;

#[allow(dead_code)]
const CELL_FREE: usize = 0;
#[allow(dead_code)]
const CELL_MINE: usize = 1;
#[allow(dead_code)]
const CELL_ENEMY_LVL1: usize = 2;
#[allow(dead_code)]
const CELL_ENEMY_LVL2: usize = 3;

const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

// 2 s
#[allow(dead_code)]
const TIME_LIMIT_MS: u64 = 1980;
// 2000/100 より小さな値
const TIME_LIMIT_MC_1TURN_MS: u64 = 18;
const MC_DEPTH: usize = 10;

const N: usize = 10;
const T: usize = 100;

#[derive(Clone, Debug)]
struct State {
    owners: Vec<Vec<isize>>,
    levels: Vec<Vec<usize>>,
    pos: Vec<(usize, usize)>,
    values: Vec<Vec<usize>>,
    player_num: usize,
    max_level: usize,
    rng: SmallRng,
}

impl State {
    fn get_all_scores(&self) -> Vec<usize> {
        let mut ret = vec![0; self.player_num];

        for i in 0..N {
            for j in 0..N {
                if self.owners[i][j] == CELL_NO_OWNER {
                    continue;
                }

                ret[self.owners[i][j] as usize] += self.levels[i][j] * self.values[i][j];
            }
        }

        ret
    }

    fn get_my_score(&self) -> f64 {
        let scores = self.get_all_scores();
        let enemy_max = *scores.iter().skip(1).max().unwrap();
        scores[0] as f64 / enemy_max as f64
    }

    /// player が可能な行動をすべて返す
    fn get_possible_moves(&self, player_idx: isize) -> Vec<(usize, usize)> {
        let mut ret = vec![];
        let mut visited = vec![vec![false; N]; N];
        let mut que = VecDeque::new();
        que.push_back(self.pos[player_idx as usize]);
        while let Some((ci, cj)) = que.pop_front() {
            if visited[ci][cj] {
                continue;
            }

            visited[ci][cj] = true;
            ret.push((ci, cj));
            if self.owners[ci][cj] != player_idx {
                continue;
            }

            for &(di, dj) in &DIRS {
                let ni = ci.wrapping_add_signed(di);
                let nj = cj.wrapping_add_signed(dj);
                if ni >= N || nj >= N {
                    continue;
                }

                let mut enemy_exists = false;
                for i in 0..self.player_num {
                    if i as isize != player_idx && self.pos[i] == (ni, nj) {
                        enemy_exists = true;
                        break;
                    }
                }
                if enemy_exists {
                    continue;
                }

                que.push_back((ni, nj));
            }
        }

        ret
    }

    /// 全プレイヤーの行動を受け取り, 1 ターン進める
    fn advance(&mut self, moves: &[(usize, usize)]) {
        let mut actual_moves = vec![None; self.player_num];
        for i in 0..self.player_num {
            actual_moves[i] = Some(moves[i]);
            let (mi, mj) = moves[i];
            for j in 0..i {
                if actual_moves[i] == actual_moves[j] {
                    if self.owners[mi][mj] == i as isize {
                        actual_moves[j] = None;
                    } else if self.owners[mi][mj] == j as isize {
                        actual_moves[i] = None;
                    } else {
                        actual_moves[i] = None;
                        actual_moves[j] = None;
                    }
                }
            }
        }

        for i in 0..self.player_num {
            let Some(m) = actual_moves[i] else {
                continue;
            };

            if self.owners[m.0][m.1] == CELL_NO_OWNER {
                // 占領
                self.owners[m.0][m.1] = i as isize;
                self.levels[m.0][m.1] = 1;
                self.pos[i] = m;
            } else if self.owners[m.0][m.1] == i as isize {
                // 強化
                self.levels[m.0][m.1] = (self.levels[m.0][m.1] + 1).min(self.max_level);
                self.pos[i] = m;
            } else {
                // 攻撃
                self.levels[m.0][m.1] -= 1;
                if self.levels[m.0][m.1] == 0 {
                    self.owners[m.0][m.1] = i as isize;
                    self.levels[m.0][m.1] = 1;
                    self.pos[i] = m;
                }
            }
        }
    }

    /// 指定ターン数だけ擬似的にゲームを行い, その結果得られた自身のスコアを返す.
    fn playout(&self, first_move: (usize, usize), depth: usize) -> f64 {
        let mut s = self.clone();
        let mut moves = Vec::with_capacity(s.player_num);
        for i in 0..depth {
            for j in 0..s.player_num {
                if i == 0 && j == 0 {
                    moves.push(first_move);
                    continue;
                }

                let p = s.get_possible_moves(j as isize);
                moves.push(p[s.rng.random_range(0..p.len())]);
            }

            s.advance(&moves);
            moves.clear();
        }

        s.get_my_score()
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
struct Weight {
    free_cell: f64,
    my_cell: f64,
    enemy_cell_lvl1: f64,
    enemy_cell_lvl2: f64,
    random: f64,
}

impl Default for Weight {
    fn default() -> Self {
        Self {
            free_cell: 1.0,
            my_cell: 1.0,
            enemy_cell_lvl1: 1.0,
            enemy_cell_lvl2: 1.0,
            random: 0.8,
        }
    }
}

fn main() {
    let stdin = std::io::stdin();
    let mut source = LineSource::new(stdin.lock());

    input! {
        // ここは不要かも
        from &mut source,
        _n: usize,
        player_num: usize, // m
        _t: usize,
        max_level: usize,
        values: [[usize; N]; N], // v
        // 初期領土は一マスのみ
        sxym: [(usize, usize); player_num],
    }

    let pos = sxym.clone();
    let owners = vec![vec![CELL_NO_OWNER; N]; N];
    let levels = vec![vec![0; N]; N];
    let rng = SmallRng::seed_from_u64(1);
    let mut state = State {
        owners,
        levels,
        pos: pos.clone(),
        values,
        player_num,
        max_level,
        rng,
    };
    for (i, p) in pos.iter().enumerate() {
        state.owners[p.0][p.1] = i as isize;
        state.levels[p.0][p.1] = 1;
    }

    // TODO: パラメータ推定を入れると改善する
    let _weights = vec![Weight::default(); player_num];

    for _ in 0..T {
        let start_time = Instant::now();
        let my_possible_moves = state.get_possible_moves(0);
        let mut scores = vec![0.0; my_possible_moves.len()];
        let mut tries = vec![0; my_possible_moves.len()];
        let mut moves_i = 0;
        while start_time.elapsed() < Duration::from_millis(TIME_LIMIT_MC_1TURN_MS) {
            scores[moves_i] += state.playout(my_possible_moves[moves_i], MC_DEPTH);
            tries[moves_i] += 1;
            moves_i = (moves_i + 1) % my_possible_moves.len();
        }

        let mut my_move = (0, 0);
        let mut my_score = 0.0;
        for i in 0..my_possible_moves.len() {
            let s = scores[i] / tries[i] as f64;
            if s > my_score {
                my_move = my_possible_moves[i];
                my_score = s;
            }
        }

        println!("{} {}", my_move.0, my_move.1);
        stdout().flush().unwrap();

        // 読み込む
        // TODO: 一々メモリ確保走るので遅い
        input! {
            from &mut source,
            // 駒の移動先は捨てる, 現在位置とマスの現況さえわかればよいので
            _txym: [(usize, usize); state.player_num],
            // ターン終了時駒位置
            pp: [(usize, usize); state.player_num],
            // 各マスの所有者
            owners: [[isize; N]; N],
            // レベル
            levels: [[usize; N]; N],
        }
        state.pos = pp;
        state.owners = owners;
        state.levels = levels;
    }
}
