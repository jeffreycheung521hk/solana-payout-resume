# 付款中斷續接分表

> 文件歸屬：solana-payout-resume
> 涉及主體：solana-payout-resume

## 本專案自造

| 中文術語 | 英文名／識別符 | 概念 | 來源 | 狀態 | 規範位置 | 別名與舊叫法 |
|---|---|---|---|---|---|---|
| 付款紀錄 | `PayoutRecord` | 一筆付款在每次送出前寫入的紀錄，含這筆付款的所有嘗試；接手者在本機唯一的判斷依據 | 自造 | 已確認（設計基線，2026-11-19） | `payout-resume/design.md`「續接依據：送出前寫入的付款紀錄」 | 紀錄 |
| 嘗試 | `Attempt` | 一次簽署產生的一筆交易：交易第一簽章、可以原封不動重送的已簽位元組、最後有效區塊高度 | 自造 | 已確認（設計基線，2026-11-19） | 同上 | 無 |
| 鏈上觀察 | `ChainObservation` | 判斷當下從 RPC 取得的事實，例如 finalized 區塊高度與各簽章的查詢結果；本身不發出查詢 | 自造 | 已確認（按命名規則 4 定名，2026-11-19） | 同上 | ChainView（示意名） |
| 下一步 | `NextAction` | 續接判斷的結果，四種之一 | 自造 | 已確認（按命名規則 4 定名，2026-11-19） | 同上 | Next（示意名）；變體：完成（Done）、重送同一份（Rebroadcast）、簽新的一筆（SignNewAttempt）、交給人（Escalate） |
| 續接判斷 | `next_action` | 由付款紀錄和鏈上觀察算出下一步的純函式 | 自造 | 已確認（按命名規則 1 定名，2026-11-19） | 同上 | next_step（示意名） |
| 交給人的原因 | `EscalationReason` | 下一步是交給人時附上的原因 | 自造 | 已確認（按命名規則 2 定名，2026-11-19） | 同上 | Reason（示意名） |
| 查歷史的結果 | `HistoryLookup` | 帶 searchTransactionHistory 查一個簽章的結果：找到、找不到、查詢失敗 | 自造 | 已確認（設計基線，2026-11-19） | `payout-resume/design.md`「查遍歷史都沒有」 | 變體：Found、NotFound、Unavailable |

## 外部既有

| 中文術語 | 英文名／識別符 | 概念 | 來源 | 狀態 | 規範位置 | 別名與舊叫法 |
|---|---|---|---|---|---|---|
| 交易第一簽章 | `signature` | 交易的第一個簽章，也是交易 id；查詢交易狀態時用它 | 外部：Solana | 已確認（外部既有名稱） | `payout-resume/design.md`「續接依據：送出前寫入的付款紀錄」 | 交易簽章、交易 id |
| 最後有效區塊高度 | `last_valid_block_height` | 取得 blockhash 時一併回傳的高度；區塊高度超過它，使用該 blockhash 的交易不會再被處理 | 外部：Solana RPC `getLatestBlockhash` 的 `lastValidBlockHeight` | 已確認（外部既有名稱） | `payout-resume/design.md`「依 Solana 官方文件定下的規則」 | lastValidBlockHeight |
| 落地 | landed | 交易已被處理並記錄在某個 slot，不論執行成功或失敗 | 外部：Solana 社群用語 | 已確認（外部既有名稱） | `payout-resume/design.md`「續接依據：送出前寫入的付款紀錄」 | land |
| 過期 | expired | 舊交易的 blockhash 已失效：finalized 區塊高度超過最後有效區塊高度 | 外部：Solana 官方文件 | 已確認（外部既有名稱） | `payout-resume/design.md`「依 Solana 官方文件定下的規則」 | blockhash 過期 |
|  | finalized | commitment 等級；Alpenglow 上線後與 confirmed 描述同一個狀態 | 外部：Solana | 已確認（外部既有名稱） | `payout-resume/design.md`「依 Solana 官方文件定下的規則」 | confirmed（同一狀態，官方將移除） |
|  | `searchTransactionHistory` | `getSignatureStatuses` 的參數；為 true 時查節點保存的歷史，不只近期的狀態快取 | 外部：Solana RPC | 已確認（外部既有名稱） | `payout-resume/design.md`「依 Solana 官方文件定下的規則」 | 無 |
| 最早可用的 slot | `getFirstAvailableBlock` | RPC 方法；回傳節點帳冊裡還沒被清掉的最早區塊 slot，用來證明帳冊涵蓋某段期間 | 外部：Solana RPC | 已確認（外部既有名稱） | `payout-resume/design.md`「查遍歷史都沒有」 | first_available_slot |
|  | `payment_id_reuse` | web3testproj1 的待查原因代碼：同一個 payment_id 第二次出現；該列自己的鏈上判定保留在 `original_reason=` | 外部：web3testproj1 | 已確認（外部既有名稱） | `shared/reconcile-export.md`「web3testproj1 的輸入格式」 | signature_reuse（同類代碼） |
