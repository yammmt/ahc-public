use proconio::input;
use proconio::marker::Chars;
use std::collections::VecDeque;

const CELL_MAX: usize = 400;
const HEIGHT_MAX: usize = 8;
const TURN_MAX: usize = 100_000;

#[derive(Debug, Clone, Copy)]
enum Dir {
    U,
    D,
    L,
    R,
}

impl Dir {
    const ALL: [Dir; 4] = [Dir::U, Dir::D, Dir::L, Dir::R];

    fn delta(self) -> (isize, isize) {
        match self {
            Dir::U => (-1, 0),
            Dir::D => (1, 0),
            Dir::L => (0, -1),
            Dir::R => (0, 1),
        }
    }

    fn to_char(self) -> char {
        match self {
            Dir::U => 'U',
            Dir::D => 'D',
            Dir::L => 'L',
            Dir::R => 'R',
        }
    }

    fn from_delta(delta: (isize, isize)) -> Dir {
        *Dir::ALL.iter().find(|d| d.delta() == delta).unwrap()
    }
}

#[derive(Debug, Clone)]
struct Ans {
    pos: (usize, usize),
    leave_num: usize,
    dir: Dir,
    flying_dist: usize,
}

impl std::fmt::Display for Ans {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{} {} {} {} {}",
            self.pos.0,
            self.pos.1,
            self.leave_num,
            self.dir.to_char(),
            self.flying_dist
        )
    }
}

impl Ans {
    /// 盤面サイズ `n` を受け取り, 移動前後の一次元座標を返却する.
    fn from_to(&self, n: usize) -> (usize, usize) {
        let c_from = self.pos.0 * n + self.pos.1;
        let i = (self.pos.0 as isize + self.dir.delta().0 * self.flying_dist as isize) as usize;
        let j = (self.pos.1 as isize + self.dir.delta().1 * self.flying_dist as isize) as usize;
        (c_from, i * n + j)
    }

    /// マス `c_from` からマス `c_to` へ跳躍する. 下の `leave_num` 匹は残す.
    fn jump(c_from: usize, c_to: usize, leave_num: usize, n: usize) -> Self {
        let (di, dj) = (
            (c_to / n) as isize - (c_from / n) as isize,
            (c_to % n) as isize - (c_from % n) as isize,
        );
        Ans {
            pos: (c_from / n, c_from % n),
            leave_num,
            // TODO: 汚い
            dir: Dir::from_delta((di.signum(), dj.signum())),
            flying_dist: (di.abs() + dj.abs()) as usize,
        }
    }
}

#[derive(Debug, Clone)]
struct Slime {
    color: char,
    pos: (usize, usize),
}

impl Slime {
    fn pos_1d(&self, n: usize) -> usize {
        self.pos.0 * n + self.pos.1
    }
}

/// マス c から方向 dir に l マス進んだマス, 障害物に衝突あるいは盤面外なら None
fn move_cell(c: usize, dir: (isize, isize), l: usize, n: usize, is_wall: &[bool]) -> Option<usize> {
    let i = (c / n) as isize + dir.0 * l as isize;
    let j = (c % n) as isize + dir.1 * l as isize;
    if !(0 <= i && i < n as isize && 0 <= j && j < n as isize) {
        return None;
    }
    let c_next = i as usize * n + j as usize;
    if is_wall[c_next] {
        return None;
    }
    Some(c_next)
}

/// マスごとの塔の高さの履歴 (挿入中のスライム s を除く).
/// マス c の塔の高さは, 時刻 starts[c][k] から starts[c][k + 1] の直前まで heights[c][k] である.
struct Timeline {
    starts: Vec<Vec<usize>>,
    heights: Vec<Vec<usize>>,
}

