// テスト用ユーティリティ: 16 進文字列（長さ 64）を 32 バイト配列にデコードする
#[allow(dead_code)]
pub(crate) fn decode(hex: &str) -> [u8; 32] {
  let mut point = [0; 32];
  // 32 バイト = 64 文字（16 進）の前提をチェック
  assert_eq!(hex.len(), 64);

  // 各 16 進文字を解析して 4 ビットずつ詰める
  for (i, c) in hex.chars().enumerate() {
    point[i / 2] |= (match c {
      // '0'..'9' を数値に変換
      '0' ..= '9' => (c as u8) - b'0',
      // 'A'..'F' を数値に変換
      'A' ..= 'F' => (c as u8) - b'A' + 10,
      // 'a'..'f' を数値に変換
      'a' ..= 'f' => (c as u8) - b'a' + 10,
      // 不正な文字はパニック（テストベクタ誤り）
      _ => panic!("test vectors contained invalid hex"),
    }) << (4 * ((i + 1) % 2)); // 偶数/奇数文字で上位/下位ニブルへシフト
  }
  point
}
