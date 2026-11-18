# Clippy（日本語版）

Windows用の超軽量インスタントリプレイレコーダー。常にバックグラウンドで録画（RAM上）し、キー（デフォルトF9）を押すと直近の数秒をMP4として保存します。ビデオはすべてGPU上で処理されます（DXGI Desktop Duplication → D3D11 NV12 → Media FoundationのハードウェアH.264エンコーダ）。

**これは [santiremedi/clippy](https://github.com/santiremedi/clippy) の日本語翻訳版です。** 変更点は翻訳のみです。

オリジナルのダウンロード・更新情報は [clpy.vercel.app](https://clpy.vercel.app)。リリースはオリジナルの [Releases](https://github.com/santiremedi/clippy/releases) で公開されており、自動アップデーターがそこを参照します。

## ビルド

```bash
cargo build --release
```

実行ファイルは `target/release/clippy.exe` に出力されます。公開時は `Clippy.exe` にリネームしてください（アップデーターがこの名前を探します）。

開発中のテスト用（ウィンドウなしのモード。公開版には含まれません）:

```bash
cargo build --profile fast --features devtools
```

## 同じ .exe のモード

| 引数 | 動作 |
| --- | --- |
| （なし） / `--background` | トレイアイコン付きレコーダー（`--background` はウィンドウを開かない。Windows起動時用）。 |
| `--settings` | メインウィンドウ / 設定。 |
| `--editor [ファイル]` | クリップエディタ。 |
| `--apply-update <パス>` | アップデーターが使用。インストール済みexeを置き換えて再起動。 |

`--features devtools` 時のみ:

| 引数 | 動作 |
| --- | --- |
| `--test <秒> <フォルダ> [game\|desktop\|mic\|juegos\|720\|1080\|1440]` | 録画してクリップを保存し終了（エンコーダと検出ゲームを表示。数字は解像度指定）。 |
| `--test-mic` | 各マイクの3秒間の音声レベルを確認。 |
| `--test-download` | 「更新」ボタンと同じように公開版をダウンロード。 |
| `--test-export`, `--test-frame`, `--test-strip` | エディタのテスト（`src/editor/devtools.rs` 参照）。 |

## コード構成

```
src/
  main.rs          モード選択
  config.rs        config.json（録画、音声、グローバルショートカット）
  log.rs           %APPDATA%\Clippy、clippy.log、タイムスタンプ
  mf.rs            Media Foundation とアプリ全体の音声フォーマット（PCM 48kHz ステレオ、AAC）
  clipcache.rs     クリップごとの波形・キーフレームキャッシュ（レコーダーが書き、エディタが読む）
  app/             Windowsアプリとしてのプロセス
    mode.rs        モード、引数、ロック、ウィンドウタイトル、トレイメッセージ
    tray.rs        トレイ、グローバルショートカット、レコーダー制御
    update.rs      自動更新
    autostart.rs   Windows起動時（HKCU\...\Run）
    keys.rs        Win32キー名
  record/          録画
    recorder.rs    録画スレッド: キャプチャ → エンコーダ → バッファ → 保存
    game.rs        「ゲーム」判定（フルスクリーンウィンドウ）— 映像・音声用
    capture.rs, convert.rs, encoder.rs, audio.rs, mux.rs, clock.rs
  settings/        メインウィンドウ（mod.rs: 状態; pages.rs: ページ）
  editor/          クリップエディタ（別プロセス）
    mod.rs         ウィンドウとメインループ
    timeline.rs    トラックとピース（映像・音声連結）、スナップ、フェード、元に戻す
    playback.rs    再生: 見える/聞こえるピースごとにプレイヤー、レイヤーとフェード
    export.rs      書き出しダイアログと処理
    panels.rs      上部バーとタイムライン（トラック、マーカー、クリップ右クリック）
    browser.rs     ライブラリパネル: 選択、フォルダ、移動、ゴミ箱へ
    inspector.rs   クリップパネル: 画像、速度、音声
    preview.rs     プレビュー: アスペクト比付きキャンバスと画像クロップ
    prefs.rs       editor.json（ショートカット、最後の書き出し設定、パネル、スナップ）
    media.rs       書き出し: 再エンコードなしコピー、またはGPUレイヤー合成・フェード、速度、音声ミックス
    shortcuts.rs, library.rs, player.rs, devtools.rs
  ui_kit/          スタイルとウィジェット、ネイティブウィンドウ、アイコン、サウンド
    watch.rs       外部変化時にウィンドウを起こす（アイドル時は再描画しない）
```

## ユーザーファイル

すべて `%APPDATA%\Clippy` 内: `config.json`, `editor.json`, `library.json`, `update.json`, `clippy.log`, `cache\`。

## バージョン公開

1. `Cargo.toml` のバージョンを上げる。
2. GitHub → Actions → **Build** → *Run workflow* でタグ（例: `v0.3.0`）を指定し「Publicar el release」にチェック。ワークフローがコンパイルして `Clippy.exe` をこのリポジトリのリリースにアップロード（存在しなければドラフトで作成）。チェックなしでもコンパイルし、ダウンロード可能なアーティファクトとして残します。
3. アプリ利用者が見るリリースノートを書く（Clippyが「新着情報」として表示: `#` 見出し、`-` 箇条書き、段落。`**` やバッククォートは使わない）。その後公開。テキストなしの公開リリースは新着情報を表示しません。

## ライセンス

[GPL-3.0-or-later](LICENSE)。使用・研究・変更・再配布が可能です。変更版を配布する場合はソースコードを含め、同じライセンスで配布する必要があります。「Clippy」という名前とそのアイコンは、変更版の識別には使用できません。

---

**日本語翻訳:** 本リポジトリはオリジナルのUI文字列・ドキュメントを日本語に翻訳したものです。機能・動作はオリジナルと同一です。
