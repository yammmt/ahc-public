use proconio::input;
use proconio::marker::Chars;

const CELL_MAX: usize = 400;
const HEIGHT_MAX: usize = 8;
const TURN_MAX: usize = 100_000;
/// 状態の z (s の下にいる匹数) が取りうる値の数
const Z_LEN: usize = HEIGHT_MAX + 1;

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
/// この時間帯を「区間 k」と呼ぶ. 区間内ではマス c の塔は変わらない.
struct Timeline {
    starts: Vec<Vec<usize>>,
    heights: Vec<Vec<usize>>,
    /// 区間の通し番号 = offset[c] + k
    offset: Vec<usize>,
    /// 区間の総数
    total: usize,
    /// cell_of_seg[区間の通し番号] = マス
    cell_of_seg: Vec<usize>,
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

        let mut offset = vec![0; n * n];
        let mut cell_of_seg = vec![];
        for c in 0..n * n {
            offset[c] = cell_of_seg.len();
            for _ in 0..starts[c].len() {
                cell_of_seg.push(c);
            }
        }

        Timeline {
            starts,
            heights,
            offset,
            total: cell_of_seg.len(),
            cell_of_seg,
        }
    }

    /// 時刻 t (操作 t の直前) を含む, マス c の区間の番号
    fn seg_at(&self, c: usize, t: usize) -> usize {
        self.starts[c].partition_point(|&x| x <= t) - 1
    }

    /// マス c の区間 k の最後の時刻. 最後の区間は全操作の後 (t_max) まで続く.
    fn seg_end(&self, c: usize, k: usize, t_max: usize) -> usize {
        if k + 1 < self.starts[c].len() {
            self.starts[c][k + 1] - 1
        } else {
            t_max
        }
    }

    /// 状態 (マス c, 区間 k, s の下にいる匹数 z) の通し番号
    fn state_id(&self, c: usize, k: usize, z: usize) -> usize {
        (self.offset[c] + k) * Z_LEN + z
    }

    /// 状態の通し番号から (マス, 区間, z) に戻す
    fn decode(&self, state: usize) -> (usize, usize, usize) {
        let c = self.cell_of_seg[state / Z_LEN];
        (c, state / Z_LEN - self.offset[c], state % Z_LEN)
    }
}

/// 状態への 1 回の到着.
/// 同じ状態でも, 着いた時刻が早いほどその後に跳べる時刻の選択肢が多い.
#[derive(Debug, Clone, Copy)]
struct Label {
    state: usize,
    /// 着いた時刻 (= 直前の跳躍を挿入する時刻)
    time: usize,
    /// 直前のラベルと, そこからの跳躍 (方向, 距離). 始点は None
    prev: Option<(usize, Dir, usize)>,
}

