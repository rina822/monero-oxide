// ファイル属性: docs.rs のビルド環境で `doc_cfg` を有効にする（条件付き機能フラグ）。
// `#![...]` はクレート属性で、コンパイル単位全体に影響します（属性はファイル先頭で扱うことが多い）。
#![cfg_attr(docsrs, feature(doc_cfg))] // docsrs でのドキュメント生成用フラグ
// `include_str!` マクロはコンパイル時にファイルを読み込み文字列として埋め込みます。
// これにより README の内容がドキュメント化されます。
#![doc = include_str!("../README.md")] // README をドキュメントに含める
// `deny(missing_docs)` はドキュメントがない公開アイテムをコンパイルエラーにする属性です。
#![deny(missing_docs)] // ドキュメントが必須

// JSON デシリアライズ用のトレイトをインポート
use serde::Deserialize; // JSON -> Rust 構造体の変換に使用
// `use` は名前を現在のスコープへ導入する構文です。
// `serde::Deserialize` は JSON -> Rust 構造体へ変換するためのトレイトです。
// `json!` マクロはコンパイル時に JSON 値を構築するためのヘルパです。
use serde_json::json;

// monero_oxide 内の必要な型やモジュールをネームスペースからインポートします。
use monero_oxide::{
  // ed25519 関連: スカラー、圧縮点、コミットメント
  ed25519::{Scalar, CompressedPoint, Commitment},
  // ringct 関連: 可プルナブル部分の列挙型とバッチ検証器
  ringct::{RctPrunable, bulletproofs::BatchVerifier},
  // トランザクションと入力型
  transaction::{Input, Transaction},
  // ブロック型
  block::Block,
};

// RPC エラーとトレイトをインポート
// `monero_rpc::Rpc` はこのコードが期待する RPC インターフェイスを表すトレイトです。
use monero_rpc::{RpcError, Rpc};
// シンプルなリクエストベースの RPC クライアント（HTTP 経由）を提供する型
use monero_simple_request_rpc::SimpleRequestRpc;

// Tokio のタスクハンドル型をインポート（非同期タスクを表す）
use tokio::task::JoinHandle;

