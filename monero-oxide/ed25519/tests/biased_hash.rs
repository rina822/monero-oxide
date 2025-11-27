// ed25519 のテスト: biased_hash
// このファイルはテストベクタ (`tests.txt`) を読み、コマンドに応じた検証を行います。
// 各行は空白で区切られたトークン列であり、最初のトークンがコマンドです。
use monero_ed25519::Point;

mod hex;

#[test]
fn biased_hash() {
  // テストベクタを読み込む（コンパイル時に埋め込まれる）
  let reader = include_str!("./tests.txt");

  // 各行を処理するループ
  for line in reader.lines() {
    // 空白で分割してコマンドと引数を取得
    let mut words = line.split_whitespace();

    // 最初のトークンはコマンド名
    let command = words.next().unwrap();
    match command {
      // `check_key` はここでは何もしない（他のテストで使用される場合あり）
      "check_key" => {}

      // `hash_to_ec` の場合、プレイメージを点にハッシュして期待値と比較する
      "hash_to_ec" => {
        // 16進文字列をデコードして 32 バイト配列にする
        let preimage = hex::decode(words.next().unwrap());
        // 実際に偏り付きハッシュを計算
        let actual = Point::biased_hash(preimage);
        // 期待値（テストベクタ側）
        let expected = hex::decode(words.next().unwrap());
        // compressed 表現のバイト列を比較して一致を確認
        assert_eq!(actual.compress().to_bytes(), expected);
      }

      // 不明なコマンドはテスト失敗
      _ => unreachable!("unknown command"),
    }
  }
}
