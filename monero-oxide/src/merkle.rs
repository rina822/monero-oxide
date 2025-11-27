use crate::primitives::keccak256;

/// Merkle ルート計算（Monero の `tree_hash` 相当）。
///
/// Monero の Merkle 木は非対称で、各リーフは一意なインデックスで一度だけ含まれる点と、
/// リーフと内部ノードが区別されない点が特徴です。このためルート検証にはリーフ数が
/// 必須になります。
///
/// この関数は一時領域として可変スライスを受け取り、その中身をスクラッチに使います。
/// 呼び出し後のスライス内容は未定義です。
pub fn merkle_root(mut leaves: impl AsMut<[[u8; 32]]>) -> Option<[u8; 32]> {
  let mut leaves = leaves.as_mut();

  // 一時 64 バイトバッファを用意して左右を連結して keccak256 を計算するヘルパを作る
  let mut pair_buf = [0; 64];
  let mut pair = |left: &[u8; 32], right: &[u8; 32]| {
    pair_buf[.. 32].copy_from_slice(left);
    pair_buf[32 ..].copy_from_slice(right);
    keccak256(pair_buf)
  };

  match leaves.len() {
    0 => None,
    1 => Some(leaves[0]),
    2 => Some(pair(&leaves[0], &leaves[1])),
    _ => {
      // まず、与えられたリーフ数以下で最大の 2 の累乗を見つける
      let mut low_pow_2 = {
        let highest_bit_set = usize::BITS - leaves.len().leading_zeros();
        1 << (highest_bit_set - 1)
      };

      while leaves.len() != 1 {
        // 与えられたリーフ数が既に pow2 なら、次の小さい pow2 に下げる
        if leaves.len() == low_pow_2 {
          low_pow_2 >>= 1;
        }

        // 次の pow2 にするためにペアリングする個数
        let overage = leaves.len() - low_pow_2;
        let start = low_pow_2 - overage;
        for i in 0 .. overage {
          let left = leaves[start + (2 * i)];
          let right = leaves[start + (2 * i) + 1];
          leaves[start + i] = pair(&left, &right);
        }
        // 初期のペア処理が終わったらスライスを短くする
        leaves = &mut leaves[.. low_pow_2];
      }

      Some(leaves[0])
    }
  }
}
