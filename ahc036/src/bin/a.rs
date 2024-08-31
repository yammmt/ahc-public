use proconio::fastout;
use proconio::input;
use rand::rngs::SmallRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use std::collections::HashSet;
use std::collections::VecDeque;

#[allow(unused_macros)]
macro_rules! debug {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        {
            println!($($arg)*);
        }
    };
}

const DUMMY: usize = usize::MAX / 4;
#[allow(dead_code)]
const OPERATION_CNT_LIMIT: usize = 100_000;
#[allow(dead_code)]
const RUN_TIME_LIMIT_MS: usize = 2800;

const B_UPDATE_BEGIN_PTRN_NUM: usize = 5;
const B_UPDATE_LEN_MAX: usize = 4;

// #[fastout]
fn main() {
    // 入力時点で 0-origin
    input! {
        n: usize,
        m: usize,
        t: usize,
        la: usize,
        lb: usize,
        uvm: [(usize, usize); m],
        tt: [usize; t],
        _xyn: [(isize, isize); n],
    }

    // TODO: 固定長だから初期化をちょっと高速にできる
    let mut edges = vec![vec![]; n];
    for (u, v) in uvm {
        edges[u].push(v);
        edges[v].push(u);
    }

    let mut rng = SmallRng::from_entropy();

    // なんもわからん 2024
    // 時間いっぱい L_A 乱択でビームサーチでそれっぽいスコアになりそうだけど
    // なんかそれだけだと寂しいのでもうひと工夫ほしい...

    // スコア: 信号を操作する回数が少ないほどよい
    // 雑に A を乱択 + ビームサーチ書くとそんな悪いスコアにはならなさそうな？
    // 乱択を考えなければ, これで一括操作が通り易くなる？:
    //   1. BFS で最短経路を求める
    //   1. 最短経路への登場回数降順に A の初期値を決める
    //   1. 1 ターンずつ通していく
    // 信号操作は 0-1 BFS に言い換えられる, あるいは A*?
    // サンプルのユークリッド距離近い側戦略はそんな悪くなさそうなんだよな, 直感的には
    // 移動先の距離が遠くなるなら探索打ち切ってもあまり悪くならない気がする

    // 都市数が 600, 道路数は [599, 1794]
    // 信号制御用配列の長さは [600, 1200]
    // 同時に青にできる信号の数は [4, 24]
    // 訪問都市数も 600 だが同じ都市が複数回含まれる可能性がある
    // - 信号制御用配列 L_A には同じ要素を複数回入れられる
    //   - が, 通過回数の多い信号を多くいれると, 却って赤になる期間が伸びてしまう可能性がある
    // - 毎ターン 0-1 BFS しようにもちょっと計算苦しめ？
    //   - L_A 固定であれば回る量ではありそう
    // 都市の始点が 600 個で街の数と道路の数の和が 2500 くらい
    // すると経路を BFS しても計算回数はワーストで 600x2500=1,500,000 だから間に合う
    // が, これに途中経路を愚直可変長配列保存すると遅い
    // サンプルのように再帰を使うか, もしくはどこから来たかを記録する, DP 経路復元相当

    // 乱択成分にしても, 直感的には L_A の寄与が大きそう, L_A について考える
    // - 長時間青にしたい要素は先頭か末尾に起きたい, 一度青にしてから放置するので
    // - 一度も通らない街は L_A に含む必要なし
    //   - "一度も通らない" の部分は遠回りを考慮するべきではある

    // 移動の回数と信号操作の回数は合わせて 10^5 回以下
    // 600 個の街を最長パスで移動すると制限にかかる
    // というより 600 回の旅行に毎度 300 回の移動をしてしまうと 600*300 = 180,000 > 10^5
    // これ制約きつくない？何か間違えている？
    // 道路の本数最小限で嫌な旅行計画が出てくると, どう操作しても間に合わなくなるのでは？

    // 何も考えずサンプルよりよいスコアを取ることを考える.
    // 旅行の経路を最短経路に固定して, L_A を弄る.
    // 最初は貪欲に経路を L_A に突っ込み, L_A の枠を食い終わったら諦めて一つずつ操作する.

    // 最短経路を先に求める
    // 経路の確定は後回しにする, アルゴリズムを分けられる
    // TODO: vec に一旦全部 push して sort して dedup して, の方が高速な可能性
    let mut edges_used = HashSet::new();
    let mut paths = vec![vec![]; t];
    let mut v_start = 0;
    for (i, &v_end) in tt.iter().enumerate() {
        let mut q = VecDeque::new();
        // TODO: 変数の個数を減らせるような気がする
        let mut visited = vec![false; n];
        let mut comes_from = vec![None; n];
        q.push_back(v_start);
        while let Some(v_cur) = q.pop_front() {
            if visited[v_cur] {
                continue;
            }

            visited[v_cur] = true;
            for &v_nxt in &edges[v_cur] {
                if v_nxt == v_start || comes_from[v_nxt].is_some() {
                    continue;
                }

                comes_from[v_nxt] = Some(v_cur);
                if v_nxt == v_end {
                    // ちょっと難読になるが, 素朴な BFS ならここで打ち切った方が高速
                    break;
                }

                q.push_back(v_nxt);
            }
        }

        let mut path_cur = vec![v_end];
        let mut v_cur = v_end;
        edges_used.insert(v_end);
        while let Some(v_prev) = comes_from[v_cur] {
            path_cur.push(v_prev);
            edges_used.insert(v_prev);
            v_cur = v_prev;
        }
        // `pop`: 始点の情報が重複することを防ぐため
        path_cur.pop();
        path_cur.reverse();
        paths[i] = path_cur;

        v_start = v_end;
    }
    debug!("paths: {:?}", paths);
    debug!("edges_used: {:?}", edges_used);
    let edges_used = edges_used.into_iter().collect::<Vec<usize>>();

    // 信号操作用の配列 A
    let mut a = vec![0; la];
    for i in 0..la {
        a[i] = if i < edges_used.len() {
            edges_used[i]
        } else {
            edges_used[rng.gen::<usize>() % edges_used.len()]
        };
    }
    a.shuffle(&mut rng);
    for (i, a) in a.iter().enumerate() {
        print!("{a}");
        if i == la - 1 {
            println!();
        } else {
            print!(" ");
        }
    }
    let mut v_to_a_idx = vec![vec![]; n];
    for (i, &a) in a.iter().enumerate() {
        v_to_a_idx[a].push(i);
    }

    // 青信号管理用の配列 B, 問題文準拠の初期値 -1 だと型変換が面倒だから別のダミー値にする
    let mut b = vec![DUMMY; lb];
    let mut b_idx = 0;
    // 一つの信号を重複してもったほうがよい可能性がある
    let mut open_cnt = vec![0; n];
    let mut operation_cnt = 0;
    for path_cur in paths {
        for (path_cur_idx, &v_nxt) in path_cur.iter().enumerate() {
            if open_cnt[v_nxt] > 0 {
                println!("m {v_nxt}");
                operation_cnt += 1;
            } else {
                // A の長さが最大 1200
                // B の始点/終点の選び方が最大 24*23=552
                // 愚直だと信号操作数/移動数が共に高々 10000 くらい？
                // A の始点を次に行くマスに固定すると計算が絞れる, 一旦 1 にする
                // 書き換える長さも高々 4 (L_B 下限) にしてしまえば,
                // 10000*4*24 程度の計算回数になるので TLE は回避できるはず
                // これに B の書き換え基準位置も 4 通りくらい見せて合計 4e6 くらい

                // 効率判定及び信号変化に必要な情報は
                // - A の使用開始地点
                // - B の書き換え開始地点
                // - 書き換えの長さ
                // - 今の最高スコア (直近の行動予定の内, 書き換え後に通れるマスの数)
                let mut a_idx_update = 0;
                let mut b_idx_update = 0;
                let mut update_len = 0;
                let mut score_max = 0;
                let mut v_nearby = HashSet::new();
                for i in 0..B_UPDATE_LEN_MAX {
                    let j = path_cur_idx + i;
                    if j >= path_cur.len() {
                        break;
                    }

                    v_nearby.insert(path_cur[j]);
                }
                let v_nearby = v_nearby;

                for idx_diff in 0..B_UPDATE_BEGIN_PTRN_NUM {
                    debug!("b_idx: {b_idx}, idx_diff: {idx_diff}");
                    let b_idx_begin = (lb + b_idx + idx_diff - B_UPDATE_BEGIN_PTRN_NUM / 2) % lb;
                    debug!("b_idx_begin: {b_idx_begin}");
                    for idx_from_begin in 0..B_UPDATE_LEN_MAX {
                        // additional_update_len + 1 個のマスを書き換える
                        if b_idx_begin + idx_from_begin >= lb {
                            continue;
                        }

                        for &a_idx_begin in &v_to_a_idx[v_nxt] {
                            if a_idx_begin + idx_from_begin >= la {
                                continue;
                            }

                            // TODO: 連続した書き換えでありループを跨いで高速化できる
                            let mut b_candidate = b.clone();
                            for i in 0..idx_from_begin + 1 {
                                b_candidate[b_idx_begin + i] = a[a_idx_begin + i];
                            }

                            let mut score_cur = 0;
                            for bb in &b_candidate {
                                if v_nearby.contains(bb) {
                                    score_cur += 1;
                                }
                            }
                            if score_cur > score_max {
                                a_idx_update = a_idx_begin;
                                b_idx_update = b_idx_begin;
                                update_len = idx_from_begin + 1;
                                score_max = score_cur;
                            }
                        }
                    }
                }


                for i in 0..update_len {
                    if b[b_idx_update + i] != DUMMY {
                        open_cnt[b[b_idx_update + i]] -= 1;
                    }

                    b[b_idx_update + i] = a[a_idx_update + i];
                    open_cnt[b[b_idx_update + i]] += 1;
                }
                b_idx = (b_idx + update_len + 1) % lb;

                println!("s {update_len} {a_idx_update} {b_idx_update}");
                b_idx = (b_idx + 1) % lb;
                println!("m {v_nxt}");
                operation_cnt += 2;
            }
            debug_assert!(operation_cnt <= OPERATION_CNT_LIMIT);
        }
    }
}
