[English (Google or DeepL translate)](./README.md) | 日本語

# sonorust

> このリポジトリは [aq2r/sonorust](https://github.com/aq2r/sonorust) のメンテナンス fork です。
> Discord の音声 E2E 暗号化 (DAVE プロトコル) が 2026 年から必須になったため、
> songbird 0.6 / serenity 0.12.5 へ更新して VC へ接続できる状態を維持しています。

[litagin02/Style-Bert-VITS2](https://github.com/litagin02/Style-Bert-VITS2) の `server_fastapi.py`

または [tuna2134/sbv2-api](https://github.com/tuna2134/sbv2-api) の sbv2_core を利用して読み上げができる Discord bot。

Download: [Releases](https://github.com/aq2r/sonorust/releases)

## 機能

- ユーザーごとに `Model` `Speaker` `Style` を変更 [^1]

- litagin02/Style-Bert-VITS2 使用の場合はアプリ起動時に API を自動起動 [^2]

- tuna2134/sbv2-api 使用の場合は必要なモデル、ONNXRuntime などを自動ダウンロード [^2]

- プレフィックスの変更

- ボイスチャットへの自動参加機能やサーバー辞書などの機能

- 日本語と英語に対応 [^3]

[^1]: Speaker, Style の切り替えは litagin02/Style-Bert-VITS2 のみ対応
[^2]: Windows のみ対応、また Windows 以外は動作未確認
[^3]: 英語はGoogle Translate, DeepL Translate を利用しています。

## ビルド方法

Rust 1.83 以降が必要です。

音声処理に使用している [songbird](https://github.com/serenity-rs/songbird) は libopus の FFI (opus2) に依存しているため、
ビルドには以下のいずれかが必要です。

- Linux: `libopus-dev` (Ubuntu/Debian) もしくは `opus` (Arch) と `pkg-config`
- Linux でシステムの libopus を使わない場合 / Windows / macOS: `cmake` と C コンパイラ (ソースから libopus をビルドします)

```sh
# Ubuntu/Debian の例
sudo apt install cmake pkg-config libopus-dev
cargo build --release
```

## Discord Developer Portal の設定

Bot には以下の Privileged Gateway Intents が必要です。

- SERVER MEMBERS INTENT
- MESSAGE CONTENT INTENT

(PRESENCE INTENT は使用していないため、オンにする必要はありません)

## 使用方法と機能解説

[Sonorust Wiki](https://github.com/aq2r/sonorust/wiki)

## Link

Style-Bert-VITS2: https://github.com/litagin02/Style-Bert-VITS2

sbv2-api: https://github.com/tuna2134/sbv2-api

(この読み上げBOTでは sbv2-api の core 部分を改変して使用しています: https://github.com/aq2r/sbv2_core )

#

#### Lisense

<sub>

    Copyright (C) 2024 aq2r

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU Affero General Public License as published
    by the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU Affero General Public License for more details.

    You should have received a copy of the GNU Affero General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.

</sub>
