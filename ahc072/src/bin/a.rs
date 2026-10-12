use proconio::input;
use proconio::marker::Chars;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;

const CELL_MAX: usize = 400;
const HEIGHT_MAX: usize = 8;
/// 挿し直しの山登りを打ち切る時刻 [秒]. 制限時間は 2 秒
const TIME_LIMIT_SEC: f64 = 1.8;
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

/// マスごとの塔の履歴 (挿入中のスライム s を除く).
/// マス c の塔は, 時刻 starts[c][k] から starts[c][k + 1] の直前まで towers[c][k] (高さ heights[c][k]) である.
/// この時間帯を「区間 k」と呼ぶ. 区間内ではマス c の塔は変わらない.
struct Timeline {
    starts: Vec<Vec<usize>>,
    heights: Vec<Vec<usize>>,
    /// 塔の色の並び (下から順)
    towers: Vec<Vec<Vec<char>>>,
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
        let mut history: Vec<Vec<Vec<char>>> =
            towers.iter().map(|tower| vec![tower.clone()]).collect();

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
                history[c].push(towers[c].clone());
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
            towers: history,
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

    /// マス c の区間 k の塔で, s の下に z 匹いるとき, s を含む色 `color` の連続部分の範囲 (lo, hi).
    /// 同色のスライムは区別できないので, s の下の匹数を lo..=hi のどれにしても塔の色の並びは同じになる
    fn same_color_run(&self, c: usize, k: usize, z: usize, color: char) -> (usize, usize) {
        let tower = &self.towers[c][k];
        let mut lo = z;
        while lo > 0 && tower[lo - 1] == color {
            lo -= 1;
        }
        let mut hi = z;
        while hi < tower.len() && tower[hi] == color {
            hi += 1;
        }
        (lo, hi)
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

/// ラベルの間の遷移
#[derive(Debug, Clone, Copy)]
enum Move {
    /// 自分で跳ぶ (方向, 距離). コスト 1
    Jump(Dir, usize),
    /// 操作 t で跳ぶ一団の下に残る. コスト 0. 操作 t の leave_num を +1 する
    Stay(usize),
    /// 既存の操作で跳ぶ一団と一緒に跳ぶ (相乗り). コスト 0. 操作列は変えない
    Ride,
    /// 既存の操作で跳んだ一団が, s の塔の上に着地する. コスト 0. 操作列は変えない
    LandedOn,
}

/// 状態への 1 回の到着.
/// 同じ状態でも, 着いた時刻が早いほどその後に跳べる時刻の選択肢が多い.
#[derive(Debug, Clone, Copy)]
struct Label {
    state: usize,
    /// 済んだ予約の数
    r: usize,
    /// 着いた時刻 (跳躍なら, その跳躍を挿入する時刻)
    time: usize,
    /// 直前のラベルと, そこからの遷移. 始点は None
    prev: Option<(usize, Move)>,
}

/// (状態 `state`, 済んだ予約の数 `r`) に時刻 `time` で着くラベルを追加する.
/// 手数が同じか少ない到着より時刻が真に早いときだけ追加し, その番号を返す.
/// `best_time` は (状態, 予約の数) ごとの最も早い時刻で, 添字は state * r_len + r
fn push_label(
    labels: &mut Vec<Label>,
    best_time: &mut [Option<usize>],
    r_len: usize,
    (state, r): (usize, usize),
    time: usize,
    prev: (usize, Move),
) -> Option<usize> {
    let key = state * r_len + r;
    if best_time[key].is_some_and(|bt| bt <= time) {
        return None;
    }
    best_time[key] = Some(time);
    labels.push(Label {
        state,
        r,
        time,
        prev: Some(prev),
    });
    Some(labels.len() - 1)
}

/// スライム `s` を操作列 `ret` に挿入する動きを, 追加手数が最小になるように求める.
/// 返り値は (挿入する跳躍の (時刻, 跳躍) の時系列順の列, leave_num を +1 する操作の番号の列).
///
/// 状態は (マス, 区間, z). z は s の下にいる匹数で, s を同色の連続部分の一番上とみなした値にそろえる.
/// 遷移は「待つ」「残る」「一緒に跳ぶ」「上に一団が着地」「自分で跳ぶ」の 5 種類.
/// 「待つ」は状態の中に含まれる.
/// 手数ごとに 1 段ずつ幅優先で広げる. コスト 0 の遷移 (「残る」など) の行き先は同じ段に加え,
/// その段の中で続けて広げる.
///
/// `reserved` (昇順) の各操作では, s が出発マスの切れ目より下に残って土台にならなければならない (予約).
/// 予約の進み具合も状態に含める. 巣に着けなければ None.
/// `rng` を渡すと, 同じ手数の経路のうちどれを返すかがランダムになる.
#[allow(clippy::too_many_arguments)]
fn insert_slime(
    n: usize,
    ret: &[Ans],
    tl: &Timeline,
    s: &Slime,
    nest: &[Option<char>],
    is_wall: &[bool],
    reserved: &[usize],
    mut rng: Option<&mut SmallRng>,
) -> Option<(Vec<(usize, Ans)>, Vec<usize>)> {
    let t_max = ret.len();
    let r_len = reserved.len() + 1;
    // best_time[(状態, 予約の数)]: 今の手数までで, そこに着いた最も早い時刻
    let mut best_time: Vec<Option<usize>> = vec![None; tl.total * Z_LEN * r_len];
    // best_time_next[(状態, 予約の数)]: 次の手数 (今 + 1) で, そこに着いた最も早い時刻.
    // 今の手数の到着より劣るとは限らないので, 今の手数の判定には使わない
    let mut best_time_next: Vec<Option<usize>> = vec![None; tl.total * Z_LEN * r_len];
    let mut labels: Vec<Label> = vec![];
    let mut dirs = Dir::ALL;
    if let Some(rng) = rng.as_deref_mut() {
        dirs.shuffle(rng);
    }

    // 状態の z は, s を同色の連続部分の一番上とみなした値にそろえる (正規化)
    let state_of = |c: usize, k: usize, z: usize| {
        let (_, hi) = tl.same_color_run(c, k, z, s.color);
        tl.state_id(c, k, hi)
    };

    let c0 = s.pos_1d(n);
    let start = state_of(c0, tl.seg_at(c0, 0), 0);
    best_time[start * r_len] = Some(0);
    labels.push(Label {
        state: start,
        r: 0,
        time: 0,
        prev: None,
    });
    // 今の手数で着いたラベルの番号. コスト 0 の遷移で段の途中にも増える
    let mut frontier = vec![0];

    let goal = 'search: loop {
        if let Some(rng) = rng.as_deref_mut() {
            frontier.shuffle(rng);
        }
        let mut next = vec![];
        let mut idx = 0;
        while idx < frontier.len() {
            let li = frontier[idx];
            idx += 1;
            let Label { state, r, time, .. } = labels[li];
            // 次の予約の操作. s はそれまでに出発マスへ行き, 切れ目より下に残らなければならない
            let reserve = reserved.get(r).copied();
            // z は同色の連続部分の一番上. s は連続部分の中の lo..=z のどこにいるとみなしてもよい
            let (c, k, z) = tl.decode(state);
            let (lo, _) = tl.same_color_run(c, k, z, s.color);
            let h = tl.heights[c][k];

            // s が自分の色の巣で一番上にいれば帰巣. 予約が残っていれば帰巣してはいけない
            if nest[c] == Some(s.color) && z == h {
                if reserve.is_none() {
                    break 'search li;
                }
                continue;
            }

            // マス c の塔が変わらない最後の時刻. 操作 t_end (t_end < t_max のとき) が c に触れる
            let t_end = tl.seg_end(c, k, t_max);
            // 操作 t_end で起きる遷移を使えるか. 予約の操作より後に進むことはできず,
            // 予約の操作そのものでは「残る」しか使えない
            let (use_end, r_after) = match reserve {
                None => (t_end < t_max, r),
                Some(e) if t_end < e => (true, r),
                Some(e) if t_end == e => (true, r + 1),
                Some(_) => (false, r),
            };
            let is_reserved_op = r_after != r;

            // 待つ: コスト 0
            // 区間の中 (time..=t_end) ではマス c の塔は変わらないので, 待つことは状態に含まれる.
            // 遷移としては書かず, 「自分で跳ぶ」で跳ぶ時刻を選ぶことで表す.

            // 残る: コスト 0
            // 操作 t_end が c から跳び, s が切れ目より下 (z <= leave_num) にいる.
            // 同色の連続部分の一番下 lo が切れ目より下なら, s はそこにいるとみなして残れる.
            // 区間 k + 1 の同じ z へ. 経路復元で ret[t_end].leave_num を +1 する
            if use_end {
                let a = &ret[t_end];
                let (c_from, _) = a.from_to(n);
                if c_from == c && lo <= a.leave_num {
                    // 連続部分のうち, 切れ目より下で最も上の位置に s がいるとみなす
                    let z_stay = z.min(a.leave_num);
                    let h_after = tl.heights[c][k + 1];
                    // 操作の後, s の下にいるはずのスライムが帰巣済み (z > h_after) なら,
                    // 実際には s が上にいて帰巣できない. ただし s が巣と同じ色なら s ごと帰巣するので,
                    // s が一番上にいる (= 帰巣する) 状態として扱う
                    let z_after = if z_stay <= h_after {
                        Some(z_stay)
                    } else if nest[c] == Some(s.color) {
                        Some(h_after)
                    } else {
                        None
                    };
                    if let Some(z_after) = z_after {
                        let ns = state_of(c, k + 1, z_after);
                        if let Some(nl) = push_label(
                            &mut labels,
                            &mut best_time,
                            r_len,
                            (ns, r_after),
                            t_end + 1,
                            (li, Move::Stay(t_end)),
                        ) {
                            frontier.push(nl);
                        }
                    }
                }
            }

            // 一緒に跳ぶ: コスト 0
            // 操作 t_end が c から跳び, s が切れ目より上 (z >= leave_num) にいる.
            // 同色の連続部分の一番上 z が切れ目より上なら, s はそこにいるとみなして跳べる.
            // 着地点の, 操作 t_end の後の区間へ. 一団は反転して積まれる
            if use_end && !is_reserved_op {
                let a = &ret[t_end];
                let (c_from, c_to) = a.from_to(n);
                // 着地点の, 操作 t_end の直前と直後の区間
                let k_to = tl.seg_at(c_to, t_end);
                let g = tl.heights[c_to][k_to];
                // s が加わっても, 着地後 (帰巣前) の高さが上限以内
                if c_from == c && z >= a.leave_num && g + (h - a.leave_num) + 1 <= HEIGHT_MAX {
                    // 連続部分のうち, 切れ目より上で最も下の位置に s がいるとみなす
                    let z_ride = lo.max(a.leave_num);
                    // 一団は反転するので, s より上にいた h - z_ride 匹が s の下に来る
                    let z_landed = g + (h - z_ride);
                    let h_after = tl.heights[c_to][k_to + 1];
                    // 「残る」と同じく, s の下にいるはずのスライムが帰巣済みなら,
                    // s が巣と同じ色のときだけ s ごと帰巣したものとして扱う
                    let z_after = if z_landed <= h_after {
                        Some(z_landed)
                    } else if nest[c_to] == Some(s.color) {
                        Some(h_after)
                    } else {
                        None
                    };
                    if let Some(z_after) = z_after {
                        let ns = state_of(c_to, k_to + 1, z_after);
                        if let Some(nl) = push_label(
                            &mut labels,
                            &mut best_time,
                            r_len,
                            (ns, r),
                            t_end + 1,
                            (li, Move::Ride),
                        ) {
                            frontier.push(nl);
                        }
                    }
                }
            }

            // 上に一団が着地: コスト 0
            // 操作 t_end が c に着地する. s の下の匹数 z は変わらず, 区間 k + 1 へ
            if use_end && !is_reserved_op {
                let a = &ret[t_end];
                let (c_from, c_to) = a.from_to(n);
                // 跳んでくる一団の匹数
                let group = tl.heights[c_from][tl.seg_at(c_from, t_end)] - a.leave_num;
                // s が加わっても, 着地後 (帰巣前) の高さが上限以内
                if c_to == c && h + group + 1 <= HEIGHT_MAX {
                    let h_after = tl.heights[c][k + 1];
                    // 「残る」と同じく, s の下にいるはずのスライムが帰巣済みなら,
                    // s が巣と同じ色のときだけ s ごと帰巣したものとして扱う
                    let z_after = if z <= h_after {
                        Some(z)
                    } else if nest[c] == Some(s.color) {
                        Some(h_after)
                    } else {
                        None
                    };
                    if let Some(z_after) = z_after {
                        let ns = state_of(c, k + 1, z_after);
                        if let Some(nl) = push_label(
                            &mut labels,
                            &mut best_time,
                            r_len,
                            (ns, r),
                            t_end + 1,
                            (li, Move::LandedOn),
                        ) {
                            frontier.push(nl);
                        }
                    }
                }
            }

            // 自分で跳ぶ: コスト +1
            // 一番上にいるときだけ, 区間の中ならいつでも跳べる.
            // 同色の連続部分が塔の一番上まで続いていれば, 一番上の同色と役割を交換したとみなして跳べる
            // 予約があれば, 跳べるのは予約の操作の直前まで
            let t_jump_end = reserve.map_or(t_end, |e| t_end.min(e));
            if z == h {
                for d in dirs {
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
                                let ns = state_of(c_next, k_next, g);
                                // 今の手数までの到着より時刻が真に早いときだけ, 次の手数の候補にする
                                if best_time[ns * r_len + r].map_or(true, |bt| tau < bt) {
                                    if let Some(nl) = push_label(
                                        &mut labels,
                                        &mut best_time_next,
                                        r_len,
                                        (ns, r),
                                        tau,
                                        (li, Move::Jump(d, l)),
                                    ) {
                                        next.push(nl);
                                    }
                                }
                            }
                            k_next += 1;
                            if k_next >= tl.starts[c_next].len()
                                || tl.starts[c_next][k_next] > t_jump_end
                            {
                                break;
                            }
                            tau = tl.starts[c_next][k_next];
                        }
                    }
                }
            }
        }
        // 次の手数のラベルを確定する. 同じ状態により早く着くもの (次の手数でも, 今の手数でも) があれば除く
        let key = |l: &Label| l.state * r_len + l.r;
        let mut kept = vec![];
        for &nl in &next {
            let l = &labels[nl];
            if best_time_next[key(l)] == Some(l.time)
                && best_time[key(l)].map_or(true, |bt| l.time < bt)
            {
                kept.push(nl);
            }
        }
        for &nl in &next {
            best_time_next[key(&labels[nl])] = None;
        }
        for &nl in &kept {
            best_time[key(&labels[nl])] = Some(labels[nl].time);
        }
        if kept.is_empty() {
            // s が巣に着けない (予約を守れない)
            return None;
        }
        frontier = kept;
    };

    // 今回のスライムの辿った動きを復元
    let mut jumps = vec![];
    let mut stays = vec![];
    let mut li = goal;
    while let Some((prev, mv)) = labels[li].prev {
        match mv {
            Move::Jump(d, l) => {
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
            }
            Move::Stay(t) => stays.push(t),
            // 既存の操作に乗るだけなので, 操作列は変えない
            Move::Ride | Move::LandedOn => {}
        }
        li = prev;
    }
    jumps.reverse();
    Some((jumps, stays))
}

