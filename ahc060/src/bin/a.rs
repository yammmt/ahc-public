use proconio::fastout;
use proconio::input;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// 2 s
const TIME_LIMIT_MS: u64 = 1980;

/// セグメント: ショップからショップへの1区間
#[derive(Clone, Debug)]
struct Segment {
    from_shop: usize,        // 出発ショップ
    to_shop: usize,          // 到着ショップ
    path: Vec<usize>,        // 経由する頂点（木の頂点のみ、ショップは含まない）
    paint_at: Option<usize>, // この区間内で塗り替える位置（pathのインデックス）
}

/// 解をセグメントの列として管理
#[derive(Clone, Debug)]
struct Solution {
    segments: Vec<Segment>,
}

impl Solution {
    /// セグメント列からフラットな ans を生成
    fn to_ans(&self, n: usize) -> Vec<(isize, bool)> {
        let mut painted = vec![false; n];
        let mut ans = vec![];
        for seg in &self.segments {
            // 木の頂点を追加
            for (i, &v) in seg.path.iter().enumerate() {
                let do_paint = seg.paint_at == Some(i) && !painted[v];
                if do_paint {
                    painted[v] = true;
                }
                ans.push((v as isize, do_paint));
            }
            // 到着ショップを追加
            ans.push((seg.to_shop as isize, false));
        }
        ans
    }

    /// フラットな ans からセグメント列を構築
    fn from_ans(ans: &Vec<(isize, bool)>, k: usize) -> Self {
        let mut segments = vec![];
        let mut current_from = 0usize;
        let mut current_path = vec![];
        let mut current_paint_at = None;

        for (i, &(v, do_paint)) in ans.iter().enumerate() {
            let v = v as usize;
            if v < k {
                // ショップに到着
                segments.push(Segment {
                    from_shop: current_from,
                    to_shop: v,
                    path: current_path.clone(),
                    paint_at: current_paint_at,
                });
                current_from = v;
                current_path.clear();
                current_paint_at = None;
            } else {
                // 木の頂点
                if do_paint {
                    current_paint_at = Some(current_path.len());
                }
                current_path.push(v);
            }
        }

        Solution { segments }
    }

    /// 総ステップ数を計算
    fn calc_steps(&self, n: usize) -> usize {
        let mut painted = vec![false; n];
        let mut steps = 0;
        for seg in &self.segments {
            for (i, &v) in seg.path.iter().enumerate() {
                steps += 1;
                if seg.paint_at == Some(i) && !painted[v] {
                    steps += 1;
                    painted[v] = true;
                }
            }
            steps += 1; // ショップへの移動
        }
        steps
    }

    /// 接続が正しいかチェック
    fn is_connected(&self) -> bool {
        for i in 1..self.segments.len() {
            if self.segments[i - 1].to_shop != self.segments[i].from_shop {
                return false;
            }
        }
        true
    }

    /// 「前回移動元に戻れない」制約のチェック
    fn check_no_immediate_return(&self, k: usize) -> bool {
        for i in 1..self.segments.len() {
            let prev_seg = &self.segments[i - 1];
            let cur_seg = &self.segments[i];

            // 前セグメントの最後の頂点（到着ショップの直前）
            let prev_last = if prev_seg.path.is_empty() {
                prev_seg.from_shop
            } else {
                *prev_seg.path.last().unwrap()
            };

            // 現セグメントの最初の頂点
            let cur_first = if cur_seg.path.is_empty() {
                cur_seg.to_shop
            } else {
                cur_seg.path[0]
            };

            if cur_first == prev_last {
                return false;
            }
        }
        true
    }

    /// 全ての制約を満たすかチェック
    fn is_valid(&self, t: usize, n: usize, k: usize) -> bool {
        if self.segments.is_empty() {
            return true;
        }
        if self.segments[0].from_shop != 0 {
            return false;
        }
        if !self.is_connected() {
            return false;
        }
        if !self.check_no_immediate_return(k) {
            return false;
        }
        if self.calc_steps(n) > t {
            return false;
        }
        true
    }
}

