use proconio::fastout;
use proconio::input;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

// 2 s
const TIME_LIMIT_MS: u64 = 1980;

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

        loop {
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

                    while let Some((v, path_so_far)) = que_detour.pop_front() {
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
    let mut best_score = calc_score(&ans, n, k);
    let mut best_ans = ans.clone();
    // eprintln!("score: {best_score}");

    // 各木の頂点について、訪問するインデックスのリストを収集
    let mut visit_indices: Vec<Vec<usize>> = vec![vec![]; n];
    for (i, &(a, _)) in ans.iter().enumerate() {
        let v = a as usize;
        if v >= k {
            visit_indices[v].push(i);
        }
    }

    // 複数回訪問する木の頂点のリスト
    let multi_visit_vertices: Vec<usize> = (k..n).filter(|&v| visit_indices[v].len() > 1).collect();

    // 山登り法
    while start_time.elapsed() < break_time {
        // 近傍操作を選択
        let op = rng.gen_range(0..2);

        match op {
            0 => {
                // 操作1: ランダムな訪問の塗り替えフラグを反転
                // ただし、その頂点で既に別の訪問で塗り替えている場合は、そちらをOFFにする
                let all_tree_visits: Vec<usize> = ans
                    .iter()
                    .enumerate()
                    .filter(|(_, (a, _))| (*a as usize) >= k)
                    .map(|(i, _)| i)
                    .collect();

                if all_tree_visits.is_empty() {
                    continue;
                }

                let idx = all_tree_visits[rng.gen_range(0..all_tree_visits.len())];
                let v = ans[idx].0 as usize;

                if ans[idx].1 {
                    // 塗り替えをOFFにする
                    ans[idx].1 = false;
                } else {
                    // 塗り替えをONにする前に、同じ頂点の他の訪問の塗り替えをOFFに
                    for &other_idx in &visit_indices[v] {
                        ans[other_idx].1 = false;
                    }
                    ans[idx].1 = true;
                }

                // ステップ数チェック
                if calc_steps(&ans, n, k) > t {
                    // 元に戻す
                    for &other_idx in &visit_indices[v] {
                        ans[other_idx].1 = best_ans[other_idx].1;
                    }
                    continue;
                }

                let cur_score = calc_score(&ans, n, k);
                if cur_score > best_score {
                    best_score = cur_score;
                    best_ans = ans.clone();
                } else {
                    // 元に戻す
                    for &other_idx in &visit_indices[v] {
                        ans[other_idx].1 = best_ans[other_idx].1;
                    }
                }
            }
            1 => {
                // 操作2: 複数回訪問する頂点で、塗り替えタイミングを変更
                if multi_visit_vertices.is_empty() {
                    continue;
                }

                let v = multi_visit_vertices[rng.gen_range(0..multi_visit_vertices.len())];
                let visits = &visit_indices[v];

                // 現在塗り替えが有効な訪問を探す
                let current_paint_idx = visits.iter().find(|&&i| ans[i].1).copied();

                // 新しい塗り替え位置をランダムに選択
                let new_idx = visits[rng.gen_range(0..visits.len())];

                // 全てOFFにしてから新しい位置をON
                for &i in visits {
                    ans[i].1 = false;
                }
                // 現在と同じ位置ならOFFのまま、違う位置ならON
                if current_paint_idx != Some(new_idx) {
                    ans[new_idx].1 = true;
                }

                // ステップ数チェック
                if calc_steps(&ans, n, k) > t {
                    for &i in visits {
                        ans[i].1 = best_ans[i].1;
                    }
                    continue;
                }

                let cur_score = calc_score(&ans, n, k);
                if cur_score > best_score {
                    best_score = cur_score;
                    best_ans = ans.clone();
                } else {
                    for &i in visits {
                        ans[i].1 = best_ans[i].1;
                    }
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
