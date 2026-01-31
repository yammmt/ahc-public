use proconio::fastout;
use proconio::input;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

const TIME_LIMIT_MS: u64 = 1980;

// 焼きなまし法の温度パラメータ
// 高すぎると良解を破壊してしまうため、少し低めに設定して山登りに近づける
const START_TEMP: f64 = 1.0;
const END_TEMP: f64 = 0.0;

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

fn choose_side_id(
    cur_pos: (usize, usize),
    card_pos0: (usize, usize),
    card_pos1: (usize, usize),
) -> u8 {
    let len0 = (cur_pos.0 as isize - card_pos0.0 as isize).abs()
        + (cur_pos.1 as isize - card_pos0.1 as isize).abs();
    let len1 = (cur_pos.0 as isize - card_pos1.0 as isize).abs()
        + (cur_pos.1 as isize - card_pos1.1 as isize).abs();
    if len0 <= len1 { 0 } else { 1 }
}

fn make_pairs_move(
    ans: &mut Vec<char>,
    cur_move_len: &mut usize,
    deck: &mut VecDeque<(usize, (usize, usize))>,
    cur_pos: &mut (usize, usize),
) {
    // 上から順にペアを作っていく
    while let Some((card_no, target_pos)) = deck.pop_back() {
        // カードのもう片割れの位置へ移動
        let p = shortest_path_plain(*cur_pos, target_pos);
        for c in p {
            ans.push(c);
            *cur_move_len += 1;
        }
        ans.push('Z');

        *cur_pos = target_pos;
    }
}

