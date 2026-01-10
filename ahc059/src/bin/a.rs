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
// use superslice::Ext;

const N: usize = 20;
const CARD_KIND_NUM: usize = 200;
const NO_CARD: usize = 9999;

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

    let mut ans = vec![];
    let mut ans_move_len = usize::MAX;

    // 乱択入れるかもなのでブロックにする
    {
        let mut cur_ans = vec![];
        let mut cur_move_len: usize = 0;

        let mut cur_pos = (0, 0);
        let mut last_move_dir = 'D';
        let mut deck = VecDeque::new();
        let mut in_deck = vec![false; CARD_KIND_NUM];
        let mut in_deck_num = 0;
        let mut cleared = vec![false; CARD_KIND_NUM];

        // とりあえず全部片側を回収する
        while in_deck_num < CARD_KIND_NUM {
            if let Some(deck_top) = deck.pop_back() {
                if deck_top == ann[cur_pos.0][cur_pos.1] {
                    // 同じ数字が連続したのでペア成立
                    cur_ans.push('Z');
                    cleared[deck_top] = true;
                    ann[cur_pos.0][cur_pos.1] = NO_CARD;
                } else {
                    deck.push_back(deck_top);
                }
            }

            if ann[cur_pos.0][cur_pos.1] != NO_CARD {
                if !in_deck[ann[cur_pos.0][cur_pos.1]] {
                    cur_ans.push('Z');
                    deck.push_back(ann[cur_pos.0][cur_pos.1]);
                    in_deck[ann[cur_pos.0][cur_pos.1]] = true;
                    in_deck_num += 1;
                    ann[cur_pos.0][cur_pos.1] = NO_CARD;
                }
            }

            if in_deck_num == CARD_KIND_NUM {
                // 無駄に動かない
                break;
            }

            let mut next_pos = next_pos_wo_check(cur_pos, last_move_dir);
            if next_pos.0 >= N || next_pos.1 >= N {
                match last_move_dir {
                    'D' => {
                        // D => R に動いて以後 U
                        cur_ans.push('R');
                        next_pos = (cur_pos.0, cur_pos.1 + 1);
                        last_move_dir = 'U';
                    }
                    'U' => {
                        // U => R に動いて以後 D
                        cur_ans.push('R');
                        next_pos = (cur_pos.0, cur_pos.1 + 1);
                        last_move_dir = 'D';
                    }
                    // その他は実装を楽するため端折る
                    _ => unreachable!(),
                }
            } else {
                cur_ans.push(last_move_dir);
            }
            cur_move_len += 1;
            cur_pos = next_pos;
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
    }

    for a in ans {
        println!("{a}");
    }
}
