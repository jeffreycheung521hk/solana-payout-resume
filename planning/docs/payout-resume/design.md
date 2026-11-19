# 付款中斷續接設計

> 文件歸屬：solana-payout-resume
> 涉及主體：solana-payout-resume

本文件未標註的內容皆為已確認。

## 要解決的問題

付款 Agent 送出一筆 SPL Token 付款之後、得知結果之前中斷。接手者必須在三件事之間選一件：繼續等、重送同一份交易、簽一筆新的交易。選錯的後果：過早簽新的一筆，舊的與新的可能都被處理，造成重複付款；放棄不送，收款方收不到款。

## 範例範圍

- 只做離線模擬：用模擬鏈重現交易落地、掉單、過期，以及節點帳冊的起點。
- 簽章與已簽位元組是合成資料，不是有效的 Solana 交易，不能拿到真鏈上查詢。

## 名詞

- 一筆付款以 payout_id 識別，例如發票號碼；一筆付款可以有多次嘗試（Attempt），每次嘗試是一筆交易，有自己的交易第一簽章。「簽新的一筆」指同一筆付款底下的新嘗試。
- 落地：交易被收進區塊，不論執行成功或失敗。

## 付款紀錄

- 每筆付款一份付款紀錄（PayoutRecord），存成一個 JSON 檔。內容：payout_id；付款內容，也就是付款人錢包擁有人、收款人錢包擁有人、mint、以最小單位整數表示的金額；依簽署順序排列的嘗試。
- 一次嘗試記錄四件事：交易第一簽章、可以原封不動重送的已簽位元組、最後有效區塊高度、取得 blockhash 時的 slot。
- 寫入順序：先寫暫存檔並 fsync，再用 rename 蓋掉舊檔，最後 fsync 所在資料夾。這幾步完成之後才廣播交易。
  - 依據：同一個檔案系統內 rename 一次完成，接手者讀到的一定是完整的舊版或新版；資料夾 fsync 之後改名才算寫進磁碟，斷電後已廣播的嘗試一定在紀錄裡。
- 相依套件：serde、serde_json、bs58。

## 續接判斷

- 接手者只依付款紀錄和鏈上觀察（ChainObservation）判斷下一步，不依賴對話紀錄或記憶。
- 判斷寫成純函式續接判斷（next_action）：輸入付款紀錄與鏈上觀察，輸出下一步（NextAction），四種之一：
  - 完成（Done）：交易已在 finalized 落地且執行成功。
  - 重送同一份（Rebroadcast）：還沒看到落地，也還沒過期。重送的位元組與簽章都不變；這也包含繼續等。
  - 簽新的一筆（SignNewAttempt）：舊交易已證明過期，而且查遍歷史都沒有落地。
  - 交給人（Escalate）：證據不足以支持以上任何一種；附上交給人的原因（EscalationReason）。

### 查遍歷史都沒有

- 兩個條件同時成立才算：
  - 節點最早可用的 slot（`getFirstAvailableBlock`）不晚於該次嘗試取得 blockhash 時的 slot。
  - 帶 `searchTransactionHistory` 查該簽章的結果（HistoryLookup）是找不到。
- 任一條件不成立就交給人。
- 依據：一般 RPC 節點只保存大約最近兩天的鏈上資料，超出範圍的查詢同樣回 null，所以 null 本身不是沒落地的證據（[Solana archival data](https://www.alchemy.com/overviews/solana-archival-data)）。`getFirstAvailableBlock` 回傳節點帳冊裡還沒被清掉的最早區塊 slot（[getFirstAvailableBlock](https://docs.metamask.io/services/reference/solana/json-rpc-methods/getfirstavailableblock/)）。交易不可能在取得 blockhash 之前落地，所以最早可用的 slot 不晚於那個 slot，代表節點帳冊涵蓋整段可能落地的期間。
- 剩餘風險：最早可用的 slot 只證明起點，不能證明中間沒有缺口；兩個查詢若經過負載平衡打到不同節點，結果可能對不上。這些風險由嘗試次數上限兜底。

### 自動簽新的一筆與上限

- 過期且查遍歷史都沒有時，Agent 不需人核准就簽新的一筆。
- 一筆付款最多 3 次嘗試，含第一次；達到上限時交給人。最壞情況是判斷每次都錯，同一筆付款被付 3 次。
- 交易落地但執行失敗時交給人。失敗多半出在輸入或帳戶狀態，原封不動再簽一次會以同樣原因再失敗，每次還要付手續費。

### 未決

- 付款紀錄遺失時的處理：未決。
- 落地但執行失敗、交給人之後，用同一個 payout_id 開新嘗試，還是另開新的 payout_id：未決。
- 同一筆付款同時被兩個接手者續接：未決。

## 對帳匯出

匯出給 web3testproj1 的規則見 [付款續接與對帳的匯出介面](../shared/reconcile-export.md)。

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

### 直接相信帶 searchTransactionHistory 的 null

- 要解決的問題：少一個欄位、少一個查詢。
- 缺陷：換了 RPC 供應商或節點設定，節點不再保存那段帳冊時，null 只代表沒有資料；程式自己察覺不到，會簽新的一筆而重複付款。
- 現行設計的處理：先用 `getFirstAvailableBlock` 證明帳冊涵蓋那段期間，再相信 null。

### 自動簽新的一筆不設上限，或每次都要人核准

- 要解決的問題：不設上限是為了完全自動；每次核准是為了完全不依賴判斷邏輯。
- 缺陷：不設上限時，判斷邏輯若有錯，沒有任何關卡擋住；每次核准時，網路壅塞下每一次過期都要等人。
- 現行設計的處理：自動簽，但一筆付款最多 3 次嘗試。

### 付款紀錄改用逐行文字格式或 SQLite

- 要解決的問題：逐行文字格式不需要相依套件；SQLite 與 web3testproj1 一致。
- 缺陷：手寫解析多約 150–200 行要審，而且最容易藏錯；SQLite 需要 C 編譯工具，對一筆付款一份紀錄來說太重。
- 現行設計的處理：每筆付款一個 JSON 檔，用 serde、serde_json、bs58。