impl Timeline {
    /// 挿入済みのスライム `placed` だけを置いた初期盤面から, 操作列 `ret` をシミュレーションする.
    fn new(n: usize, ret: &[Ans], placed: &[Slime], nest: &[Option<char>]) -> Self {
        let mut towers: Vec<Vec<char>> = vec![vec![]; n * n];
        for ss in placed {
            towers[ss.pos_1d(n)].push(ss.color);
        }
        let mut starts = vec![vec![0]; n * n];
        let mut heights: Vec<Vec<usize>> = towers.iter().map(|tower| vec![tower.len()]).collect();

        for (t, a) in ret.iter().enumerate() {
            let (c_from, c_to) = a.from_to(n);

            // 残す/跳ばす群を分離
            let mut group_jumping = towers[c_from].split_off(a.leave_num);
            // 跳ばす
            group_jumping.reverse();
            towers[c_to].extend(group_jumping);
            for c in [c_from, c_to] {
                // 帰巣
                if let Some(nest_color) = nest[c] {
                    while towers[c].last() == Some(&nest_color) {
                        towers[c].pop();
                    }
                }
                // 操作が触れたマスでは, 高さが変わらなくても履歴を区切る
                starts[c].push(t + 1);
                heights[c].push(towers[c].len());
            }
        }

        Timeline { starts, heights }
    }

    /// 時刻 t (操作 t の直前) を含む, マス c の履歴の番号
    fn seg_at(&self, c: usize, t: usize) -> usize {
        self.starts[c].partition_point(|&x| x <= t) - 1
    }

    /// 時刻 t (操作 t の直前) のマス c の塔の高さ
    fn height_at(&self, c: usize, t: usize) -> usize {
        self.heights[c][self.seg_at(c, t)]
    }
}