/// スライム `s` を操作列 `ret` に挿入する跳躍を, 追加手数が最小になるように求める.
/// 返り値は (挿入する時刻, 跳躍) の時系列順の列.
///
/// 状態は (マス, 区間, z). 遷移は「待つ」「残る」「一緒に跳ぶ」「上に一団が着地」「自分で跳ぶ」の 5 種類で,
/// 今は「待つ」と「自分で跳ぶ」だけを実装している. 「待つ」は状態の中に含まれ, 辺になるのは
/// コスト 1 の「自分で跳ぶ」だけなので, 手数ごとに 1 段ずつ幅優先で広げる.
///
/// TODO の 3 つ (コスト 0) を足すときは, 同じ段の中でもラベルを広げる必要がある.
/// また, 経路復元で跳躍以外の遷移 (「残る」の leave_num +1) も扱えるよう, Label に遷移の種類を持たせる.
fn insert_slime(
    n: usize,
    ret: &[Ans],
    tl: &Timeline,
    s: &Slime,
    nest: &[Option<char>],
    is_wall: &[bool],
) -> Vec<(usize, Ans)> {
    let t_max = ret.len();
    // best_time[状態]: その状態に着いた最も早い時刻 (手数が少ない到着も含む)
    let mut best_time: Vec<Option<usize>> = vec![None; tl.total * Z_LEN];
    let mut labels: Vec<Label> = vec![];

    let c0 = s.pos_1d(n);
    let start = tl.state_id(c0, tl.seg_at(c0, 0), 0);
    best_time[start] = Some(0);
    labels.push(Label {
        state: start,
        time: 0,
        prev: None,
    });
    // 今の手数で着いたラベルの番号
    let mut frontier = vec![0];

    let goal = loop {
        // s が自分の色の巣で一番上にいれば帰巣
        let found = frontier.iter().find(|&&li| {
            let (c, k, z) = tl.decode(labels[li].state);
            nest[c] == Some(s.color) && z == tl.heights[c][k]
        });
        if let Some(&li) = found {
            break li;
        }

        let mut next = vec![];
        for &li in &frontier {
            let Label { state, time, .. } = labels[li];
            let (c, k, z) = tl.decode(state);
            let h = tl.heights[c][k];
            // マス c の塔が変わらない最後の時刻. 操作 t_end (t_end < t_max のとき) が c に触れる
            let t_end = tl.seg_end(c, k, t_max);

            // 待つ: コスト 0
            // 区間の中 (time..=t_end) ではマス c の塔は変わらないので, 待つことは状態に含まれる.
            // 遷移としては書かず, 「自分で跳ぶ」で跳ぶ時刻を選ぶことで表す.

            // TODO: 残る: コスト 0
            // 操作 t_end が c から跳び, s が切れ目より下 (z <= leave_num) にいる.
            // 区間 k + 1 の同じ z へ. 経路復元で ret[t_end].leave_num を +1 する必要がある

            // TODO: 一緒に跳ぶ: コスト 0
            // 操作 t_end が c から跳び, s が切れ目より上 (z >= leave_num) にいる.
            // 着地点の, 操作 t_end の後の区間へ. 一団は反転して積まれる

            // TODO: 上に一団が着地: コスト 0
            // 操作 t_end が c に着地する. s の下の匹数 z は変わらず, 区間 k + 1 へ

            // 自分で跳ぶ: コスト +1
            // 一番上にいるときだけ, 区間の中ならいつでも跳べる
            if z == h {
                for d in Dir::ALL {
                    for l in 1..=h + 1 {
                        let Some(c_next) = move_cell(c, d.delta(), l, n, is_wall) else {
                            break;
                        };
                        // 跳ぶ時刻の候補は, 今と, 着地点の塔が変わった直後だけでよい
                        let mut k_next = tl.seg_at(c_next, time);
                        let mut tau = time;
                        loop {
                            let g = tl.heights[c_next][k_next];
                            if g + 1 <= HEIGHT_MAX {
                                let ns = tl.state_id(c_next, k_next, g);
                                // 手数が同じか少ない到着より, 時刻が真に早いときだけ残す
                                if best_time[ns].map_or(true, |bt| tau < bt) {
                                    best_time[ns] = Some(tau);
                                    labels.push(Label {
                                        state: ns,
                                        time: tau,
                                        prev: Some((li, d, l)),
                                    });
                                    next.push(labels.len() - 1);
                                }
                            }
                            k_next += 1;
                            if k_next >= tl.starts[c_next].len()
                                || tl.starts[c_next][k_next] > t_end
                            {
                                break;
                            }
                            tau = tl.starts[c_next][k_next];
                        }
                    }
                }
            }
        }
        assert!(!next.is_empty(), "s が巣に着けない");
        frontier = next;
    };

    // 今回のスライムの辿った跳躍を復元
    let mut jumps = vec![];
    let mut li = goal;
    while let Some((prev, d, l)) = labels[li].prev {
        let (c, _, z) = tl.decode(labels[prev].state);
        jumps.push((
            labels[li].time,
            Ans {
                pos: (c / n, c % n),
                // s の下の全員を土台として残す
                leave_num: z,
                dir: d,
                flying_dist: l,
            },
        ));
        li = prev;
    }
    jumps.reverse();
    jumps
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
        // 現在の操作列による, マスごとの塔の高さの履歴
        let tl = Timeline::new(n, &ret, &slimes[..i], &nest);
        let jumps = insert_slime(n, &ret, &tl, s, &nest, &is_wall);

        // 時刻 t の跳躍を ret[t] の直前に挿入する
        let mut ret_new = Vec::with_capacity(ret.len() + jumps.len());
        let mut p = 0;
        for t in 0..=ret.len() {
            while p < jumps.len() && jumps[p].0 == t {
                ret_new.push(jumps[p].1.clone());
                p += 1;
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
