# 付款中斷續接設計

> 文件歸屬：solana-payout-resume
> 涉及主體：solana-payout-resume

本文件未標註的內容皆為已確認。

## 要解決的問題

付款 Agent 送出一筆 SPL Token 付款之後、得知結果之前中斷。接手者必須在三件事之間選一件：繼續等、重送同一份交易、簽一筆新的交易。選錯的後果：過早簽新的一筆，舊的與新的可能都被處理，造成重複付款；放棄不送，收款方收不到款。

## 續接依據：送出前寫入的付款紀錄

- 每一次嘗試（Attempt）在廣播之前寫入付款紀錄（PayoutRecord）。一次嘗試記錄三件事：交易第一簽章、可以原封不動重送的已簽位元組、最後有效區塊高度。
- 接手者只依付款紀錄和鏈上觀察（ChainObservation）判斷下一步，不依賴對話紀錄或記憶。
- 判斷寫成純函式續接判斷（next_action）：輸入付款紀錄與鏈上觀察，輸出下一步（NextAction），四種之一：
  - 完成（Done）：交易已在 finalized 落地且執行成功。
  - 重送同一份（Rebroadcast）：還沒看到落地，也還沒過期。重送的位元組與簽章都不變。
  - 簽新的一筆（SignNewAttempt）：舊交易已證明過期，而且查遍歷史都沒有落地。
  - 交給人（Escalate）：證據不足以支持以上任何一種；附上交給人的原因（EscalationReason）。
- 「查遍歷史都沒有」的判定標準：未決。
- 付款紀錄遺失時的處理：未決。

## 依 Solana 官方文件定下的規則

- commitment 一律用 finalized。依據：Alpenglow 上線後，confirmed 與 finalized 描述同一個狀態；官方建議新程式改用 finalized，confirmed 之後會移除。（[Alpenglow](https://solana.com/upgrades/alpenglow)）
- 簽新的一筆之前，必須先確認舊交易的 blockhash 已過期。（[Retrying Transactions](https://solana.com/developers/guides/advanced/retry)）
- 過期的判斷：記下取得 blockhash 時一併回傳的最後有效區塊高度；finalized 區塊高度超過它之後，舊交易不會再被處理。（[Transaction Confirmation & Expiration](https://solana.com/docs/advanced/confirmation)）
- `getSignatureStatuses` 預設只查近期的狀態快取，查不到不代表沒落地；判斷是否落地時一律帶 `searchTransactionHistory`。（[getSignatureStatuses](https://solana.com/docs/rpc/http/getsignaturestatuses)）
- 重送同一份已簽位元組不會產生新的簽章；RPC 節點本身也會重送同一筆交易，直到 finalized 或 blockhash 過期。（[Retrying Transactions](https://solana.com/developers/guides/advanced/retry)）

## 不採用的方案

### 事後以 reference key 在鏈上搜尋

- 要解決的問題：本機付款紀錄遺失時，仍能找回付款。做法是每筆付款帶一個唯一的 reference key，事後用 `getSignaturesForAddress` 搜尋。（[Solana Pay 規格](https://docs.solanapay.com/spec)）
- 缺陷：查不到時，必須等所有可能送出的交易都過期才能下結論；依賴 RPC 的地址索引與歷史保存深度；下游的對帳工具 web3testproj1 只核對已知簽章。
- 現行設計的處理：付款紀錄在送出前就帶著簽章與最後有效區塊高度，接手者直接查那個簽章，不需要搜尋。

### 鏈上保證只執行一次的離線簽署程式

- 要解決的問題：消除和過期時間的競賽。簽一份不含 blockhash 的訊息，由程式的 nonce 保證同一份訊息最多執行一次；執行失敗時 nonce 回滾，可以再送。
- 缺陷：官方尚未在候選程式（Anza 的 Programmatic Signer、Blueshift 的 Vector）之間定案。（[Durable Nonce Deprecation](https://solana.com/ko/upgrades/durable-nonce-deprecation)）
- 現行設計的處理：用最後有效區塊高度證明舊交易不會再被處理，之後才簽新的一筆。

### durable nonce

- 要解決的問題：交易不會過期，可以先簽、晚點再送。
- 缺陷：官方已宣布將移除，尚無日期。（[Durable Nonce Deprecation](https://solana.com/ko/upgrades/durable-nonce-deprecation)）
- 現行設計的處理：同上。
