// use itertools::Itertools;
// use permutohedron::heap_recursive;
// use petgraph::unionfind::UnionFind;
use proconio::fastout;
use proconio::input;
// use rand::rngs::SmallRng;
// use rand::{Rng, SeedableRng};
// use std::collections::BinaryHeap;
// use std::collections::HashSet;
// use std::collections::HashMap;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
// use superslice::Ext;

const TIME_LIMIT_MS: u64 = 1800;

const N: usize = 20;
const CARD_KIND_NUM: usize = 200;
const NO_CARD: usize = 9999;

const DUMMY: usize = 9999;

/// 範囲外判定はしない
fn next_pos_wo_check(cur_pos: (usize, usize), dir: char) -> (usize, usize) {
    return match dir {
        'U' => (cur_pos.0.wrapping_add_signed(-1), cur_pos.1),
        'D' => (cur_pos.0.wrapping_add(1), cur_pos.1),
        'L' => (cur_pos.0, cur_pos.1.wrapping_add_signed(-1)),
        'R' => (cur_pos.0, cur_pos.1.wrapping_add(1)),
        _ => unreachable!(),
    };
}

/// 目的のマスへの最短経路を返す.
/// 障害物なしの正方形グリッドであり, O(1) で求まる
fn shortest_path(
    cur_pos: (usize, usize),
    card_no: usize,
    card_pos: &[(usize, usize)],
) -> Vec<char> {
    let mut ans = vec![];

    let diff_ud = card_pos[card_no].0 as isize - cur_pos.0 as isize;
    for _ in diff_ud..0 {
        ans.push('U');
    }
    for _ in 0..diff_ud {
        ans.push('D');
    }

    let diff_lr = card_pos[card_no].1 as isize - cur_pos.1 as isize;
    for _ in diff_lr..0 {
        ans.push('L');
    }
    for _ in 0..diff_lr {
        ans.push('R');
    }

    ans
}

fn shortest_path_plain(start_pos: (usize, usize), goal_pos: (usize, usize)) -> Vec<char> {
    let mut ans = vec![];

    let diff_ud = goal_pos.0 as isize - start_pos.0 as isize;
    for _ in diff_ud..0 {
        ans.push('U');
    }
    for _ in 0..diff_ud {
        ans.push('D');
    }

    let diff_lr = goal_pos.1 as isize - start_pos.1 as isize;
    for _ in diff_lr..0 {
        ans.push('L');
    }
    for _ in 0..diff_lr {
        ans.push('R');
    }

    ans
}

fn shorter_path(
    cur_pos: (usize, usize),
    card_pos0: (usize, usize),
    card_pos1: (usize, usize),
) -> Vec<char> {
    let len0 = (cur_pos.0 as isize - card_pos0.0 as isize).abs()
        + (cur_pos.1 as isize - card_pos0.1 as isize).abs();
    let len1 = (cur_pos.0 as isize - card_pos1.0 as isize).abs()
        + (cur_pos.1 as isize - card_pos1.1 as isize).abs();
    if len0 < len1 {
        shortest_path_plain(cur_pos, card_pos0)
    } else {
        shortest_path_plain(cur_pos, card_pos1)
    }
}

fn make_pairs_move(
    ans: &mut Vec<char>,
    cur_move_len: &mut usize,
    deck: &mut VecDeque<usize>,
    cur_pos: &mut (usize, usize),
    ann: &mut Vec<Vec<usize>>,
) {
    // 未回収カードの位置を舐めておく
    let mut card_pos = vec![(0, 0); CARD_KIND_NUM];
    for i in 0..N {
        for j in 0..N {
            if ann[i][j] != NO_CARD {
                card_pos[ann[i][j]] = (i, j);
            }
        }
    }

    // 上から順にペアを作っていく
    while let Some(card_no) = deck.pop_back() {
        let p = shortest_path(*cur_pos, card_no, &card_pos);
        for c in p {
            ans.push(c);
            *cur_move_len += 1;
        }
        ans.push('Z');

        *cur_pos = card_pos[card_no];
        ann[cur_pos.0][cur_pos.1] = NO_CARD;
    }
}

