# 付款中斷續接分表

> 文件歸屬：solana-payout-resume
> 涉及主體：solana-payout-resume

## 本專案自造

| 中文術語 | 英文名／識別符 | 概念 | 來源 | 狀態 | 規範位置 | 別名與舊叫法 |
|---|---|---|---|---|---|---|
| 付款紀錄 | `PayoutRecord` | 一筆付款在每次送出前寫入的紀錄：付款內容與這筆付款的所有嘗試；接手者在本機唯一的判斷依據 | 自造 | 已確認（設計基線，2026-11-19） | `payout-resume/design.md`「付款紀錄」；`source/src/record.rs` | 紀錄；模組 record |
| 嘗試 | `Attempt` | 一次簽署產生的一筆交易：交易第一簽章、可以原封不動重送的已簽位元組、最後有效區塊高度、取得 blockhash 時的 slot | 自造 | 已確認（設計基線，2026-11-19） | 同上 | 無 |
| 鏈上觀察 | `ChainObservation` | 判斷當下從 RPC 取得的事實，例如 finalized 區塊高度、節點最早可用的 slot、各簽章的查詢結果；本身不發出查詢 | 自造 | 已確認（按命名規則 4 定名，2026-11-19） | `payout-resume/design.md`「續接判斷」；`source/src/observation.rs` | ChainView（示意名）；模組 observation |
| 下一步 | `NextAction` | 續接判斷的結果，四種之一 | 自造 | 已確認（按命名規則 4 定名，2026-11-19） | `payout-resume/design.md`「續接判斷」；`source/src/decision.rs` | Next（示意名）；變體：完成（Done）、重送同一份（Rebroadcast）、簽新的一筆（SignNewAttempt）、交給人（Escalate） |
| 續接判斷 | `next_action` | 由付款紀錄和鏈上觀察算出下一步的純函式 | 自造 | 已確認（按命名規則 1 定名，2026-11-19） | 同上 | next_step（示意名）；模組 decision |
| 交給人的原因 | `EscalationReason` | 下一步是交給人時附上的原因，一種原因一個變體 | 自造 | 已確認（按命名規則 2 定名，2026-11-19） | `source/src/decision.rs` | Reason（示意名） |
| 查歷史的結果 | `HistoryLookup` | 帶 searchTransactionHistory 查一個簽章的結果：找到、找不到、查詢失敗 | 自造 | 已確認（設計基線，2026-11-19） | `payout-resume/design.md`「查遍歷史都沒有」；`source/src/observation.rs` | 變體：Found、NotFound、Unavailable |
| 執行結果 | `ExecutionOutcome` | 已落地交易的執行結果：成功，或失敗但手續費照扣 | 自造 | 已確認（按命名規則 4 定名，2026-11-19） | `source/src/observation.rs` | 變體：Succeeded、Failed |
| 嘗試次數上限 | `MAX_ATTEMPTS` | 一筆付款最多幾次嘗試，含第一次；目前是 3 | 自造 | 已確認（設計基線，2026-11-19） | `payout-resume/design.md`「自動簽新的一筆與上限」；`source/src/decision.rs` | 無 |
| 付款紀錄存放處 | `PayoutRecordStore` | 把每筆付款存成一個 JSON 檔、用 rename 原子替換的元件；新增嘗試的呼叫回傳之後才可以廣播 | 自造 | 已確認（按命名規則 3 定名，2026-11-19） | `payout-resume/design.md`「付款紀錄」；`source/src/store.rs` | 模組 store；StoreError；MAX_PAYOUT_ID_LEN；append_attempt |
| 對帳匯出 | `reconcile_csv` | 把付款的每次嘗試匯出成 web3testproj1 的輸入 CSV，完成的那次排第一 | 自造 | 已確認（按命名規則 1 定名，2026-11-19） | `shared/reconcile-export.md`；`source/src/export.rs` | 模組 export；reconcile_rows；RECONCILE_CSV_HEADER；ExportError；MAX_PAYMENT_ID_LEN |
| 模擬鏈 | `SimulatedChain` | 離線、可重現的鏈替身，供測試與範例使用；簽章與已簽位元組是合成的 | 自造 | 已確認（設計基線「範例範圍」，2026-11-19） | `payout-resume/design.md`「範例範圍」；`source/src/simulation.rs` | 模組 simulation；BroadcastFate；BlockhashInfo；Interruption；SessionEnd；run_session；run_session_with；sign_attempt；BLOCKHASH_VALIDITY_BLOCKS；POLL_INTERVAL_BLOCKS；MAX_SESSION_STEPS |
| 對照用的錯誤策略 | `contrast` | 故意寫錯的續接策略，用來對照續接判斷防住了什麼：不等過期就重簽、相信不涵蓋的歷史 | 自造 | 已確認（按命名規則 4 定名，2026-11-19） | `source/src/simulation.rs` | resign_when_unseen；trust_null_history |

