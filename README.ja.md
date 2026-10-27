<img src="docs/icon.svg" alt="" width="72" height="72">

# sapphire-sync

[English](README.md) | [日本語](README.ja.md)

## どんなものか

sapphire-sync は [sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework)
の上に作った P2P ファイル同期アプリです。フレームワークが担えるいちばん小さなアプリ——
同期専用のリファレンスアプリです。Syncthing と同じ考え方で、`serve` が
組み込みサーバーそのものになり、そのほかのサブコマンドはローカル IPC 経由でサーバーに
話しかけるワンショットのクライアントになります。サブコマンドなしで起動するとヘルプを表示します。

## インストール

```sh
# Linux, macOS
curl -fsSL https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.sh | sh -s -- sync
```

```powershell
# Windows
& ([scriptblock]::Create((irm https://raw.githubusercontent.com/sapphire-turtle/sapphire-framework/main/install/install.ps1))) sync
```

`sapphire-sync` と、同期に使う `sapphire-bridge` が一緒に `~/.local/bin` へ入ります。
デスクトップアプリも入れるには `sync-desktop` を足してください。オプションは
[インストールスクリプトの README](https://github.com/sapphire-turtle/sapphire-framework/tree/main/install)
を参照してください。

[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) でも入れられます。同じリリースファイルを
ビルドせずに `~/.cargo/bin` へダウンロードします。

```sh
cargo binstall sapphire-sync
# デスクトップアプリは crates.io にない
cargo binstall --git https://github.com/sapphire-turtle/sapphire-sync sapphire-sync-desktop
```

## クイックスタート

```console
$ sapphire-bridge serve &            # 常時稼働のピア（sapphire-bridge アプリ）
$ sapphire-sync workspace init --sync ~/Documents/notes
$ sapphire-sync serve &             # このホストの同期ノード
$ sapphire-sync workspace show      # このホストのワークスペースと、ワークグループのもの
```

フレームワーク自身の動詞（`workgroup create`、`device invite`、`workgroup join`）は
フレームワーク側リポジトリの README にあります。このアプリはそれをそのまま再エクスポート
しています。

## 部品のつながり

このリポジトリはひとつのバイナリを出します。アプリサーバー兼 CLI です。常時稼働のピアは
`sapphire-bridge` アプリで、ホストごとに `sapphire-sync serve` をちょうどひとつ動かします。
サーバーが扱うワークスペースは一度にひとつ（`workspace select` で切り替え）で、
`SyncRuntime` はそのレプリカを持ち、ブリッジに登録します。レプリケーションエンジンそのものはアプリではなくフレームワークの実装です。
サーバーとブリッジの起動順は問いません。ブリッジがまだ動いていなければサーバーは同期なしで動き、ブリッジが起動した時点で接続します。ブリッジが再起動したときも接続し直します。

## ステータス出力

`sapphire-sync status` はフレームワークの行（アプリ名・バージョン・エンドポイント）に
加えて、このアプリのワークスペース単位の行を出力します——同期済みのワークスペースひとつ
につき一行、稼働中の `SyncRuntime` からレンダリングされます。ブリッジに接続していない間は、
その前に `bridge: not connected; reconnecting` の行が入ります。

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
「Install & start service」ボタンが見つけられるよう、`sapphire-sync` と
`sapphire-bridge` と同じ場所に入れてください（上の `sh -s -- sync sync-desktop`）。

Windows では Vulkan で描画します。DX12 のデバイスはリモートデスクトップの
再接続のたびに失われるためです。それ以外の OS では wgpu の通常の選択に
任せます。`WGPU_BACKEND` で上書きできます。Windows で起動時にクラッシュする
場合（Vulkan のオーバーレイレイヤーが原因になることがあります）は、
`WGPU_BACKEND=dx12` を付けて起動してください。

## リンク

- [sapphire-framework](https://github.com/sapphire-turtle/sapphire-framework) — このアプリが
  基づくフレームワーク
- [CONTRIBUTING.md](CONTRIBUTING.md) — このリポジトリへの貢献方法

## ライセンス

このリポジトリは異なるライセンスのコンポーネントを含みます：

| コンポーネント | ライセンス |
|-----------|---------|
| `sapphire-sync`（`cli/`） | MIT OR Apache-2.0 |
| `sapphire-sync-core` | MIT OR Apache-2.0 |
| `sapphire-sync-api` | MIT OR Apache-2.0 |
| `sapphire-sync-desktop` | GPL-3.0-or-later、[App Storeマーケットプレイス例外](desktop/LICENSE.App-Store-Exception)付き |

各コンポーネントのディレクトリにある`LICENSE`（または`LICENSE-MIT` / `LICENSE-APACHE`）ファイルに完全なライセンス文書があります。