#[fastout]
fn main() {
    let start_time = Instant::now();
    let break_time = Duration::from_millis(TIME_LIMIT_MS);

    input! {
        // 固定値
        _n: usize,
        mut ann: [[usize; N]; N],
    }

    // スコアは全カード回収を優先すべき…というかそれ以外の方針が思い浮かばず
    // 愚直だと, 拾う操作のみを考え, 一枚拾ってもう一枚の方に最短経路, を繰り返す
    // ちょっとだけの改善なら, 最短経路上に同じカードが二枚あればペアを作る
    // 置く操作は, マスに山札は作れないために後半ほど使い易くなりそう
    // 切ってしまってもよい？でもそれだと単純になりそう
    // 前半にとりあえず重複なしに拾うだけでも移動回数減らせないか
    // 全マスなめて半分拾う -> 残り半分を上から順で, 前半が N^2, 後半が高々 (N^2)/2*2N
    // 合計だと N^2+N^3 回には収まるので, 制約には反しないし実装もそれほどではない
    // 半分回収する部分も毎度 BFS かけてよいが, 近い順に取っていくとぐるぐる回り得るので最短になるとは限らない
    // 二点間の経路はマンハッタン距離ですぐに求まる
    // 乱択の余地は回収順くらいにしかない

    // マスは 400 個, 数字は 200 種類
    // 全マス間の最短経路は作れる
    // 次の一枚を取るための移動回数が高々 2N で, カードは N^2 枚であるので
    // 最短経路の移動を繰り返すと 2N^3 回の操作が必要, これは最大操作回数と一致
    // つまりは愚直でも X=0 は保証される

    // 置く手を使わないとすると, 最初の半分を回収した経路でスコアが確定する
    // 回収パート内でペア成立させた場合でもそう
    // すると, 回収順を焼き鈍すことはできるが, それほどスコアが伸びる気がしない
    // が, 提出時点での一位との差が二割もないのでちょっと伸ばすとリターンが大きいかも

    // あるいは, 近くにもう片方があるなら先に成立させてしまうとか
    // 同じ経路を何度も通るとスコアが悪化するため

    // Visualizer を見ると, 後半のグラデーションが汚く, 赤マスが離れた位置にある

    // pair 位置
    let mut card_pos = vec![((DUMMY, DUMMY), (DUMMY, DUMMY)); CARD_KIND_NUM];
    for i in 0..N {
        for j in 0..N {
            let no = ann[i][j];

            if card_pos[no].0 == (DUMMY, DUMMY) {
                card_pos[no].0 = (i, j);
            } else {
                card_pos[no].1 = (i, j);
            }
        }
    }

    let mut ans = vec![];
    let mut ans_move_len = usize::MAX;

    // 初期値は DD...D->R->UU...U->... の愚直一本道
    let mut pair_order = vec![];
    {
        let mut cur_pos = (0, 0);
        let mut in_deck = vec![false; CARD_KIND_NUM];
        let mut in_deck_num = 0;
        let mut last_move_dir = 'D';

        // とりあえず全部片側を回収する
        while in_deck_num < CARD_KIND_NUM {
            if !in_deck[ann[cur_pos.0][cur_pos.1]] {
                pair_order.push(ann[cur_pos.0][cur_pos.1]);
                in_deck[ann[cur_pos.0][cur_pos.1]] = true;
                in_deck_num += 1;
            }

            let mut next_pos = next_pos_wo_check(cur_pos, last_move_dir);
            if next_pos.0 >= N || next_pos.1 >= N {
                match last_move_dir {
                    'D' => {
                        // D => R に動いて以後 U
                        next_pos = (cur_pos.0, cur_pos.1 + 1);
                        last_move_dir = 'U';
                    }
                    'U' => {
                        // U => R に動いて以後 D
                        next_pos = (cur_pos.0, cur_pos.1 + 1);
                        last_move_dir = 'D';
                    }
                    // その他は実装を楽するため端折る
                    _ => unreachable!(),
                }
            }
            cur_pos = next_pos;
        }
    }

    while start_time.elapsed() < break_time {
        let mut cur_ans = vec![];
        let mut cur_move_len: usize = 0;

        let mut cur_pos = (0, 0);
        let mut deck = VecDeque::new();
        let mut cleared = vec![false; CARD_KIND_NUM];

        // pair_order 順に最短経路を通って回収

        // 全部片側を回収する
        for &card_num in &pair_order {
            let cur_path = shorter_path(cur_pos, card_pos[card_num].0, card_pos[card_num].1);

            for p in cur_path {
                // 同じ数字が連続するなら先に取る
                if let Some(deck_top) = deck.pop_back() {
                    if deck_top == ann[cur_pos.0][cur_pos.1] {
                        cur_ans.push('Z');
                        cleared[deck_top] = true;
                        ann[cur_pos.0][cur_pos.1] = NO_CARD;
                    } else {
                        deck.push_back(deck_top);
                    }
                }
                cur_ans.push(p);
                cur_pos = next_pos_wo_check(cur_pos, p);
            }
            cur_ans.push('Z');
            deck.push_back(card_num);
            ann[cur_pos.0][cur_pos.1] = NO_CARD;
        }

        // 回収パートに工夫がないので
        make_pairs_move(
            &mut cur_ans,
            &mut cur_move_len,
            &mut deck,
            &mut cur_pos,
            &mut ann,
        );

        if cur_move_len < ans_move_len {
            ans = cur_ans;
            ans_move_len = cur_move_len;
        }
        // TODO: debug
        break;
    }

    for a in ans {
        println!("{a}");
    }
}