/// ショップ間の最短移動経路を `cycles` に格納して返す.
/// 返す経路には始点は含まないが, 終点は含まれる.
fn bfs(
    vstart: usize,
    edges: &Vec<Vec<usize>>,
    cycles: &mut Vec<Vec<(usize, Vec<usize>)>>,
    n: usize,
    k: usize,
) {
    // 本来最短経路以外も考慮すべきではあるが, 実装すぐには思い浮かばないので妥協
    let mut comes_from = vec![None; n];
    let mut que = VecDeque::new();
    que.push_back((vstart, vstart));

    while let Some((vcur, vfrom)) = que.pop_front() {
        if comes_from[vcur].is_some() {
            continue;
        }

        comes_from[vcur] = Some(vfrom);
        if vcur != vstart && vcur < k {
            // goal
            let mut path = vec![vcur];
            let mut v = vcur;
            while let Some(vprev) = comes_from[v] {
                if vprev == v {
                    break;
                }

                path.push(vprev);
                v = vprev;
            }
            // 始点を除く
            path.pop();
            path.reverse();

            cycles[vstart].push((vcur, path));
        } else {
            for &vnext in &edges[vcur] {
                if comes_from[vnext].is_none() {
                    que.push_back((vnext, vcur));
                }
            }
        }
    }
}

/// スコアを返す. WA 判定はめんどいので略
/// 塗り替えは移動後に行うため、収穫してから塗り替える順序で処理
fn calc_score(ans: &Vec<(isize, bool)>, n: usize, k: usize) -> usize {
    let mut is_white = vec![true; n];
    let mut icecreams = vec![HashSet::new(); k];

    let mut icecur = vec![];
    for &(a, do_paint) in ans {
        let v = a as usize;
        if v < k {
            // ショップに到着：納品
            icecreams[v].insert(icecur.clone());
            icecur.clear();
        } else {
            // 木に到着：収穫（現在の色を取得）
            icecur.push(is_white[v]);
            // 塗り替え（行動2）は収穫後に行う
            if do_paint && is_white[v] {
                is_white[v] = false;
            }
        }
    }

    let mut ret = 0;
    icecreams.iter().for_each(|a| ret += a.len());

    ret
}

/// 総ステップ数を計算（移動 + 有効な塗り替え）
/// 同じ頂点での2回目以降の塗り替えはカウントしない
fn calc_steps(ans: &Vec<(isize, bool)>, n: usize, k: usize) -> usize {
    let mut painted = vec![false; n];
    let mut steps = 0;
    for &(a, do_paint) in ans {
        steps += 1; // 移動
        let v = a as usize;
        if v >= k && do_paint && !painted[v] {
            steps += 1; // 塗り替え（初回のみ）
            painted[v] = true;
        }
    }
    steps
}

fn calc_icecream(path: &Vec<usize>, is_white: &Vec<bool>) -> Vec<bool> {
    let mut ret = vec![];
    for p in path {
        ret.push(is_white[*p]);
    }
    ret
}

