// 二列分解で往路小さい値, 復路大きい値
// AI なし実装はかなり厳しく非現実的？
// 参考:
// https://zenn.dev/yoto1980yen/articles/f4867e91ead7f1
// https://x.com/takytank/status/2032759903467786507?s=20
// https://x.com/hossie/status/2035564922340573188?s=20
// https://x.com/kadonox/status/2032792699032789366?s=20
// https://x.com/rotti_coder/status/2032779088541397485?s=20

use proconio::input;

const N: usize = 200;

fn main() {
    input! {
        _n: usize,
        ann: [[u32; N]; N],
    }

    let path = solve(&ann);

    // 出力
    for &(r, c) in &path {
        println!("{} {}", r, c);
    }
}

fn add_to_path(path: &mut Vec<(usize, usize)>, visited: &mut [Vec<bool>], pos: (usize, usize)) {
    let (r, c) = pos;
    assert!(!visited[r][c]);
    path.push(pos);
    visited[r][c] = true;
}

fn solve(ann: &[Vec<u32>]) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(N * N);
    let mut visited = vec![vec![false; N]; N];

    // 初期位置
    let mut cur_pos = (0, 0);
    add_to_path(&mut path, &mut visited, cur_pos);

    // 往路
    for i in 0..N {
        if i % 2 == 1 {
            continue;
        }

        if (i / 2) % 2 == 0 {
            // 上から下
            while cur_pos.0 != N - 3 {
                cur_pos = if ann[cur_pos.0 + 1][i] < ann[cur_pos.0 + 1][i + 1] {
                    (cur_pos.0 + 1, i)
                } else {
                    (cur_pos.0 + 1, i + 1)
                };
                add_to_path(&mut path, &mut visited, cur_pos);
            }

            // 折り返し部は固定
            add_to_path(&mut path, &mut visited, (N - 2, i));
            add_to_path(&mut path, &mut visited, (N - 1, i));
            add_to_path(&mut path, &mut visited, (N - 1, i + 1));
            add_to_path(&mut path, &mut visited, (N - 2, i + 2));
            cur_pos = (N - 2, i + 2);
        } else {
            // 下から上
            while cur_pos.0 != 2 {
                cur_pos = if ann[cur_pos.0 - 1][i] < ann[cur_pos.0 - 1][i + 1] {
                    (cur_pos.0 - 1, i)
                } else {
                    (cur_pos.0 - 1, i + 1)
                };
                add_to_path(&mut path, &mut visited, cur_pos);
            }

            // 折り返し部は固定
            add_to_path(&mut path, &mut visited, (1, i));
            add_to_path(&mut path, &mut visited, (0, i));
            add_to_path(&mut path, &mut visited, (0, i + 1));
            if i != N - 2 {
                // 最終列では折り返せない
                add_to_path(&mut path, &mut visited, (1, i + 2));
                cur_pos = (1, i + 2);
            } else {
                cur_pos = (0, i + 1);
            }
        }
    }

    // 復路
    for i in (0..N).rev() {
        if i % 2 == 1 {
            continue;
        }

        if (i / 2) % 2 == 0 {
            // 下から上
            while cur_pos.0 != 2 {
                cur_pos = if !visited[cur_pos.0 - 1][i] {
                    (cur_pos.0 - 1, i)
                } else {
                    (cur_pos.0 - 1, i + 1)
                };
                add_to_path(&mut path, &mut visited, cur_pos);
            }
            if i >= 2 {
                // 折り返し部
                add_to_path(&mut path, &mut visited, (1, i + 1));
                add_to_path(&mut path, &mut visited, (0, i + 1));
                add_to_path(&mut path, &mut visited, (0, i));
                add_to_path(&mut path, &mut visited, (1, i - 1));
                cur_pos = (1, i - 1);
            } else {
                // 最終列は折り返し考えず続行
                while cur_pos.0 != 0 {
                    cur_pos = if !visited[cur_pos.0 - 1][i] {
                        (cur_pos.0 - 1, i)
                    } else {
                        (cur_pos.0 - 1, i + 1)
                    };
                    add_to_path(&mut path, &mut visited, cur_pos);
                }
            }
        } else {
            // 上から下
            while cur_pos.0 != N - 3 {
                cur_pos = if !visited[cur_pos.0 + 1][i] {
                    (cur_pos.0 + 1, i)
                } else {
                    (cur_pos.0 + 1, i + 1)
                };
                add_to_path(&mut path, &mut visited, cur_pos);
            }
            // 折り返し部
            add_to_path(&mut path, &mut visited, (N - 2, i + 1));
            add_to_path(&mut path, &mut visited, (N - 1, i + 1));
            add_to_path(&mut path, &mut visited, (N - 1, i));
            add_to_path(&mut path, &mut visited, (N - 2, i - 1));
            cur_pos = (N - 2, i - 1);
        }
    }

    path
}