/// 操作列 `ret` に, 挿入したスライムの跳躍 `jumps` (時刻の昇順) と, 残す匹数を増やす操作 `stays` を反映する
fn apply_insertion(mut ret: Vec<Ans>, jumps: &[(usize, Ans)], stays: &[usize]) -> Vec<Ans> {
    // s が残る操作は, 残す匹数が s の分だけ増える
    for &t in stays {
        ret[t].leave_num += 1;
    }

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
    ret_new
}

/// 操作列 `ret` から, スライム `slimes[y]` の動きを取り除く.
/// 返り値は (y を除いた操作列, y が土台として必要な操作の番号の昇順の列 (予約)).
/// y を除くと動かせなくなる操作があれば None.
fn remove_slime(
    n: usize,
    ret: &[Ans],
    slimes: &[Slime],
    y: usize,
    nest: &[Option<char>],
) -> Option<(Vec<Ans>, Vec<usize>)> {
    // 塔をスライムの番号で持ち, y がどの操作でどこにいるかを追う
    let mut towers: Vec<Vec<usize>> = vec![vec![]; n * n];
    for (id, ss) in slimes.iter().enumerate() {
        towers[ss.pos_1d(n)].push(id);
    }
    let mut new_ops = Vec::with_capacity(ret.len());
    let mut reserved = vec![];
    for a in ret {
        let (c_from, c_to) = a.from_to(n);
        let h = towers[c_from].len();
        let mut a_new = a.clone();
        let mut keep = true;
        match towers[c_from].iter().position(|&id| id == y) {
            // y が跳ぶ一団にいる. y だけの跳躍なら消す
            Some(p) if p >= a.leave_num => keep = h - a.leave_num > 1,
            // y が土台にいる. 残す匹数が 1 減り, 飛距離が足りなくなれば y を予約する
            Some(_) => {
                a_new.leave_num -= 1;
                if a_new.flying_dist > a_new.leave_num + 1 {
                    reserved.push(new_ops.len());
                }
            }
            None => {}
        }
        if keep {
            new_ops.push(a_new);
        }

        // y も含めて実際に動かす
        let mut group = towers[c_from].split_off(a.leave_num);
        group.reverse();
        towers[c_to].extend(group);
        for c in [c_from, c_to] {
            if let Some(nest_color) = nest[c] {
                while towers[c]
                    .last()
                    .is_some_and(|&id| slimes[id].color == nest_color)
                {
                    towers[c].pop();
                }
            }
        }
    }

    // y を除いた盤面で, 各操作が動かす一団が空にならないか (y がいないと帰巣が早まる場合など)
    let others: Vec<Slime> = (0..slimes.len())
        .filter(|&id| id != y)
        .map(|id| slimes[id].clone())
        .collect();
    let mut colors: Vec<Vec<char>> = vec![vec![]; n * n];
    for ss in &others {
        colors[ss.pos_1d(n)].push(ss.color);
    }
    for a in &new_ops {
        let (c_from, c_to) = a.from_to(n);
        if colors[c_from].len() <= a.leave_num {
            return None;
        }
        let mut group = colors[c_from].split_off(a.leave_num);
        group.reverse();
        colors[c_to].extend(group);
        for c in [c_from, c_to] {
            if let Some(nest_color) = nest[c] {
                while colors[c].last() == Some(&nest_color) {
                    colors[c].pop();
                }
            }
        }
    }
    Some((new_ops, reserved))
}

