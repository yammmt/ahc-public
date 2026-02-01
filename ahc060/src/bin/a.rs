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
    cycles: &mut Vec<Vec<(usize, Vec<isize>)>>,
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
            let mut path = vec![vcur as isize];
            let mut v = vcur;
            while let Some(vprev) = comes_from[v] {
                if vprev == v {
                    break;
                }

                path.push(vprev as isize);
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
fn calc_score(ans: &Vec<isize>, n: usize, k: usize) -> usize {
    let mut is_white = vec![true; n];
    let mut icecreams = vec![HashSet::new(); k];

    let mut vcur = 0;
    let mut icecur = vec![];
    for &a in ans {
        if a == -1 {
            is_white[vcur] = false;
        } else {
            if a < k as isize {
                icecreams[a as usize].insert(icecur.clone());
                icecur.clear();
            } else {
                icecur.push(is_white[a as usize]);
            }
            vcur = a as usize;
        }
    }

    let mut ret = 0;
    icecreams.iter().for_each(|a| ret += a.len());

    ret
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

    let mut cycles: Vec<Vec<(usize, Vec<isize>)>> = vec![vec![]; k];
    for i in 0..k {
        bfs(i, &edges, &mut cycles, n, k);
    }
    // println!("{:?}", cycles[0]);
    // return;

    // 初期解の生成
    // それぞれのショップ到達ごとに次に向かうショップをローテーションするだけ
    let mut ans: Vec<isize> = Vec::with_capacity(t);
    {
        let mut nth_visit = vec![0; k];
        let mut vcur = 0;
        loop {
            let mut cur_i = nth_visit[vcur] % cycles[vcur].len();
            let (mut vnext, mut pathcur) = cycles[vcur][cur_i].clone();
            // 前回パスに戻る経路は禁止
            // pathcur[0] は次に向かう最初の頂点、ans[ans.len() - 2] は直前に来た頂点
            // 全ての経路が禁止パスの場合は無限ループを避けるためカウンタで制限
            let mut skip_count = 0;
            while ans.len() >= 2
                && pathcur[0] as isize == ans[ans.len() - 2]
                && skip_count < cycles[vcur].len()
            {
                // eprintln!(
                //     "DEBUG: skipping path. vcur={}, pathcur[0]={}, ans[-2]={}",
                //     vcur,
                //     pathcur[0],
                //     ans[ans.len() - 2]
                // );
                nth_visit[vcur] += 1;
                cur_i = nth_visit[vcur] % cycles[vcur].len();
                (vnext, pathcur) = cycles[vcur][cur_i].clone();
                skip_count += 1;
            }

            if ans.len() + pathcur.len() >= t {
                break;
            }

            pathcur.iter().for_each(|p| ans.push(*p as isize));

            nth_visit[vcur] += 1;
            vcur = vnext;
        }
    }
    let mut best_score = calc_score(&ans, n, k);
    // eprintln!("score: {best_score}");

    // 乱択
    while start_time.elapsed() < break_time {
        break;

        let mut cur = vec![];

        // TODO

        let cur_score = calc_score(&cur, n, k);
        if cur_score > best_score {
            best_score = cur_score;
            ans = cur;
        }
    }

    for a in ans {
        println!("{a}");
    }
}
