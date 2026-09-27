# AHC072 ローカル評価

Rust と `pahcer` 0.4.0 を用意し、リポジトリのルートで次を実行する。

```console
pahcer run
```

このコマンドは [pahcer_config.toml](pahcer_config.toml) に従い、解法と AtCoder 配布の `tools/src/bin/vis.rs` をビルドする。`tools/in` の seed 0〜99 を解法へ入力し、出力を `vis --no-vis` で検証・採点する。配布ツール単体の使い方は [tools/README.md](tools/README.md) (リポジトリ管理外) を参照。

終了時の `Accepted` が `100 / 100` なら、全ケースで正のスコアが得られている。配布 `vis` は不正な出力に `Score = 0` を返し、`pahcer` はスコア 0 を WA として扱うため、結果 JSON の手動確認は不要。スコアと実行時間は同じコマンドの出力に表示され、詳細は `pahcer/json/` に保存される。

`pahcer` は実行時間を表示するが、2 秒を超えても自動では TLE にしない。時間制限については、表示される `Max Execution Time` を確認する。