## 外部既有

| 中文術語 | 英文名／識別符 | 概念 | 來源 | 狀態 | 規範位置 | 別名與舊叫法 |
|---|---|---|---|---|---|---|
| 交易第一簽章 | `Signature` | 交易的第一個簽章，也是交易 id；查詢交易狀態時用它；以 base58 表示 | 外部：Solana | 已確認（外部既有名稱） | `payout-resume/design.md`「付款紀錄」；`source/src/record.rs` | signature；交易簽章；交易 id |
| 地址 | `Address` | 32 bytes 的 Solana 地址，以 base58 表示；本專案用於錢包擁有人與 mint | 外部：Solana | 已確認（外部既有名稱） | `source/src/record.rs` | Base58Error |
| 簽章狀態 | `SignatureStatus` | `getSignatureStatuses` 對已落地交易回傳的狀態：slot、是否 finalized、執行結果 | 外部：Solana RPC | 已確認（外部既有名稱） | `source/src/observation.rs` | 無 |
| 最後有效區塊高度 | `last_valid_block_height` | 取得 blockhash 時一併回傳的高度；區塊高度超過它，使用該 blockhash 的交易不會再被處理 | 外部：Solana RPC `getLatestBlockhash` 的 `lastValidBlockHeight` | 已確認（外部既有名稱） | `payout-resume/design.md`「依 Solana 官方文件定下的規則」 | lastValidBlockHeight |
| 落地 | landed | 交易被收進區塊，不論執行成功或失敗 | 外部：Solana 社群用語 | 已確認（外部既有名稱） | `payout-resume/design.md`「名詞」 | land |
| 過期 | expired | 舊交易的 blockhash 已失效：finalized 區塊高度超過最後有效區塊高度 | 外部：Solana 官方文件 | 已確認（外部既有名稱） | `payout-resume/design.md`「依 Solana 官方文件定下的規則」 | blockhash 過期 |
|  | finalized | commitment 等級；Alpenglow 上線後與 confirmed 描述同一個狀態 | 外部：Solana | 已確認（外部既有名稱） | `payout-resume/design.md`「依 Solana 官方文件定下的規則」 | confirmed（同一狀態，官方將移除） |
|  | `searchTransactionHistory` | `getSignatureStatuses` 的參數；為 true 時查節點保存的歷史，不只近期的狀態快取 | 外部：Solana RPC | 已確認（外部既有名稱） | `payout-resume/design.md`「依 Solana 官方文件定下的規則」 | 無 |
| 最早可用的 slot | `getFirstAvailableBlock` | RPC 方法；回傳節點帳冊裡還沒被清掉的最早區塊 slot，用來證明帳冊涵蓋某段期間 | 外部：Solana RPC | 已確認（外部既有名稱） | `payout-resume/design.md`「查遍歷史都沒有」 | first_available_slot |
|  | `payment_id_reuse` | web3testproj1 的待查原因代碼：同一個 payment_id 第二次出現；該列自己的鏈上判定保留在 `original_reason=` | 外部：web3testproj1 | 已確認（外部既有名稱） | `shared/reconcile-export.md`「web3testproj1 的輸入格式」 | signature_reuse（同類代碼） |
