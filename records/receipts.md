# 回執

## 送出前寫入付款紀錄（2026-11-19）

- 對應提交：`23efe8a`（設計基線、術語表、待辦帳本）。
- 落實 2 項：
  - 形狀 1a：送出前把嘗試寫入付款紀錄；接手者只依紀錄和鏈上觀察判斷；下一步分完成、重送同一份、簽新的一筆、交給人四種。
  - 1c（鏈上保證只執行一次的離線簽署程式）列入「不採用的方案」。
- 回覆 0 項。
- 未看 0 項。
- 隨同寫進基線、不需裁決的內容：
  - 依 Solana 官方文件定下的 5 條規則（finalized、過期後才簽新的一筆、過期的判斷方式、查全歷史、重送不產生新簽章），各附來源。
  - 沒被選上的 1b 與 durable nonce 寫進「不採用的方案」，因為之後很可能有人再提。
- 新名字（按命名規則定名，可以事後改）：

| 名字 | 含義 | 依據 | 不採用 |
|---|---|---|---|
| `PayoutRecord` | 一筆付款在每次送出前寫入的紀錄 | 形狀卡原名 | 無 |
| `Attempt` | 一筆付款底下的一次簽署交易 | 形狀卡原名 | 無 |
| `NextAction` | 續接判斷的結果 | 規則 4：名字要說出它是什麼 | `Next` |
| `next_action` | 算出下一步的純函式 | 規則 1：操作函式隨類型名改 | `next_step` |
| `ChainObservation` | 一次判斷所用的鏈上事實，本身不發查詢 | 規則 4：`View` 容易被讀成會自己去查的即時連線 | `ChainView` |
| `EscalationReason` | 交給人時附的原因 | 規則 2：web3testproj1 的報告已有 `reason` 欄 | `Reason` |

- 尚未寫回：決策包 5 項（等待裁決），以及「付款紀錄遺失」「落地失敗後用同一個或新的 payout_id」兩項（下一包）。

## 第一個決策包（2026-11-19）

- 對應提交：`11512a8`（設計基線、匯出介面、待辦帳本）。
- 落實 5 項：範例範圍（只做離線模擬）；付款紀錄格式與相依（每筆一個 JSON 檔，serde、serde_json、bs58）；「查遍歷史都沒有」的標準（帳冊涵蓋證明）；自動簽新的一筆（上限 3 次嘗試，落地失敗交給人）；匯出範圍（每次嘗試都匯出，完成的排第一）。
- 回覆 0 項；未看 0 項。
- 隨同寫進「不採用的方案」：直接相信 null；不設上限或每次核准；逐行文字格式或 SQLite（以上在設計基線）；只匯出成功的那次（在匯出介面）。
- 新列為未決：同一筆付款同時被兩個接手者續接。寫回時發現，還沒出卡。
- 新名字：`HistoryLookup`（決策包第 3 項形狀裡的原名）。

## 實作（2026-11-19）

- 對應提交：`66200f0`（程式碼）、`6a637e4`（術語表檢查只列頂層名字）、`9fdf82c`（術語表）。
- 檢查：`cargo fmt --check`、`cargo clippy --locked --all-targets -- -D warnings`、`cargo test --locked`（36 個：25 個單元測試、10 個情境測試、1 個文件範例）、`RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps`，全部通過；專案的四支檢查腳本也通過。
- 確認後驗證：把確認過的形狀寫成可執行的測試之後，再故意關掉規則，確認測試會失敗：

| 關掉的規則 | 失敗的測試 |
|---|---|
| 帳冊涵蓋證明 | `history_starting_after_the_fetch_slot_escalates`、`pruned_history_escalates_instead_of_signing_again` |
| 過期之前不簽新的一筆 | `not_found_rebroadcasts_up_to_and_including_the_last_valid_height`、`killed_after_broadcast_resumes_without_paying_twice`、`killed_between_write_and_broadcast_sends_on_resume` |
| 一筆付款最多 3 次嘗試 | `attempt_limit_stops_automatic_signing`、`every_attempt_dropped_stops_at_the_limit` |
| 落地失敗交給人 | `finalized_failure_escalates_with_the_error`、`landed_failure_goes_to_a_person` |
| 較早的嘗試在鏈上就交給人 | `an_earlier_attempt_on_chain_escalates_even_if_the_latest_succeeded` |
| 完成的那次排第一 | `done_attempt_comes_first_and_the_rest_keep_signing_order`、`export_lists_every_attempt_with_the_done_one_first` |

- 實作取捨（同時滿足三個條件，先做後報）：
  1. 付款紀錄還沒有任何嘗試時，下一步是簽新的一筆：嘗試在廣播之前寫入，紀錄裡沒有就代表沒有送出過。
  2. 較早的嘗試在鏈上查得到時交給人（`EarlierAttemptOnChain`）：它在下一次嘗試之前已被證明沒落地，查得到代表那次證明有誤。
  3. 查得到但還沒 finalized：未過期就重送同一份，過期後交給人（`NotFinalizedAfterExpiry`）。
  4. 鏈上觀察少了某個簽章的查詢結果時交給人（`ObservationMissing`），和「查詢失敗」分開。
  5. payout_id 會當成檔名，所以限定為 1–128 bytes 的英數字與 `.`、`_`、`-`，而且不能以 `.` 開頭。
  6. 資料夾 fsync 只在 Unix 上執行；在其他平台沒有這一步，斷電保證只適用於 Unix。
  7. 付款紀錄加上付款內容（付款人、收款人、mint、金額）：匯出給 web3testproj1 需要這些欄位。
  8. 模擬鏈裡，被丟掉的交易在過期前重送，會再得到一次落地的機會。
- 測試本身的一個錯誤：匯出的情境測試原本沒有中斷 Session，重送讓第一次嘗試落地，只產生一列；改成先中斷、讓第一次嘗試過期，再續接。
