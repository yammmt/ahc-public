// 二列分解で往路小さい値, 復路大きい値, を Gemini に依頼
// 参考:
// https://zenn.dev/yoto1980yen/articles/f4867e91ead7f1
// https://x.com/takytank/status/2032759903467786507?s=20
// https://x.com/hossie/status/2035564922340573188?s=20
// https://x.com/kadonox/status/2032792699032789366?s=20
// https://x.com/rotti_coder/status/2032779088541397485?s=20

use proconio::input;

fn main() {
    input! {
        n: usize,
        a: [[u32; n]; n],
    }

    let path = solve(n, &a);

    // 出力
    for &(r, c) in &path {
        println!("{} {}", r, c);
    }
}

fn solve(n: usize, a: &[Vec<u32>]) -> Vec<(usize, usize)> {
    let mut path = Vec::with_capacity(n * n);
    let mut visited = vec![vec![false; n]; n];

    // 解決策1: パス追加と訪問済みの記録を「未訪問の場合のみ」行うクロージャ
    let mut add_to_path = |r: usize, c: usize| {
        if !visited[r][c] {
            path.push((r, c));
            visited[r][c] = true;
        }
    };

    // 【往路】 (Outward Journey) : 左のグループから右のグループへ、小さい数を拾いながら進む
    for g in 0..n / 2 {
        let c0 = 2 * g;
        let c1 = 2 * g + 1;

        if g % 2 == 0 {
            // 偶数グループ: 下方向へ進む
            if g == 0 {
                add_to_path(0, 0); // 初期位置
                for r in 1..=n - 4 {
                    if a[r][c0] < a[r][c1] {
                        add_to_path(r, c0);
                    } else {
                        add_to_path(r, c1);
                    }
                }
            } else {
                add_to_path(2, c0); // 遷移後の開始位置
                for r in 3..=n - 4 {
                    if a[r][c0] < a[r][c1] {
                        add_to_path(r, c0);
                    } else {
                        add_to_path(r, c1);
                    }
                }
            }

            // 次のグループへ安全に斜め移動するため、N-3行目は強制的に内側(c0)を踏む
            add_to_path(n - 3, c0);

            // 下端折り返し (右方向へ)
            add_to_path(n - 2, c0);
            add_to_path(n - 1, c0);
            add_to_path(n - 1, c1);
            add_to_path(n - 2, c1);
        } else {
            // 奇数グループ: 上方向へ進む
            add_to_path(n - 3, c0); // 遷移後の開始位置

            for r in (3..=n - 4).rev() {
                if a[r][c0] < a[r][c1] {
                    add_to_path(r, c0);
                } else {
                    add_to_path(r, c1);
                }
            }

            // 次のグループへ安全に斜め移動するため、2行目は強制的に内側(c0)を踏む
            add_to_path(2, c0);

            // 上端折り返し (右方向へ)
            add_to_path(1, c0);
            add_to_path(0, c0);
            add_to_path(0, c1);
            add_to_path(1, c1);
        }
    }

    // 【復路】 (Return Journey) : 右のグループから左のグループへ、未訪問のマスを回収しながら戻る
    for g in (0..n / 2).rev() {
        let c0 = 2 * g;
        let c1 = 2 * g + 1;

        if g % 2 != 0 {
            // 奇数グループ: 復路は下方向へ進む
            add_to_path(2, c1);
            for r in 3..=n - 4 {
                // 片方が往路で訪問済みなので、両方呼べば未訪問の方が追加される
                add_to_path(r, c0);
                add_to_path(r, c1);
            }
            add_to_path(n - 3, c1);

            // 下端折り返し
            add_to_path(n - 2, c1);
            add_to_path(n - 1, c1);
            add_to_path(n - 1, c0);
            add_to_path(n - 2, c0);
        } else {
            // 偶数グループ: 復路は上方向へ進む
            if g == 0 {
                add_to_path(n - 3, c1);
                for r in (2..=n - 4).rev() {
                    add_to_path(r, c0);
                    add_to_path(r, c1);
                }
                // ゴール地点（残った0行目・1行目の回収）
                add_to_path(1, c0);
                add_to_path(1, c1);
                add_to_path(0, c1);
            } else {
                add_to_path(n - 3, c1);
                for r in (3..=n - 4).rev() {
                    add_to_path(r, c0);
                    add_to_path(r, c1);
                }
                add_to_path(2, c1);

                // 上端折り返し
                add_to_path(1, c1);
                add_to_path(0, c1);
                add_to_path(0, c0);
                add_to_path(1, c0);
            }
        }
    }

    // 全てのマスを訪問したかをアサート
    assert_eq!(path.len(), n * n, "Visited cell count mismatch!");

    path
}
