# sbv2_core

このクレートは [aq2r/sbv2_core](https://github.com/aq2r/sbv2_core) を取り込んだものです。
さらにその元は [tuna2134/sbv2-api](https://github.com/tuna2134/sbv2-api/) の sbv2_core 部分です。

取り込み元のコミット: `3b592e2fdbb4be117db60d9fcd8a73f0c0ee172e`

## 取り込み後の変更点

- `src/model.rs`: 推論時に、モデルのセッションに存在する入力だけを渡すようにした

  配布されている `.sbv2` (`version.txt` = `1`) は ONNX グラフに `noise_scale` と
  `noise_scale_w` を持たず (書き出し時に値が埋め込まれている)、それらを渡すと
  `Invalid input name: noise_scale` で推論できなかったため。

### LICENSE

ライセンス: [LICENSE](./LICENSE)

オリジナルのライセンス: [LICENSE.original](./LICENSE.original)
