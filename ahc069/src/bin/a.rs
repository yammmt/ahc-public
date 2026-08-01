// use petgraph::unionfind::UnionFind;
use proconio::input;
use proconio::marker::Chars;
// use rand::rngs::SmallRng;
// use rand::{Rng, SeedableRng};
// use std::cmp::Ordering;
// use std::cmp::Reverse;
// use std::collections::BinaryHeap;
// use std::collections::BTreeSet;
// use std::collections::HashSet;
// use std::collections::HashMap;
// use std::collections::VecDeque;

// 盤面サイズ 50x50
const N: usize = 50;
// グループ数
const M: usize = 1000;

fn main() {
    input! {
        _n: usize,
        _m: usize,
        // 移動コスト係数, [0.001, 0.1] で小数点以下三桁
        r: f64,
        row: [Chars; N],
    }

    for _ in 0..M {
        input! {
            i: usize,
            s: usize,
            t: usize,
            // 割り当てるマス数, [4, 150]
            p: usize,
            // 取得金額係数, 移動時の減額にも用いられる, [0, 10^8]
            v: usize
        }
    }
}
