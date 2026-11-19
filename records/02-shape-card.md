# 形狀卡｜付款送出後中斷，接手的 Session 怎麼知道錢有沒有付出去？

以下是呈現給使用者的原文（2026-11-19）；引用的來源改成文末連結。

## 原文

**情境**　Agent 要付 120 USDC 給供應商（payout_id = INV-001）。它簽好交易、送出之後，Session 就中斷了。新 Session 接手時，這筆交易可能已經落地、還在路上，或者已經掉了。新 Session 只有三個選擇：繼續等、重送同一筆、簽一筆新的。選錯有代價：太早簽新的，兩筆可能都落地，等於付了 240；放棄不送，供應商就收不到錢。

**建議形狀 1a：送出前先寫紀錄，續接時只看紀錄和鏈上查詢**

```rust
pub struct Attempt {                // 送出「之前」先寫入磁碟
    signature: Signature,           // 交易第一簽章，也就是交易 id
    signed_tx: Vec<u8>,             // 已簽好的原始位元組，可以原封不動重送
    last_valid_block_height: u64,   // 鏈上高度超過它，這筆就永遠不會被處理
}
pub struct PayoutRecord { payout_id: String, attempts: Vec<Attempt> }

pub enum Next {
    Done { signature: Signature },        // 已在 finalized 落地且成功
    Rebroadcast { signature: Signature }, // 還沒看到落地、也還沒過期：重送同一份位元組
    SignNewAttempt,                       // 已過期，而且查遍歷史都沒有：這時才可以簽新的
    Escalate { reason: Reason },          // 證據不足：交給人處理
}
pub fn next_step(record: &PayoutRecord, chain: &ChainView) -> Next
```

**這個形狀代表什麼**　接手的 Session 不用讀聊天紀錄，只看這份紀錄和鏈上查詢結果。判斷寫成純函式，可以用固定的測試資料驗證。重送同一份位元組時簽章不變，而且 RPC 節點本來就會每兩秒重送同一筆，直到 finalized 或 blockhash 過期[^retry]。真正會造成重複付款的是「簽一筆新的」，所以要等到證明舊的已過期才允許。這就是 Notes 說的「換 Session 前先寫交接」，只是對象從寫程式的 Agent 換成付款的 Agent。紀錄裡的簽章之後也能直接匯出成 web3testproj1 的輸入 CSV，讓它獨立對帳。

**替代形狀**

- **1b 事後用 reference 去鏈上找**：送出前不寫紀錄，每筆付款帶一個唯一的 reference key。這是 Solana Pay 的做法，事後可以用 getSignaturesForAddress 找到交易[^pay]。好處是本機紀錄遺失也找得回來。代價有三個：找不到時，要等所有可能送出的交易都過期才能下結論；要看 RPC 的地址索引和保留多久的歷史；web3testproj1 目前只核對已知簽章，不支援 reference。
- **1c 鏈上保證只執行一次（離線簽署程式）**：簽一份不含 blockhash 的訊息，程式的 nonce 讓同一份訊息不能執行兩次；執行失敗時 nonce 會回滾，可以再送一次[^nonce]。好處是完全不用跟過期時間賽跑。代價是候選程式有 Anza 的 Programmatic Signer 和 Blueshift 的 Vector，官方還沒選定，範例只能先押其中一個。

**照協定直接定下的部分（不需要你裁決）**

- commitment 一律用 finalized：官方建議新程式改用 finalized，confirmed 之後會移除[^alpenglow]。
- 重新簽署之前，一定要先確認原交易的 blockhash 已經過期[^retry]。判斷方法是記下 getLatestBlockhash 回傳的 lastValidBlockHeight，再查 getBlockHeight，直到高度超過它[^confirmation]，commitment 照上一點用 finalized。
- getSignatureStatuses 預設只查近期的狀態快取，查不到不代表沒落地，所以下結論前一律加上 `searchTransactionHistory`[^status]。
- 不採用 durable nonce，理由是官方已宣布將移除[^nonce]。

**問題**　接手的 Session 要靠什麼判斷錢有沒有付出去：**1a 送出前先寫紀錄**，還是 **1b 事後用 reference 去鏈上找**？1c 我建議先列入「不採用的方案」，等官方定案再說。

## 裁決

- 使用者第一次回覆時，處理了建立 repo（「Ready 了」）和同意子代理審查（「同意」），沒有回覆形狀卡。照「沒提到的項算未看，不算採納」，沒有把推薦當成採納，而是用選項式提問再問一次。
- 第二次提問的結果（2026-11-19）：

| 問題 | 使用者選擇 |
|---|---|
| 接手的 Session 要靠什麼判斷錢有沒有付出去 | 1a 送出前先寫紀錄 |
| 替代形狀 1c 要怎麼處理 | 列入不採用的方案 |

[^retry]: [Retrying Transactions](https://solana.com/developers/guides/advanced/retry)
[^pay]: [Solana Pay Specification](https://docs.solanapay.com/spec)
[^nonce]: [Durable Nonce Deprecation](https://solana.com/ko/upgrades/durable-nonce-deprecation)
[^alpenglow]: [Alpenglow](https://solana.com/upgrades/alpenglow)
[^confirmation]: [Transaction Confirmation & Expiration](https://solana.com/docs/advanced/confirmation)
[^status]: [getSignatureStatuses](https://solana.com/docs/rpc/http/getsignaturestatuses)