#[fastout]
fn main() {
    let start_time = Instant::now();
    let break_time = Duration::from_millis(TIME_LIMIT_MS);

    let mut rng = SmallRng::seed_from_u64(1);

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

    let mut all_card_locations = vec![Vec::new(); CARD_KIND_NUM];
    for i in 0..N {
        for j in 0..N {
            let no = ann[i][j];
            all_card_locations[no].push((i, j));
        }
    }

    let mut best_ans = vec![];
    let mut best_move_len = usize::MAX;

    // 現在の解（焼きなまし法用）
    let mut cur_move_len_sa: usize = usize::MAX;

    // 初期値は DD...D->R->UU...U->... の愚直一本道
    let mut pair_order: Vec<(usize, u8)> = vec![];
    {
        let mut cur_pos = (0, 0);
        let mut in_deck = vec![false; CARD_KIND_NUM];
        let mut in_deck_num = 0;
        let mut last_move_dir = 'D';

        // とりあえず全部片側を回収する
        while in_deck_num < CARD_KIND_NUM {
            let cur_card = ann[cur_pos.0][cur_pos.1];
            if !in_deck[cur_card] {
                // all_card_locations[cur_card][0] か [1] か判定
                let idx = if cur_pos == all_card_locations[cur_card][0] {
                    0
                } else {
                    1
                };
                pair_order.push((cur_card, idx));
                in_deck[cur_card] = true;
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

    let mut is_first_try = true;
    // デバッグ用カウンタ
    let mut iteration_count: usize = 0;
    let mut accept_count: usize = 0;
    let mut improve_count: usize = 0;
    let mut best_update_count: usize = 0;
    let mut worsen_accept_count: usize = 0; // 改悪を受け入れた回数
    let mut total_cur_move_len: usize = 0; // 評価した解のスコア合計（平均計算用）

    while start_time.elapsed() < break_time {
        iteration_count += 1;

        // 高速化のため ann_cur の複製を削除
        let mut pair_order_cur = pair_order.clone();
        if !is_first_try {
            // 近傍解の生成
            let neighbor_type = rng.random_range(0..100);
            if neighbor_type < 40 {
                // 2 点入れ替え (近傍限定)
                // 遠方同士のスワップは破壊的すぎるため、近い距離のものに限る
                let a = rng.random_range(0..CARD_KIND_NUM);
                let dist = rng.random_range(1..=20);
                let b = if rng.random_bool(0.5) {
                    (a + dist).min(CARD_KIND_NUM - 1)
                } else {
                    a.saturating_sub(dist)
                };
                pair_order_cur.swap(a, b);
            } else if neighbor_type < 90 {
                // 区間反転 (2-opt)
                let a = rng.random_range(0..CARD_KIND_NUM);
                let len = rng.random_range(2..=50);
                let r = (a + len).min(CARD_KIND_NUM - 1);
                pair_order_cur[a..=r].reverse();
            } else {
                // 1 枚の回収順を反転 (side_id を反転)
                let a = rng.random_range(0..CARD_KIND_NUM);
                pair_order_cur[a].1 ^= 1;
            }
        }

        let mut cur_ans = vec![];
        let mut cur_move_len: usize = 0;

        let mut cur_pos = (0, 0);
        let mut deck = VecDeque::new();
        // let mut cleared = vec![false; CARD_KIND_NUM]; // 未使用

        // pair_order 順に最短経路を通って回収

        // 全部片側を回収する
        for (card_num, side_id) in &pair_order_cur {
            // side_id は選んだ方 (0 or 1). 残っている方は (1 ^ side_id)
            let target_pos = all_card_locations[*card_num][*side_id as usize];
            let remaining_pos = all_card_locations[*card_num][(*side_id ^ 1) as usize];

            let cur_path = shortest_path_plain(cur_pos, target_pos);

            for p in cur_path {
                cur_ans.push(p);
                cur_pos = next_pos_wo_check(cur_pos, p);
                cur_move_len += 1;
            }
            cur_ans.push('Z');
            deck.push_back((*card_num, remaining_pos));
        }

        // 回収パートに工夫がないので
        make_pairs_move(&mut cur_ans, &mut cur_move_len, &mut deck, &mut cur_pos);

        total_cur_move_len += cur_move_len;

        // 焼きなまし法の受け入れ判定
        let delta = cur_move_len as f64 - cur_move_len_sa as f64;
        let accept = if is_first_try {
            true
        } else {
            if delta <= 0.0 {
                // 改善解は常に受け入れ
                improve_count += 1;
                true
            } else {
                // 改悪解は確率的に受け入れ
                // 温度計算は accept 判定時のみ行う（軽量化）
                let elapsed = start_time.elapsed().as_secs_f64();
                let time_ratio = (elapsed / (TIME_LIMIT_MS as f64 / 1000.0)).min(1.0);
                // 指数冷却を採用
                let temp = START_TEMP * (END_TEMP / START_TEMP).powf(time_ratio);
                let prob = (-delta / temp).exp();
                let r = rng.random::<f64>();
                if r < prob {
                    worsen_accept_count += 1;
                    true
                } else {
                    false
                }
            }
        };

        if accept {
            accept_count += 1;
            // 変な挙動がないかチェック (デバッグ用)
            // if cur_move_len_sa != usize::MAX && cur_move_len > cur_move_len_sa && delta <= 0.0 {
            //     eprintln!("Bug: Worsening score accepted as improvement!");
            // }
            cur_move_len_sa = cur_move_len;
            pair_order = pair_order_cur;

            // 最良解の更新
            if cur_move_len < best_move_len {
                best_update_count += 1;
                best_ans = cur_ans;
                best_move_len = cur_move_len;
            }
        }

        is_first_try = false;
    }

    eprintln!("=== デバッグ情報 ===");
    eprintln!("試行回数: {}", iteration_count);
    eprintln!(
        "accept 回数: {} ({:.2}%)",
        accept_count,
        accept_count as f64 / iteration_count as f64 * 100.0
    );
    eprintln!(
        "  - 改善で accept: {} ({:.2}%)",
        improve_count,
        improve_count as f64 / iteration_count as f64 * 100.0
    );
    eprintln!(
        "  - 改悪で accept: {} ({:.2}%)",
        worsen_accept_count,
        worsen_accept_count as f64 / iteration_count as f64 * 100.0
    );
    eprintln!("best 更新回数: {}", best_update_count);
    eprintln!("最終 best_move_len: {}", best_move_len);
    eprintln!("最終 cur_move_len_sa: {}", cur_move_len_sa);
    eprintln!(
        "評価した解の平均スコア: {:.1}",
        total_cur_move_len as f64 / iteration_count as f64
    );

    for a in best_ans {
        println!("{a}");
    }
}