fn jikukan_bfs(slimes: &[Slime], map_default: &[Vec<char>]) -> Vec<Ans> {
    let n = map_default.len();
    let is_wall: Vec<bool> = (0..n * n)
        .map(|c| map_default[c / n][c % n] == '#')
        .collect();
    let nest: Vec<Option<char>> = (0..n * n)
        .map(|c| {
            let ch = map_default[c / n][c % n];
            if ('A'..='L').contains(&ch) {
                // スライムと合わせる
                Some(ch.to_ascii_lowercase())
            } else {
                None
            }
        })
        .collect();

    let mut ret: Vec<Ans> = vec![];

    for (i, s) in slimes.iter().enumerate() {
        // t 番目の操作の前に今回のスライムの動きを挿入
        // ターン最大値で宣言するとメモリが足りなくなる
        let t_len = ret.len() + 1;
        // cost[t][c][z]: ターン t 操作直前のマス c において下から z 匹目の上に s がいる場合の最小手数
        let mut costs = vec![vec![vec![None; HEIGHT_MAX + 1]; n * n]; t_len];
        // came_from[t][c][z]: ターン t 操作直前のマス c において z 匹目の上に s がいる場合の移動元
        // TODO: (usize, usize, usize) を型として切り出す
        let mut came_from: Vec<Vec<Vec<Option<(usize, usize, usize)>>>> =
            vec![vec![vec![None; HEIGHT_MAX + 1]; n * n]; t_len];

        // 現在の操作列による, マスごとの塔の高さの履歴
        let tl = Timeline::new(n, &ret, &slimes[..i], &nest);

        let mut deque = VecDeque::new();
        // (マス, コスト, 直前のマス)
        deque.push_back(((0, s.pos_1d(n), 0), 0, None));
        let mut goal = None;
        while let Some(((t_cur, c_cur, z_cur), cost_cur, from)) = deque.pop_front() {
            if costs[t_cur][c_cur][z_cur].is_some() {
                continue;
            }

            costs[t_cur][c_cur][z_cur] = Some(cost_cur);
            came_from[t_cur][c_cur][z_cur] = from;

            // s の帰巣が起きたなら終わり
            // 今回のスライムに関する経路のみを復元する
            if nest[c_cur] == Some(s.color) && z_cur == tl.height_at(c_cur, t_cur) {
                goal = Some((t_cur, c_cur, z_cur));
                break;
            }

            // TODO: (AI) 帰巣を妨げる遷移、高さ 8 を超える遷移は捨てる

            if t_cur < ret.len() {
                let a = &ret[t_cur];
                let (c_from, c_to) = a.from_to(n);

                // 待つ: コスト増えない
                // 操作 t がマス c を含むものであれば, 待つの選択肢は選べない
                if c_cur != c_from && c_cur != c_to {
                    deque.push_front((
                        (t_cur + 1, c_cur, z_cur),
                        cost_cur,
                        Some((t_cur, c_cur, z_cur)),
                    ));
                }

                // 残る: 自分より上が跳ぶ, コスト増えない (最小限実装ならなくても動く)
                // c が出発マスかつ s が切れ目より下 (= 残る側にいる)
                // if c_cur == c_from && z_cur <= a.leave_num && z_cur <= tl.height_at(c_cur, t_cur + 1) {
                //     deque.push_front((
                //         (t_cur + 1, c_cur, z_cur),
                //         cost_cur,
                //         Some((t_cur, c_cur, z_cur)),
                //     ));
                // }

                // TODO: 一緒に跳ぶ, コスト増えない (最小限実装ならなくても動く)

                // TODO: 上に一団が着地, コスト増えない (最小限実装ならなくても動く)
            }

            // 自身が一番上で跳ぶ, コスト +1
            let h = tl.height_at(c_cur, t_cur);
            if z_cur != h {
                // 自身が最上段にいない
                continue;
            }
            for d in Dir::ALL {
                for l in 1..=h + 1 {
                    let Some(c_next) = move_cell(c_cur, d.delta(), l, n, &is_wall) else {
                        break;
                    };
                    let g = tl.height_at(c_next, t_cur);
                    if g + 1 > HEIGHT_MAX {
                        // 高さ制限にかかって着地不可
                        continue;
                    }
                    deque.push_back((
                        (t_cur, c_next, g),
                        cost_cur + 1,
                        Some((t_cur, c_cur, z_cur)),
                    ));
                }
            }
        }

        // 今回のスライムの辿った操作を復元
        let mut cur = goal.unwrap();
        let mut paths = vec![cur];
        while let Some(prev) = came_from[cur.0][cur.1][cur.2] {
            paths.push(prev);
            cur = prev;
        }
        paths.reverse();

        // jumps_at[t]: 時刻 t に挿入する s の跳躍 (時刻 t の先頭に挿入)
        let mut jumps_at: Vec<Vec<Ans>> = vec![vec![]; t_len];
        for k in 0..paths.len() - 1 {
            let (t0, c0, _) = paths[k];
            let (t1, c1, _) = paths[k + 1];
            if t0 == t1 {
                // 挿入先が同じターンであれば自身で跳んでいる
                // 既存ターンを消費せずに自身の位置を変える操作は他にないので
                jumps_at[t0].push(Ans::jump(c0, c1, tl.height_at(c0, t0), n));
            }
        }

        let mut ret_new = vec![];
        for t in 0..t_len {
            for a in &jumps_at[t] {
                ret_new.push(a.clone());
            }
            if t < ret.len() {
                ret_new.push(ret[t].clone());
            }
        }
        ret = ret_new;
    }

    ret
}

fn main() {
    input! {
        n: usize,
        // k: スライムの色数
        _k: usize,
        rows: [Chars; n],
    }

    // TODO: スライムを巣に近い順に並べると相乗りが起き易くなる
    let mut slimes = Vec::with_capacity(CELL_MAX);
    for i in 0..n {
        for j in 0..n {
            if 'a' <= rows[i][j] && rows[i][j] <= 'l' {
                slimes.push(Slime {
                    color: rows[i][j],
                    pos: (i, j),
                });
            }
        }
    }

    let ans = jikukan_bfs(&slimes, &rows);

    for a in ans {
        println!("{a}");
    }
}
