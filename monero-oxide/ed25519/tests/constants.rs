// ed25519 の定数に関するテスト
// 主要な定数（INV_EIGHT, G, H）が期待値と一致することを検証します。
use sha3::{Digest, Keccak256};
use curve25519_dalek::{constants::ED25519_BASEPOINT_POINT, edwards::CompressedEdwardsY};
use monero_ed25519::{Scalar, CompressedPoint};

#[test]
fn constants() {
  // Scalar::INV_EIGHT が curve25519_dalek の 8 の逆数と一致することを確認
  assert_eq!(
    <[u8; 32]>::from(Scalar::INV_EIGHT),
    curve25519_dalek::Scalar::from(8u8).invert().to_bytes()
  );

  // G（ベースポイント圧縮）の定義が期待通りか検証
  assert_eq!(
    CompressedPoint::G,
    CompressedPoint::from(ED25519_BASEPOINT_POINT.compress().to_bytes())
  );

  // H の生成ロジックを再現して期待値と一致するかを検証
  assert_eq!(
    CompressedPoint::H,
    CompressedPoint::from(
      CompressedEdwardsY(Keccak256::digest(ED25519_BASEPOINT_POINT.compress().to_bytes()).into())
        .decompress()
        .expect("known on-curve point wasn't on-curve")
        .mul_by_cofactor()
        .compress()
        .to_bytes()
    )
  );
}
