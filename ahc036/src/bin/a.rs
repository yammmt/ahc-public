// use ordered_float::NotNan;
use proconio::fastout;
use proconio::input;
// use rand::rngs::SmallRng;
// use rand::{Rng, SeedableRng};
// use std::cmp::Ordering;
// use std::cmp::Reverse;
// use std::collections::BinaryHeap;
use std::collections::HashSet;
// use std::collections::HashMap;
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

    // 雑に最短経路を進むだけの愚直解, 通過先が青か否かの判定だけはする
    // 信号操作用の配列 A
    let mut a = vec![0; la];
    for i in 0..n {
        a[i] = i;
    }
    for (i, a) in a.iter().enumerate() {
        print!("{a}");
        if i == la - 1 {
            println!();
        } else {
            print!(" ");
        }
    }
    // 青信号管理用の配列 B, 問題文準拠の初期値 -1 だと型変換が面倒だから別のダミー値にする
    // 信号操作が一つずつだからスコアは非常に悪い
    let mut b = vec![DUMMY; lb];
    let mut b_idx = 0;
    let mut is_open = vec![false; n];
    let mut operation_cnt = 0;
    for path_cur in paths {
        for &v_nxt in &path_cur {
            if is_open[v_nxt] {
                println!("m {v_nxt}");
                operation_cnt += 1;
            } else {
                if b[b_idx] != DUMMY {
                    is_open[b[b_idx]] = false;
                }
                is_open[v_nxt] = true;
                b[b_idx] = v_nxt;
                println!("s 1 {v_nxt} {b_idx}");
                b_idx = (b_idx + 1) % lb;
                println!("m {v_nxt}");
                operation_cnt += 2;
            }
            debug_assert!(operation_cnt <= OPERATION_CNT_LIMIT);
        }
    }
}
