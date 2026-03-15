use proconio::{input, source::line::LineSource};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::io::{Write, stdout};
use std::time::{Duration, Instant};

const N: usize = 10;
const T_MAX: usize = 100;
const MAX_PLAYERS: usize = 8;
const CELL_NO_OWNER: i8 = -1;
const DIRS: [(isize, isize); 4] = [(-1, 0), (1, 0), (0, -1), (0, 1)];

const TIME_LIMIT_MS: u64 = 18;
const MC_DEPTH: usize = 10;

#[derive(Clone, Copy, Debug)]
struct State {
    owners: [[i8; N]; N],
    levels: [[u8; N]; N],
    pos: [(u8, u8); MAX_PLAYERS],
    player_num: usize,
    max_level: u8,
}

impl State {
    #[inline]
    fn get_raw_scores(&self, values: &[[u16; N]; N]) -> [u32; MAX_PLAYERS] {
        let mut ret = [0u32; MAX_PLAYERS];
        for i in 0..N {
            for j in 0..N {
                let owner = self.owners[i][j];
                if owner >= 0 {
                    ret[owner as usize] += (self.levels[i][j] as u32) * (values[i][j] as u32);
                }
            }
        }
        ret
    }

    fn evaluate(&self, values: &[[u16; N]; N]) -> f64 {
        let scores = self.get_raw_scores(values);
        let my_score = scores[0] as f64;
        let mut enemy_max = 0u32;
        for i in 1..self.player_num {
            if scores[i] > enemy_max {
                enemy_max = scores[i];
            }
        }
        // スコア比率の対数を取る（1.0を加えることで正の値を保証）
        (1.0 + my_score / enemy_max as f64).log2()
    }

    fn get_possible_moves(&self, player_idx: usize, out: &mut [(u8, u8); 100]) -> usize {
        let mut count = 0;
        let mut visited = [false; 100];
        let mut queue = [0u8; 100];
        let mut head = 0;
        let mut tail = 0;

        let (si, sj) = self.pos[player_idx];
        let start_idx = (si as usize) * 10 + (sj as usize);

        queue[tail] = start_idx as u8;
        tail += 1;
        visited[start_idx] = true;

        while head < tail {
            let curr = queue[head];
            head += 1;
            let ci = curr / 10;
            let cj = curr % 10;

            out[count] = (ci, cj);
            count += 1;

            // 自分のマスであれば、その隣接マスも移動候補になり得る（BFS拡張）
            if self.owners[ci as usize][cj as usize] == player_idx as i8 {
                for &(di, dj) in &DIRS {
                    let ni = ci as isize + di;
                    let nj = cj as isize + dj;

                    if ni >= 0 && ni < 10 && nj >= 0 && nj < 10 {
                        let idx = (ni * 10 + nj) as usize;
                        if !visited[idx] {
                            // 「他人が現在いるマス」への移動（攻撃）は禁止
                            let mut occupied = false;
                            for p in 0..self.player_num {
                                if p != player_idx && self.pos[p] == (ni as u8, nj as u8) {
                                    occupied = true;
                                    break;
                                }
                            }

                            if !occupied {
                                visited[idx] = true;
                                queue[tail] = idx as u8;
                                tail += 1;
                            }
                        }
                    }
                }
            }
        }
        count
    }

    fn advance(&mut self, moves: &[(u8, u8)]) {
        let mut actual_moves = [true; MAX_PLAYERS];
        // 競合解決
        for i in 0..self.player_num {
            for j in 0..i {
                if moves[i] == moves[j] {
                    let (mi, mj) = (moves[i].0 as usize, moves[i].1 as usize);
                    if self.owners[mi][mj] == i as i8 {
                        actual_moves[j] = false;
                    } else if self.owners[mi][mj] == j as i8 {
                        actual_moves[i] = false;
                    } else {
                        actual_moves[i] = false;
                        actual_moves[j] = false;
                    }
                }
            }
        }

        // 行動適用
        for i in 0..self.player_num {
            if !actual_moves[i] {
                continue;
            }
            let (mi, mj) = (moves[i].0 as usize, moves[i].1 as usize);

            if self.owners[mi][mj] == CELL_NO_OWNER {
                self.owners[mi][mj] = i as i8;
                self.levels[mi][mj] = 1;
                self.pos[i] = (mi as u8, mj as u8);
            } else if self.owners[mi][mj] == i as i8 {
                self.levels[mi][mj] = (self.levels[mi][mj] + 1).min(self.max_level);
                self.pos[i] = (mi as u8, mj as u8);
            } else {
                // 攻撃（他人のマス）
                if self.levels[mi][mj] > 0 {
                    self.levels[mi][mj] -= 1;
                    if self.levels[mi][mj] == 0 {
                        self.owners[mi][mj] = i as i8;
                        self.levels[mi][mj] = 1;
                        self.pos[i] = (mi as u8, mj as u8);
                    }
                }
            }
        }
    }
}

