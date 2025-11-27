// ed25519 のポイント復号(decompress)に関するテスト
// テストベクタを読み、圧縮表現が曲線上の点かどうかを検証します。
use monero_ed25519::CompressedPoint;

mod hex;

#[test]
fn decompress() {
  // 埋め込みテストベクタを読み込む
  let reader = include_str!("./tests.txt");

  for line in reader.lines() {
    let mut words = line.split_whitespace();

    let command = words.next().unwrap();
    match command {
      // `check_key` コマンド: 次のトークンは 16 進エンコードされたキー、次は true/false
      "check_key" => {
        let key = hex::decode(words.next().unwrap());
        let expected = match words.next().unwrap() {
          "true" => true,
          "false" => false,
          _ => unreachable!("invalid result"),
        };

        // 圧縮表現から点を復号し、Some ならオンカーブ、None ならオフカーブ
        let actual = CompressedPoint::from(key).decompress();
        assert_eq!(actual.is_some(), expected);
      }

      // `hash_to_ec` はこのテストファイルでは処理しない（別テストで使用）
      "hash_to_ec" => {}

      // 不明なコマンドはエラー
      _ => unreachable!("unknown command"),
    }
  }
}