#[fastout]
fn main() {
    let start_time = Instant::now();
    let break_time = Duration::from_millis(TIME_LIMIT_MS);
    let mut rng = SmallRng::seed_from_u64(1);

    input! {
        n: usize,
        m: usize,
        k: usize,
        t: usize,
        abm: [(usize, usize); m],
        _xyn: [(usize, usize); n],
    }

    let mut edges = vec![vec![]; n];
    for (a, b) in abm {
        edges[a].push(b);
        edges[b].push(a);
    }
    let edges = edges;

    // 種類数がスコアになるので, あまり長く積むと時間の無駄になる
    // T=10,000 より最大行動回数は 10,000 回
    // ストロベリーへの味変で行動を消費するので, 味変も極力少ない方がよい
    //   味変なしである程度考えられる？
    // 頂点の座標値は使い所がない？どの辺も重みが同じ
    // 店起点に一歩進んで一歩戻るような遷移が制約上できない
    // 空集合納品でも得点が得られる
    // A. 貪欲やるなら, 今の店から最も近い店を計算して新味なら納品, を繰り返すとか
    //   N=100, M>=200 であり, 毎回 BFS しても間に合う…？ M の上限いくつだ
    // B. あるいは, 頂点を回る順番を固定して味で辻褄をあわせる
    //   これと味変入れて乱択すると多少はスコアが出そうだけれど, 工夫が弱い
    //   この頂点から来たなら次に向かえる頂点はこれ, の制約がある
    //   頂点というかサイクルで管理してやれば
    // C. 全頂点でオイラーツアーしながら一つずつ味変するとか
    //   頂点間の距離に依っては味変してもスコアが伸びなくなるし, それを引きそう
    // とりあえず B でやってみる

    let mut cycles: Vec<Vec<(usize, Vec<usize>)>> = vec![vec![]; k];
    for i in 0..k {
        bfs(i, &edges, &mut cycles, n, k);
    }
    // println!("{:?}", cycles[0]);
    // return;

    // 初期解の生成
    // それぞれのショップ到達ごとに次に向かうショップをローテーションするだけ
    let mut ans: Vec<(isize, bool)> = Vec::with_capacity(t);
    {
        let mut nth_visit = vec![0; k];
        let mut vcur = 0;
        // 直前に来た頂点を記録（最初は存在しない）
        let mut prev_vertex: Option<usize> = None;
        // 無限ループ防止用カウンタ
        let max_iterations = t * 2;
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > max_iterations || ans.len() >= t - 10 {
                break;
            }

            // 有効な経路を探す
            let mut found_valid = false;
            let mut vnext = 0;
            let mut pathcur = vec![];

            // 全ての到達可能なショップを試す
            for attempt in 0..cycles[vcur].len() {
                let cur_i = (nth_visit[vcur] + attempt) % cycles[vcur].len();
                let (to_shop, path) = &cycles[vcur][cur_i];

                // 前回の移動元に戻る経路は禁止
                let first_vertex = if path.is_empty() { *to_shop } else { path[0] };
                if let Some(prev) = prev_vertex {
                    if first_vertex == prev {
                        continue; // この経路は禁止
                    }
                }

                // 有効な経路を見つけた
                vnext = *to_shop;
                pathcur = path.clone();
                nth_visit[vcur] += attempt + 1;
                found_valid = true;
                break;
            }

            if !found_valid {
                // 全ての経路が禁止されている場合、隣接する木の頂点に一歩移動して回避
                // prev_vertex 以外の隣接頂点を探す
                let mut detour_vertex: Option<usize> = None;
                for &neighbor in &edges[vcur] {
                    if Some(neighbor) != prev_vertex {
                        detour_vertex = Some(neighbor);
                        break;
                    }
                }

                if let Some(dv) = detour_vertex {
                    // 迂回頂点に移動
                    ans.push((dv as isize, false));
                    prev_vertex = Some(vcur);

                    // 迂回頂点がショップの場合、そこを新しい現在地とする
                    if dv < k {
                        vcur = dv;
                        continue;
                    }

                    // 迂回頂点が木の場合、そこから最寄りのショップへ移動
                    // 簡易実装: 迂回頂点の隣接頂点でショップを探す
                    let mut found_shop = false;
                    for &neighbor2 in &edges[dv] {
                        if neighbor2 < k && neighbor2 != vcur {
                            ans.push((neighbor2 as isize, false));
                            prev_vertex = Some(dv);
                            vcur = neighbor2;
                            found_shop = true;
                            break;
                        }
                    }
                    if found_shop {
                        continue;
                    }

                    // 隣接にショップがない場合、さらに探索（BFS）
                    let mut visited_detour = vec![false; n];
                    let mut que_detour = VecDeque::new();
                    visited_detour[vcur] = true;
                    visited_detour[dv] = true;
                    que_detour.push_back((dv, vec![dv]));

                    let mut bfs_iterations = 0;
                    while let Some((v, path_so_far)) = que_detour.pop_front() {
                        bfs_iterations += 1;
                        if bfs_iterations > n * 2 {
                            break; // BFSの無限ループ防止
                        }
                        for &nv in &edges[v] {
                            if visited_detour[nv] {
                                continue;
                            }
                            visited_detour[nv] = true;
                            let mut new_path = path_so_far.clone();
                            new_path.push(nv);

                            if nv < k {
                                // ショップに到達
                                // path_so_far の最初の要素は既に追加済み（dv）
                                for i in 1..new_path.len() {
                                    ans.push((new_path[i] as isize, false));
                                }
                                prev_vertex = if new_path.len() >= 2 {
                                    Some(new_path[new_path.len() - 2])
                                } else {
                                    Some(dv)
                                };
                                vcur = nv;
                                found_shop = true;
                                break;
                            }
                            que_detour.push_back((nv, new_path));
                        }
                        if found_shop {
                            break;
                        }
                    }

                    if !found_shop {
                        break; // 本当にどこにも行けない場合は終了
                    }
                } else {
                    break; // 迂回先もない場合は終了
                }
                continue;
            }

            if ans.len() + pathcur.len() >= t - m + k {
                break;
            }

            // 経路を追加
            for &p in pathcur.iter() {
                ans.push((p as isize, false));
            }

            // 直前の頂点を更新: 到着ショップ(vnext)の直前にいた頂点
            // pathcur = [途中の木..., 到着ショップ] なので、
            // pathcur.len() >= 2 なら pathcur[len-2] が到着ショップの直前
            // pathcur.len() == 1 なら 現在のショップ(vcur)から直接到着ショップに行った
            if pathcur.len() >= 2 {
                prev_vertex = Some(pathcur[pathcur.len() - 2]);
            } else {
                // 2つのショップが直接繋がっている場合、直前は現在のショップ
                prev_vertex = Some(vcur);
            }

            vcur = vnext;
        }
    }

    // Solution 構造体を構築
    let mut solution = Solution::from_ans(&ans, k);
    let mut best_solution = solution.clone();

    let mut cur_score = calc_score(&ans, n, k);
    let mut best_score = cur_score;
    let mut best_ans = ans.clone();
    // eprintln!("score: {best_score}");

    // 焼きなまし法のパラメータ
    let start_temp: f64 = 5.0;
    let end_temp: f64 = 0.1;
    let time_limit = break_time.as_secs_f64();

    // 焼きなまし法
    while start_time.elapsed() < break_time {
        let elapsed = start_time.elapsed().as_secs_f64();
        let progress = elapsed / time_limit;
        let temp = start_temp * (end_temp / start_temp).powf(progress);

        // 近傍操作を選択
        // 0,1: 塗り替え操作、2,3,4: 訪問順操作
        // パス操作:塗替え をいい感じにする
        let op = if rng.random_bool(0.90) {
            // パス操作
            rng.random_range(2..5)
        } else {
            // 塗替え操作
            rng.random_range(0..2)
        };

        match op {
            0 => {
                // 操作0: ランダムな訪問の塗り替えフラグを反転
                if solution.segments.is_empty() {
                    continue;
                }

                // ランダムなセグメントを選択
                let seg_idx = rng.random_range(0..solution.segments.len());
                if solution.segments[seg_idx].path.is_empty() {
                    continue;
                }

                let old_paint = solution.segments[seg_idx].paint_at;
                let path_len = solution.segments[seg_idx].path.len();

                // 塗り替え位置を変更（None または ランダムな位置）
                let new_paint = if rng.random_bool(0.3) {
                    None
                } else {
                    Some(rng.random_range(0..path_len))
                };
                solution.segments[seg_idx].paint_at = new_paint;

                if !solution.is_valid(t, n, k) {
                    solution.segments[seg_idx].paint_at = old_paint;
                    continue;
                }

                let new_ans = solution.to_ans(n);
                let new_score = calc_score(&new_ans, n, k);
                let diff = new_score as f64 - cur_score as f64;
                let accept = diff > 0.0 || rng.random::<f64>() < (diff / temp).exp();

                if accept {
                    cur_score = new_score;
                    if new_score > best_score {
                        best_score = new_score;
                        best_ans = new_ans;
                        best_solution = solution.clone();
                    }
                } else {
                    solution.segments[seg_idx].paint_at = old_paint;
                }
            }
            1 => {
                // 操作1: 複数のセグメントで同じ頂点を通る場合、塗り替えタイミングを移動
                if solution.segments.is_empty() {
                    continue;
                }

                // ランダムなセグメントを選び、そのパス内の頂点を選ぶ
                let seg_idx = rng.random_range(0..solution.segments.len());
                if solution.segments[seg_idx].path.is_empty() {
                    continue;
                }

                let path_idx = rng.random_range(0..solution.segments[seg_idx].path.len());
                let target_v = solution.segments[seg_idx].path[path_idx];

                // 同じ頂点を含む他のセグメントを探す
                let mut candidates: Vec<(usize, usize)> = vec![];
                for (si, seg) in solution.segments.iter().enumerate() {
                    for (pi, &v) in seg.path.iter().enumerate() {
                        if v == target_v {
                            candidates.push((si, pi));
                        }
                    }
                }

                if candidates.len() <= 1 {
                    continue;
                }

                // 現在の塗り替え位置を保存
                let old_paints: Vec<Option<usize>> =
                    solution.segments.iter().map(|s| s.paint_at).collect();

                // 全セグメントでこの頂点の塗り替えをOFFに
                for &(si, _) in &candidates {
                    if let Some(paint_idx) = solution.segments[si].paint_at {
                        if solution.segments[si].path.get(paint_idx) == Some(&target_v) {
                            solution.segments[si].paint_at = None;
                        }
                    }
                }

                // ランダムな位置でONに
                let (new_si, new_pi) = candidates[rng.random_range(0..candidates.len())];
                solution.segments[new_si].paint_at = Some(new_pi);

                if !solution.is_valid(t, n, k) {
                    for (si, old_p) in old_paints.into_iter().enumerate() {
                        solution.segments[si].paint_at = old_p;
                    }
                    continue;
                }

                let new_ans = solution.to_ans(n);
                let new_score = calc_score(&new_ans, n, k);
                let diff = new_score as f64 - cur_score as f64;
                let accept = diff > 0.0 || rng.random::<f64>() < (diff / temp).exp();

                if accept {
                    cur_score = new_score;
                    if new_score > best_score {
                        best_score = new_score;
                        best_ans = new_ans;
                        best_solution = solution.clone();
                    }
                } else {
                    for (si, old_p) in old_paints.into_iter().enumerate() {
                        solution.segments[si].paint_at = old_p;
                    }
                }
            }
            2 => {
                // 操作2: 2つの隣接セグメントの中間ショップを変更
                if solution.segments.len() < 2 {
                    continue;
                }

                let idx = rng.random_range(0..solution.segments.len() - 1);
                let from_shop = solution.segments[idx].from_shop;
                let end_shop = solution.segments[idx + 1].to_shop;

                // 新しい中間ショップをランダムに選択
                let new_mid = rng.random_range(0..k);
                if new_mid == from_shop || new_mid == end_shop {
                    continue;
                }

                // cycles から経路を取得
                let path1_opt = cycles[from_shop]
                    .iter()
                    .find(|(to, _)| *to == new_mid)
                    .map(|(_, p)| p.clone());
                let path2_opt = cycles[new_mid]
                    .iter()
                    .find(|(to, _)| *to == end_shop)
                    .map(|(_, p)| p.clone());

                if path1_opt.is_none() || path2_opt.is_none() {
                    continue;
                }

                let path1 = path1_opt.unwrap();
                let path2 = path2_opt.unwrap();

                // 経路から到着ショップを除いた木の頂点のみを抽出
                let tree_path1: Vec<usize> = path1.iter().filter(|&&v| v >= k).cloned().collect();
                let tree_path2: Vec<usize> = path2.iter().filter(|&&v| v >= k).cloned().collect();

                let old_seg0 = solution.segments[idx].clone();
                let old_seg1 = solution.segments[idx + 1].clone();

                solution.segments[idx] = Segment {
                    from_shop: from_shop,
                    to_shop: new_mid,
                    path: tree_path1,
                    paint_at: None,
                };
                solution.segments[idx + 1] = Segment {
                    from_shop: new_mid,
                    to_shop: end_shop,
                    path: tree_path2,
                    paint_at: None,
                };

                if !solution.is_valid(t, n, k) {
                    solution.segments[idx] = old_seg0;
                    solution.segments[idx + 1] = old_seg1;
                    continue;
                }

                let new_ans = solution.to_ans(n);
                let new_score = calc_score(&new_ans, n, k);
                let diff = new_score as f64 - cur_score as f64;
                let accept = diff > 0.0 || rng.random::<f64>() < (diff / temp).exp();

                if accept {
                    cur_score = new_score;
                    if new_score > best_score {
                        best_score = new_score;
                        best_ans = new_ans;
                        best_solution = solution.clone();
                    }
                } else {
                    solution.segments[idx] = old_seg0;
                    solution.segments[idx + 1] = old_seg1;
                }
            }
            3 => {
                // 操作3: セグメントを削除（2つのセグメントを1つに統合）
                if solution.segments.len() < 2 {
                    continue;
                }

                let idx = rng.random_range(0..solution.segments.len() - 1);
                let from_shop = solution.segments[idx].from_shop;
                let end_shop = solution.segments[idx + 1].to_shop;

                // cycles から直接経路を取得
                let direct_path_opt = cycles[from_shop]
                    .iter()
                    .find(|(to, _)| *to == end_shop)
                    .map(|(_, p)| p.clone());

                if direct_path_opt.is_none() {
                    continue;
                }

                let direct_path = direct_path_opt.unwrap();
                let tree_path: Vec<usize> =
                    direct_path.iter().filter(|&&v| v >= k).cloned().collect();

                let old_seg0 = solution.segments[idx].clone();
                let old_seg1 = solution.segments[idx + 1].clone();

                solution.segments[idx] = Segment {
                    from_shop: from_shop,
                    to_shop: end_shop,
                    path: tree_path,
                    paint_at: None,
                };
                solution.segments.remove(idx + 1);

                if !solution.is_valid(t, n, k) {
                    solution.segments[idx] = old_seg0;
                    solution.segments.insert(idx + 1, old_seg1);
                    continue;
                }

                let new_ans = solution.to_ans(n);
                let new_score = calc_score(&new_ans, n, k);
                let diff = new_score as f64 - cur_score as f64;
                let accept = diff > 0.0 || rng.random::<f64>() < (diff / temp).exp();

                if accept {
                    cur_score = new_score;
                    if new_score > best_score {
                        best_score = new_score;
                        best_ans = new_ans;
                        best_solution = solution.clone();
                    }
                } else {
                    solution.segments[idx] = old_seg0;
                    solution.segments.insert(idx + 1, old_seg1);
                }
            }
            4 => {
                // 操作4: セグメントを挿入（1つのセグメントを2つに分割）
                if solution.segments.is_empty() {
                    continue;
                }

                let idx = rng.random_range(0..solution.segments.len());
                let from_shop = solution.segments[idx].from_shop;
                let to_shop = solution.segments[idx].to_shop;

                // 中間ショップをランダムに選択
                let mid_shop = rng.random_range(0..k);
                if mid_shop == from_shop || mid_shop == to_shop {
                    continue;
                }

                // cycles から経路を取得
                let path1_opt = cycles[from_shop]
                    .iter()
                    .find(|(to, _)| *to == mid_shop)
                    .map(|(_, p)| p.clone());
                let path2_opt = cycles[mid_shop]
                    .iter()
                    .find(|(to, _)| *to == to_shop)
                    .map(|(_, p)| p.clone());

                if path1_opt.is_none() || path2_opt.is_none() {
                    continue;
                }

                let path1 = path1_opt.unwrap();
                let path2 = path2_opt.unwrap();

                let tree_path1: Vec<usize> = path1.iter().filter(|&&v| v >= k).cloned().collect();
                let tree_path2: Vec<usize> = path2.iter().filter(|&&v| v >= k).cloned().collect();

                let old_seg = solution.segments[idx].clone();

                solution.segments[idx] = Segment {
                    from_shop: from_shop,
                    to_shop: mid_shop,
                    path: tree_path1,
                    paint_at: None,
                };
                solution.segments.insert(
                    idx + 1,
                    Segment {
                        from_shop: mid_shop,
                        to_shop: to_shop,
                        path: tree_path2,
                        paint_at: None,
                    },
                );

                if !solution.is_valid(t, n, k) {
                    solution.segments.remove(idx + 1);
                    solution.segments[idx] = old_seg;
                    continue;
                }

                let new_ans = solution.to_ans(n);
                let new_score = calc_score(&new_ans, n, k);
                let diff = new_score as f64 - cur_score as f64;
                let accept = diff > 0.0 || rng.random::<f64>() < (diff / temp).exp();

                if accept {
                    cur_score = new_score;
                    if new_score > best_score {
                        best_score = new_score;
                        best_ans = new_ans;
                        best_solution = solution.clone();
                    }
                } else {
                    solution.segments.remove(idx + 1);
                    solution.segments[idx] = old_seg;
                }
            }
            _ => unreachable!(),
        }
    }

    for a in best_ans {
        println!("{}", a.0);
        if a.1 {
            println!("-1");
        }
    }
}
