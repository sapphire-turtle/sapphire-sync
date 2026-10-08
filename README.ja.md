# sapphire-sync

[English](README.md) | [日本語](README.ja.md)

## どんなものか

sapphire-sync は [sapphire-framework](https://github.com/fluo10/sapphire-framework)
の上に作った P2P ファイル同期アプリです。フレームワークが担えるいちばん小さなアプリ——
同期専用のリファレンスアプリです。Syncthing と同じ考え方で、サブコマンドなしの起動は
組み込みサーバーそのものになり、そのほかのサブコマンドはローカル IPC 経由でサーバーに
話しかけるワンショットのクライアントになります。

## クイックスタート

```console
$ sapphire-bridge serve &            # 常時稼働のピア（sapphire-bridge アプリ）
$ sapphire-sync workspace init --sync ~/Documents/notes
$ sapphire-sync serve &             # このホストの同期ノード
$ sapphire-sync workspace list      # 全デバイスにワークスペースが映る
```

フレームワーク自身の動詞（`workgroup create`、`device invite`、`workgroup join`）は
フレームワーク側リポジトリの README にあります。このアプリはそれをそのまま再エクスポート
しています。

## 部品のつながり

このリポジトリはひとつのバイナリを出します。アプリサーバー兼 CLI です。常時稼働のピアは
`sapphire-bridge` アプリで、ホストごとに `sapphire-sync serve` をちょうどひとつ動かします。
`SyncRuntime` は同期済みのワークスペースルートごとにひとつのレプリカを持ち、ブリッジに
登録します。レプリケーションエンジンそのものはアプリではなくフレームワークの実装です。

## ステータス出力

`sapphire-sync status` はフレームワークの行（アプリ名・バージョン・エンドポイント）に
加えて、このアプリのワークスペース単位の行を出力します——同期済みのワークスペースひとつ
につき一行、稼働中の `SyncRuntime` からレンダリングされます。

## `.sapphireignore`

ワークスペースの*中に*置かれる、`.gitignore` 形式のファイルです。他のファイルと同じく
同期されます。組み込みルールとして、このファイル自身の競合コピー
（`.sapphireignore.conflict-*`）は除外されたままになります——競合ノイズが伝播しません。

## 競合

同時編集は両方のバージョンを残す形で解決されます。負けた側は勝者の隣に
`<stem>.conflict-<replica-id>-<counter>.<ext>` として書き込まれます。上書きは起こらず、
競合は人間の手で解決します——Syncthing の流儀です。

## 「サーバーが動いていない」契約

ワンショットの動詞はサーバーを起動しません。サーバーがリッスンしていないとき、コマンドは
`no sapphire-sync server is running` と表示し、終了コード 1 を返します。

## デスクトップアプリ

`sapphire-sync-desktop` は、インストール済みサービスの GUI クライアントです。
フレームワークの同期パネルを表示するだけで、サーバーもブリッジも自分では
起動しません。ウィンドウを閉じても同期は止まりません。
`cargo build -p sapphire-sync-desktop --release` でビルドし、「Install & start
service」ボタンが見つけられるよう `sapphire-sync` と `sapphire-bridge` の
隣に置いて配布してください。

Windows では Vulkan で描画します。DX12 のデバイスはリモートデスクトップの
再接続のたびに失われるためです。それ以外の OS では wgpu の通常の選択に
任せます。`WGPU_BACKEND` で上書きできます。Windows で起動時にクラッシュする
場合（Vulkan のオーバーレイレイヤーが原因になることがあります）は、
`WGPU_BACKEND=dx12` を付けて起動してください。

## リンク

- [sapphire-framework](https://github.com/fluo10/sapphire-framework) — このアプリが
  基づくフレームワーク
- [CONTRIBUTING.md](CONTRIBUTING.md) — このリポジトリへの貢献方法