fn playout(
    state: &State,
    first_move: (u8, u8),
    depth: usize,
    values: &[[u16; N]; N],
    rng: &mut SmallRng,
) -> f64 {
    let mut s = *state;
    let mut move_buffer = [(0u8, 0u8); 100];
    let mut turn_moves = [(0u8, 0u8); MAX_PLAYERS];

    for d in 0..depth {
        for p in 0..s.player_num {
            if d == 0 && p == 0 {
                turn_moves[p] = first_move;
            } else {
                let count = s.get_possible_moves(p, &mut move_buffer);
                // countは必ず1以上（現在地が含まれるため）
                turn_moves[p] = move_buffer[rng.random_range(0..count)];
            }
        }
        s.advance(&turn_moves);
    }
    s.evaluate(values)
}

fn main() {
    let mut source = LineSource::new(std::io::stdin().lock());
    input! {
        from &mut source,
        _n: usize, m: usize, _t: usize, l: usize,
        v: [[u16; N]; N],
        sxym: [(u8, u8); m]
    }

    let mut values = [[0u16; N]; N];
    for i in 0..N {
        for j in 0..N {
            values[i][j] = v[i][j];
        }
    }

    let mut owners = [[CELL_NO_OWNER; N]; N];
    let mut levels = [[0u8; N]; N];
    let mut pos = [(0u8, 0u8); MAX_PLAYERS];

    for (i, &(r, c)) in sxym.iter().enumerate() {
        if i < MAX_PLAYERS {
            owners[r as usize][c as usize] = i as i8;
            levels[r as usize][c as usize] = 1;
            pos[i] = (r, c);
        }
    }

    let mut state = State {
        owners,
        levels,
        pos,
        player_num: m,
        max_level: l as u8,
    };
    let mut rng = SmallRng::seed_from_u64(42);

    for _ in 0..T_MAX {
        let start_time = Instant::now();
        let mut move_candidates_pre = [(0u8, 0u8); 100];
        let n_moves_pre = state.get_possible_moves(0, &mut move_candidates_pre);

        let mut move_candidates = [(0u8, 0u8); 100];
        let mut n_moves = 0;
        for i in 0..n_moves_pre {
            let mi = move_candidates_pre[i].0 as usize;
            let mj = move_candidates_pre[i].1 as usize;
            if !(state.owners[mi][mj] == 0 && state.levels[mi][mj] == state.max_level) {
                move_candidates[n_moves] = move_candidates_pre[i];
                n_moves += 1;
            }
        }
        if n_moves == 0 {
            move_candidates = move_candidates_pre;
            n_moves = n_moves_pre;
        }

        let mut scores = vec![0.0; n_moves];
        let mut tries = vec![0; n_moves];
        let mut m_idx = 0;

        // 18ms ギリギリまで回す
        while start_time.elapsed() < Duration::from_millis(TIME_LIMIT_MS) {
            scores[m_idx] += playout(&state, move_candidates[m_idx], MC_DEPTH, &values, &mut rng);
            tries[m_idx] += 1;
            m_idx = (m_idx + 1) % n_moves;
        }

        let mut best_move = move_candidates[0];
        let mut max_avg = -1.0;
        for i in 0..n_moves {
            if tries[i] > 0 {
                let avg = scores[i] / tries[i] as f64;
                if avg > max_avg {
                    max_avg = avg;
                    best_move = move_candidates[i];
                }
            }
        }

        println!("{} {}", best_move.0, best_move.1);
        stdout().flush().unwrap();

        input! {
            from &mut source,
            _txym: [(u8, u8); m],
            pp: [(u8, u8); m],
            cur_o: [[i8; N]; N],
            cur_l: [[u8; N]; N]
        }
        // Stateの同期
        for i in 0..m {
            state.pos[i] = pp[i];
        }
        for r in 0..N {
            for c in 0..N {
                state.owners[r][c] = cur_o[r][c];
                state.levels[r][c] = cur_l[r][c];
            }
        }
    }
}
