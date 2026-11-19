# 付款續接與對帳的匯出介面

> 文件歸屬：跨主體共同基線（solana-payout-resume、web3testproj1）
> 涉及主體：solana-payout-resume

輸入格式由 web3testproj1 決定；本文件只規範 solana-payout-resume 的匯出端。

## web3testproj1 的輸入格式

- UTF-8 CSV，標頭與順序固定：

```text
payment_id,tx_signature,expected_sender_owner,expected_recipient_owner,expected_mint,expected_amount_atomic
```

- payment_id 是 1–256 bytes 的業務識別碼；tx_signature 是 base58 解碼為 64 bytes 的交易第一簽章；三個地址是 base58 解碼為 32 bytes 的錢包擁有人或 mint；金額是 1 到 18446744073709551615 的十進位最小單位整數，不接受前導零。欄位可以加標準引號，不支援欄位內換行。
- 同一個 payment_id 或簽章第二次出現時，那一列改標待查（needs_review），原因是 `payment_id_reuse` 或 `signature_reuse`；該列自己的鏈上判定保留在 `reason_details` 的 `original_reason=`。
- 來源：web3testproj1 的 README「CSV 輸入」一節，以及原始碼 `src/store.rs`、`src/reason.rs`（[web3testproj1](https://github.com/jeffreycheung521hk/web3testproj1)）。

## 匯出規則

- payout_id 寫進 payment_id 欄；付款內容的付款人、收款人、mint、金額分別寫進對應欄位。
- 每次嘗試都匯出一列。續接判斷為完成（Done）的那次排第一，其餘依簽署順序排列；沒有完成的付款，全部依簽署順序排列。
- 判讀方式：第一列吻合即為 matched。其餘列為待查，`original_reason` 是 `signature_not_found…` 表示該次嘗試沒落地，屬於正常；是 `finalized_direct_transfer_verified` 表示該次也落地而且轉帳成功，等於重複付款。
- 範例範圍是離線模擬，匯出的簽章是合成的，只能檢查格式，不能上鏈核對。

## 不採用的方案

### 只匯出判定成功的那一次

- 要解決的問題：報告乾淨，每筆付款一列。
- 缺陷：對帳工具看不到其他嘗試，續接邏輯的錯誤（例如兩次嘗試都落地）無法被獨立發現。
- 現行設計的處理：每次嘗試都匯出，靠 web3testproj1 的 `payment_id_reuse` 與 `original_reason` 獨立揭露。