/// 操作列 `ret` がすべて合法で, 最後に全スライムが帰巣しているか
fn is_valid(
    n: usize,
    ret: &[Ans],
    slimes: &[Slime],
    nest: &[Option<char>],
    is_wall: &[bool],
) -> bool {
    let mut towers: Vec<Vec<char>> = vec![vec![]; n * n];
    for ss in slimes {
        towers[ss.pos_1d(n)].push(ss.color);
    }
    for a in ret {
        if a.pos.0 >= n || a.pos.1 >= n {
            return false;
        }
        let c_from = a.pos.0 * n + a.pos.1;
        let h = towers[c_from].len();
        if h == 0 || a.leave_num >= h || a.flying_dist == 0 || a.flying_dist > a.leave_num + 1 {
            return false;
        }
        // 途中のマスもすべて盤面内の床
        let mut c_to = c_from;
        for step in 1..=a.flying_dist {
            match move_cell(c_from, a.dir.delta(), step, n, is_wall) {
                Some(c) => c_to = c,
                None => return false,
            }
        }
        if towers[c_to].len() + (h - a.leave_num) > HEIGHT_MAX {
            return false;
        }
        let mut group = towers[c_from].split_off(a.leave_num);
        group.reverse();
        towers[c_to].extend(group);
        for c in [c_from, c_to] {
            if let Some(nest_color) = nest[c] {
                while towers[c].last() == Some(&nest_color) {
                    towers[c].pop();
                }
            }
        }
    }
    towers.iter().all(|tower| tower.is_empty())
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
        let (jumps, stays) = insert_slime(n, &ret, &tl, s, &nest, &is_wall, &[], None)
            .expect("予約がなければ必ず巣に着ける");
        ret = apply_insertion(ret, &jumps, &stays);
    }

    ret
}

