# 術語衝突清單

> 文件歸屬：solana-payout-resume
> 涉及主體：solana-payout-resume

| 叫法 | 衝突 | 推薦叫法 |
|---|---|---|
| 重試 | 一名多義：可指重送同一份已簽位元組（簽章不變，不會重複付款），也可指簽新的一筆（新簽章，可能重複付款） | 分別寫「重送同一份」（Rebroadcast）與「簽新的一筆」（SignNewAttempt） |
| 確認 | 一名多義：設計結論的狀態「已確認」、commitment 等級 confirmed、交易已被處理 | 設計狀態寫「已確認」；commitment 寫 finalized；交易已被處理寫「落地」 |
| reason | 一名多義：web3testproj1 報告的 `reason` 欄是對帳結果代碼；本專案的是交給人的原因 | 本專案寫「交給人的原因」（EscalationReason） |