// `async fn` は非同期関数を定義するための構文です。
// 戻り値は `impl Future<Output=...>` 相当になり、`await` で待てます。
// この関数は一つのブロックを検証するための主要ロジックを持ちます。
async fn check_block(rpc: impl Rpc, block_i: usize) {
  // ブロックハッシュを取得するループ（接続エラー時にリトライする）
  // `loop { ... }` は無限ループで、`break value;` によって式として値を返せます。
  // ここでは `match` によるパターンマッチで Result を分岐し、成功時に break しています。
  let hash = loop {
    match rpc.get_block_hash(block_i).await {
      // 成功時: 取得したハッシュをループ外へ返す
      Ok(hash) => break hash,
      // 接続エラーの場合はログを出して再試行
      Err(RpcError::ConnectionError(e)) => {
        println!("get_block_hash ConnectionError: {e}");
        continue;
      }
      // その他のエラーは致命的と判断してパニック
      Err(e) => panic!("couldn't get block {block_i}'s hash: {e:?}"),
    }
  };

  // `get_block` RPC の JSON レスポンスを受け取るための構造体定義
  // `#[derive(Deserialize, Debug)]` により自動で JSON -> 構造体のマッピングと
  // `Debug` フォーマットが有効になります。
  #[derive(Deserialize, Debug)]
  struct BlockResponse {
    // フィールドは `name: Type` の形で宣言されます。
    // この API は `blob` を hex 文字列として返却します。
    blob: String,
  }

  // RPC からブロックの JSON を取得するループ（接続エラー時にリトライ）
  // `rpc.json_rpc_call(..).await` は非同期呼び出しで `Result<T, E>` を返す。
  // `match` で `Ok` と `Err` を分岐して処理します。
  let res: BlockResponse = loop {
    match rpc.json_rpc_call("get_block", Some(json!({ "hash": hex::encode(hash) }))).await {
      // 成功したら結果を使う
      Ok(res) => break res,
      // 接続エラーはログを出して再試行
      Err(RpcError::ConnectionError(e)) => {
        println!("get_block ConnectionError: {e}");
        continue;
      }
      // その他はパニック
      Err(e) => panic!("couldn't get block {block_i} via block.hash(): {e:?}"),
    }
  };

  // hex 文字列をバイト列へ変換。
  // `hex::decode` は `Result<Vec<u8>, _>` を返し、`expect` は Err の場合に panic を発生させる。
  let blob = hex::decode(res.blob).expect("node returned non-hex block");
  // バイト列から Block をデシリアライズする
  // `Block::read` は Result を返すため、`unwrap_or_else` でエラー時の処理を指定している。
  let block = Block::read(&mut blob.as_slice())
    .unwrap_or_else(|e| panic!("couldn't deserialize block {block_i}: {e}"));
  // デシリアライズ後にハッシュが一致するか検証
  assert_eq!(block.hash(), hash, "hash differs");
  // シリアライズ後のバイト列が元の blob と一致するか検証
  assert_eq!(block.serialize(), blob, "serialization differs");

  // トランザクション数: coinbase を含めた合計
  // `vec.len()` はコレクションの要素数を返すメソッド
  let txs_len = 1 + block.transactions.len();

  // トランザクションが存在する場合は追加の検証を行う
  if !block.transactions.is_empty() {
    // プルーンド（省略された）トランザクションを取得するテスト
    loop {
      match rpc.get_pruned_transactions(&block.transactions).await {
        Ok(_) => break,
        Err(RpcError::ConnectionError(e)) => {
          println!("get_pruned_transactions ConnectionError: {e}");
          continue;
        }
        Err(e) => panic!("couldn't call get_pruned_transactions: {e:?}"),
      }
    }

    // フルトランザクションを RPC から取得（リトライあり）
    let txs = loop {
      match rpc.get_transactions(&block.transactions).await {
        Ok(txs) => break txs,
        Err(RpcError::ConnectionError(e)) => {
          println!("get_transactions ConnectionError: {e}");
          continue;
        }
        Err(e) => panic!("couldn't call get_transactions: {e:?}"),
      }
    };

    // バレットプルーフ等をバッチ検証するための検証器インスタンスを生成
    let mut batch = BatchVerifier::new();
    // 取得した各トランザクションを順に検査
    for tx in txs {
      match tx {
        // V1 トランザクション: prefix と署名が存在するはず
        Transaction::V1 { prefix: _, signatures } => {
          // 署名が空でないことを確認
          assert!(!signatures.is_empty());
          continue;
        }
        // V2 トランザクションで proofs が None（マイナーによる特殊ケース）ならパニック
        Transaction::V2 { prefix: _, proofs: None } => {
          panic!("proofs were empty in non-miner v2 transaction");
        }
        // V2 トランザクションで proofs が存在する通常ケース
        Transaction::V2 { ref prefix, proofs: Some(ref proofs) } => {
          // 署名ハッシュを取得（なければパニック）
          let sig_hash = tx.signature_hash().expect("no signature hash for TX with proofs");
          // 以下はサポートされる各種証明の検証処理
          // 一部のデバッグアサーションや CLSAG のマルチシグ実装で verify を呼ぶため、
          // signature_hash のアルゴリズムと verify の正当性を確認する目的がある
          match &proofs.prunable {
            // Borromean/MLSAG 系はここでは明示的な検証処理なし
            RctPrunable::AggregateMlsagBorromean { .. } | RctPrunable::MlsagBorromean { .. } => {}
            // Bulletproofs 系はバッチ検証に追加
            RctPrunable::MlsagBulletproofs { bulletproof, .. } |
            RctPrunable::MlsagBulletproofsCompactAmount { bulletproof, .. } => {
              assert!(bulletproof.batch_verify(
                &mut rand_core::OsRng,
                &mut batch,
                &proofs.base.commitments
              ));
            }
            // CLSAG 型: バレットプルーフの検証と CLSAG 検証を行う
            RctPrunable::Clsag { bulletproof, clsags, pseudo_outs } => {
              // バレットプルーフをバッチに追加
              assert!(bulletproof.batch_verify(
                &mut rand_core::OsRng,
                &mut batch,
                &proofs.base.commitments
              ));

              // 各 CLSAG に対して論理的に対応する出力群を取得して検証
              for (i, clsag) in clsags.iter().enumerate() {
                // トランザクションの対応する入力から amount, key_offsets, key_image を取得
                let (amount, key_offsets, image) = match &prefix.inputs[i] {
                  // coinbase 入力はここでは無効（存在するならパニック）
                  Input::Gen(_) => panic!("Input::Gen"),
                  // ToKey 型の入力の場合はフィールドを取得
                  Input::ToKey { amount, key_offsets, key_image } => {
                    (amount, key_offsets, key_image)
                  }
                };

                // オフセット（差分から実際のインデックスを計算）
                let mut running_sum = 0;
                let mut actual_indexes = vec![];
                for offset in key_offsets {
                  running_sum += offset;
                  actual_indexes.push(running_sum);
                }

                            // 内部非同期関数: RPC から出力情報（鍵とマスク）を取得して圧縮点に変換する
                            // 関数シグネチャの読み方:
                            // - `rpc: &impl Rpc` は参照を受け取り、トレイト境界を満たす任意の実装を許す
                            // - `indexes: &[u64]` は u64 のスライス参照を受け取る
                            // - 戻り値は `Vec<[CompressedPoint; 2]>`（圧縮点ペアのベクタ）
                            async fn get_outs(
                              rpc: &impl Rpc,
                              amount: u64,
                              indexes: &[u64],
                            ) -> Vec<[CompressedPoint; 2]> {
                  // RPC 応答の個別アウト構造体
                  #[derive(Deserialize, Debug)]
                  struct Out {
                    key: String,
                    mask: String,
                  }

                  // 複数アウトを包む構造体
                  #[derive(Deserialize, Debug)]
                  struct Outs {
                    outs: Vec<Out>,
                  }

                  // 指定したインデックス群で get_outs を呼ぶ（接続エラー時はリトライ）
                  let outs: Outs = loop {
                    match rpc
                      .rpc_call(
                        "get_outs",
                        Some(json!({
                          "get_txid": true,
                          // outputs は amount と index の配列を期待する
                          "outputs": indexes.iter().map(|o| json!({
                            "amount": amount,
                            "index": o
                          })).collect::<Vec<_>>()
                        })),
                      )
                      .await
                    {
                      Ok(outs) => break outs,
                      Err(RpcError::ConnectionError(e)) => {
                        println!("get_outs ConnectionError: {e}");
                        continue;
                      }
                      Err(e) => panic!("couldn't connect to RPC to get outs: {e:?}"),
                    }
                  };

                  // 16 進文字列を圧縮点に変換するクロージャ
                  // `let name = |args| expr;` はクロージャ（無名関数）の定義
                  let rpc_point = |point: &str| {
                    CompressedPoint::from(
                      <[u8; 32]>::try_from(
                        hex::decode(point).expect("invalid hex for ring member"),
                      )
                      .expect("invalid point len for ring member"),
                    )
                  };

                  // 取得した outs を圧縮点の配列に変換して返す
                  outs
                    .outs
                    .iter()
                    .map(|out| {
                      let mask = rpc_point(&out.mask);
                      // amount が 0 でない場合はマスクが期待されるコミットメントと一致するか検算
                      if amount != 0 {
                        assert_eq!(mask, Commitment::new(Scalar::ONE, amount).commit().compress());
                      }
                      [rpc_point(&out.key), mask]
                    })
                    .collect()
                }

                // CLSAG の検証を実行: RPC で得た出力群、イメージ、疑似アウト、署名ハッシュを渡す
                // 非同期関数の呼び出しは `.await` で待ち、戻り値を得る点に注意
                clsag
                  .verify(
                    get_outs(&rpc, amount.unwrap_or(0), &actual_indexes).await,
                    image,
                    &pseudo_outs[i],
                    &sig_hash,
                  )
                  .unwrap(); // `unwrap()` は Result の Ok 値を取り出し、Err の場合は panic する
              }
            }
          }
        }
      }
    }
    // バッチに追加された全ての検証を実行
    assert!(batch.verify());
  }

  // ログ出力: ブロック番号とトランザクション数を表示
  // `println!` はフォーマット付きマクロで、`{}` や `{name}` で変数を埋め込める
  println!("Deserialized, hashed, and reserialized {block_i} with {txs_len} TXs");
}