/// 壁を避けて上下左右に 1 マスずつ歩いたときの, マス `start` から各マスへの最短距離.
/// 届かないマスは usize::MAX
fn walk_dist(start: usize, n: usize, is_wall: &[bool]) -> Vec<usize> {
    let mut dist = vec![usize::MAX; n * n];
    let mut queue = VecDeque::new();
    dist[start] = 0;
    queue.push_back(start);
    while let Some(c) = queue.pop_front() {
        for d in Dir::ALL {
            let Some(c_next) = move_cell(c, d.delta(), 1, n, is_wall) else {
                continue;
            };
            if dist[c_next] == usize::MAX {
                dist[c_next] = dist[c] + 1;
                queue.push_back(c_next);
            }
        }
    }
    dist
}

fn main() {
    let start_time = std::time::Instant::now();
    input! {
        n: usize,
        // k: スライムの色数
        _k: usize,
        rows: [Chars; n],
    }

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

    // スライムを自分の巣に近い順に挿入する.
    // 遠くのスライムほど, 先に決まった近いスライムの経路に相乗りしやすい
    let is_wall: Vec<bool> = (0..n * n).map(|c| rows[c / n][c % n] == '#').collect();
    let mut nest_dist = std::collections::HashMap::new();
    for c in 0..n * n {
        let ch = rows[c / n][c % n];
        if ('A'..='L').contains(&ch) {
            nest_dist.insert(ch.to_ascii_lowercase(), walk_dist(c, n, &is_wall));
        }
    }
    // 距離が同じなら, 左上から順のまま (安定ソート)
    slimes.sort_by_key(|s| nest_dist[&s.color][s.pos_1d(n)]);

    let mut best = jikukan_bfs(&slimes, &rows);

    // 挿し直しの山登り: ランダムな 1 匹を取り除いて最短の挿入で挿し直し, 操作数が増えなければ採用する
    let nest: Vec<Option<char>> = (0..n * n)
        .map(|c| {
            let ch = rows[c / n][c % n];
            ('A'..='L').contains(&ch).then(|| ch.to_ascii_lowercase())
        })
        .collect();
    let mut rng = SmallRng::seed_from_u64(0);
    let (mut iterations, mut improved, mut skipped) = (0, 0, 0);
    while !slimes.is_empty() && start_time.elapsed().as_secs_f64() < TIME_LIMIT_SEC {
        iterations += 1;
        let y = rng.random_range(0..slimes.len());
        let Some((ops, reserved)) = remove_slime(n, &best, &slimes, y, &nest) else {
            skipped += 1;
            continue;
        };
        let others: Vec<Slime> = (0..slimes.len())
            .filter(|&id| id != y)
            .map(|id| slimes[id].clone())
            .collect();
        let tl = Timeline::new(n, &ops, &others, &nest);
        let Some((jumps, stays)) = insert_slime(
            n,
            &ops,
            &tl,
            &slimes[y],
            &nest,
            &is_wall,
            &reserved,
            Some(&mut rng),
        ) else {
            skipped += 1;
            continue;
        };
        let cand = apply_insertion(ops, &jumps, &stays);
        if cand.len() <= best.len() && is_valid(n, &cand, &slimes, &nest, &is_wall) {
            if cand.len() < best.len() {
                improved += 1;
            }
            best = cand;
        }
    }
    eprintln!("reinsertion hill climbing: iterations {iterations}, improved {improved}, skipped {skipped}");

    for a in best {
        println!("{a}");
    }
}
