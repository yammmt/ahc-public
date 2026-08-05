---
name: run-many-cases
description: Run all test cases for this AHC. You can use this for any
             evaluation steps.
---

# テストケースを一斉に評価する

pahcer を用いて, すべてのテストケースに対してあなたのコードを実行します.

## Workflow

以下のコマンドで, テストを実行します.

```console
pahcer run
```

実行結果は, `pahcer/json/` 以下に JSON ファイルとして出力されます.

## 実行後のコード変更

スコアの結果を見て次のアクションを考える部分は人間の仕事です.
すなわち, あなたは一度のユーザー指示に対し, (その指示に特別な言及がない限りは) 自律的に `pahcer` を実行し, スコア改善を試みる行動を取ってはなりません.

一方, 実装後に一度 `pahcer` を実行しスコアの変化を報告することは行って下さい.
ここでは, 実行時にエラーが発生した場合には, そのエラーを解消するまでは自律的に修正を行って下さい.