// Tokio のメイン関数マクロ: 非同期ランタイムのエントリポイントを定義
// `#[tokio::main]` はこの関数のために Tokio 実行器をセットアップします。
#[tokio::main]
async fn main() {
  // コマンドライン引数をベクタに収集する
  // `std::env::args()` はイテレータを返し、`collect::<Vec<_>>()` で Vec に変換
  let args = std::env::args().collect::<Vec<String>>();

  // 最初の引数を開始ブロック番号としてパースする（存在しないと panic）
  let mut block_i =
    args.get(1).expect("no start block specified").parse::<usize>().expect("invalid start block");

  // 同時に処理するブロック数（並列度）を 2 番目の引数から取得、デフォルトは 8
  let async_parallelism: usize =
    args.get(2).unwrap_or(&"8".to_string()).parse::<usize>().expect("invalid parallelism argument");

  // 追加の引数は RPC エンドポイントの URL 列と見なす
  let default_nodes = vec![
    "http://xmr-node-uk.cakewallet.com:18081".to_string(),
    "http://xmr-node-eu.cakewallet.com:18081".to_string(),
  ];
  let mut specified_nodes = vec![];
  {
    // args[3..] を走査して指定ノードを収集
    // `let Some(x) = opt else {}` は Option の分岐を簡潔に表す構文（Rust 1.65 以降）
    let mut i = 0;
    loop {
      let Some(node) = args.get(3 + i) else { break };
      specified_nodes.push(node.clone());
      i += 1;
    }
  }
  // 指定がなければデフォルトノードを使用、あれば指定ノードを使用
  let nodes = if specified_nodes.is_empty() { default_nodes } else { specified_nodes };

  // クロージャ: URL から SimpleRequestRpc を生成する非同期関数を作る
  // `|url: String| async move { ... }` は引数を受け取る非同期クロージャ。
  // `move` はクロージャが外側の変数を所有（move）することを意味します。
  let rpc = |url: String| async move {
    SimpleRequestRpc::new(url.clone())
      .await
      .unwrap_or_else(|_| panic!("couldn't create SimpleRequestRpc connected to {url}"))
  };
  // メイン RPC クライアントを最初のノードで作成
  let main_rpc = rpc(nodes[0].clone()).await;
  // 並列度分の RPC クライアントを作る（ノードはラウンドロビンで選択）
  let mut rpcs = vec![];
  // range 構文 `start..end` は start から end-1 までの値を生成します。
  for i in 0 .. async_parallelism {
    rpcs.push(rpc(nodes[i % nodes.len()].clone()).await);
  }

  // ラウンドロビン用のインデックスとタスクハンドル、チェーン高さを初期化
  let mut rpc_i = 0;
  let mut handles: Vec<JoinHandle<()>> = vec![];
  let mut height = 0;
  loop {
    // ノードから現在のチェーン高さを取得
    let new_height = main_rpc.get_height().await.expect("couldn't call get_height");
    if new_height == height {
      // 高さが変わらなければループを抜ける
      break;
    }
    height = new_height;

    // 未処理のブロックがある限り spawn でタスクを追加
    while block_i < height {
      if handles.len() >= async_parallelism {
        // 少なくとも 1 つのハンドルが完了するのを保証する
        handles.swap_remove(0).await.unwrap();

        // 完了しているハンドルをすべて取り除くループ
        let mut i = 0;
        while i < handles.len() {
          if handles[i].is_finished() {
            handles.swap_remove(i).await.unwrap();
            continue;
          }
          i += 1;
        }
      }

      // 非同期タスクとして `check_block` を spawn し、ハンドルを保持
      // `tokio::spawn(...)` は `JoinHandle` を返し、タスクの終了を待てます。
      handles.push(tokio::spawn(check_block(rpcs[rpc_i].clone(), block_i)));
      // 次の RPC クライアントをラウンドロビンで選択
      rpc_i = (rpc_i + 1) % rpcs.len();
      // 次のブロック番号へ進める
      block_i += 1;
    }
  }
}
