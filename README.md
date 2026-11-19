# solana-payout-resume

> **English summary.** A worked example of a multi-session AI-agent workflow (referred to below as *the Notes*) applied to a Web3 problem. An agent sends a USDC payout on Solana and is interrupted before it learns the result; the session that resumes must decide whether to wait, resend the same transaction, sign a new one, or hand the payout to a person, without ever paying twice. Every attempt is written to disk before it is broadcast, and the decision is a pure Rust function of that record and of chain facts. The repo holds the crate (offline simulation, 36 tests, mutation-checked), the confirmed design baseline, the agent instruction system, and the actual records of one round of human–agent collaboration: a shape card, a decision pack, write-back receipts, a review pack and a handoff. The table in「這個範例怎麼用上 Notes 的方法」maps each technique in the Notes to the file where it was applied.

## 這是什麼

一個 AI Agent 要付 120 USDC 給供應商。它簽好交易、送出之後，Session 就中斷了：可能是額度用完、Context 被壓縮，或程序被砍掉。接手的 Session 沒讀過前面的對話，卻要判斷這筆錢到底付出去了沒有；判斷錯了，不是重複付款，就是供應商收不到錢。

這個 repo 用 Notes 的方法解這個問題，而且整個過程本身就照 Notes 跑了一輪：Agent 出形狀卡和決策包，使用者在聊天裡裁決，Agent 只把裁決過的內容寫進設計基線，再實作、驗證、交給沒有 Context 的審查線審查。所有卡片、裁決原話、回執和審查包都留在 [`records/`](records/README.md)。

核心想法只有一句：**付款 Agent 也需要「換 Session 前先寫交接」**。Notes 要求寫程式的 Agent 在 Context 遺失前寫好交接；這裡要求付款 Agent 在不可逆的一步（廣播交易）之前，先把交易簽章、可原封不動重送的位元組和過期高度寫進磁碟。接手的 Session 只看這份紀錄和鏈上查詢，不靠記憶。

## 結果

`cargo run --example interrupted_payout` 的輸出（離線模擬，簽章是合成的）：

```text
scenario                                       resumed session ended with         attempts  paid
killed after broadcast, lands 30 blocks later  Done                                      1     1
killed between write and broadcast             Done                                      1     1
dropped while the session was down             Done                                      2     1
node history starts after the attempt          Escalate: HistoryDoesNotCover             1     1
landed but failed                              Escalate: LandedButFailed                 1     0
every attempt dropped                          Escalate: AttemptLimitReached             3     0
contrast: re-sign before expiry                Done                                      4     2
contrast: trust null from pruned history       Done                                      2     2
```

照規則續接的六個情境，`paid`（實際成功落地的轉帳數）都不超過 1；最後兩行是故意寫錯的對照策略，各付了 2 次。

## 快速開始

需要 Rust 1.85 以上。

```sh
cd source
cargo test --locked                          # 36 個測試
cargo run --locked --example interrupted_payout
```

專案的文件檢查在根目錄執行：

```sh
python3 planning/.agents/scripts/check_reader_docs.py
python3 planning/.agents/scripts/check_glossary.py
python3 planning/.agents/scripts/check_traces.py
python3 planning/.agents/scripts/check_instructions.py
```

## 這個範例怎麼用上 Notes 的方法

| Notes 的方法 | 在這個範例怎麼用 | 看哪裡 |
|---|---|---|
| §1、§4 角色：統籌、線、審查線、子代理 | 使用者兼統籌，負責裁決與按下 merge；一個 Session 擔任「USDC 付款續接線」；子代理只做通讀和審查，不寫設計結論 | [`records/01-opening.md`](records/01-opening.md)、[`line.md`](planning/.agents/topics/line.md) |
| §2 工作區與 Repository 佈局 | 用 `planning/` 代替規劃 Repository、`source/` 代替原始碼 Repository；同一次提交不同時改兩個目錄 | [`workspace.md`](planning/.agents/topics/workspace.md)、commit 歷史 |
| §3 指令體系：根路由只放指標、唯一觸發表、主題是唯一規範來源 | 根目錄 `AGENTS.md` 只有三條指標；`planning/AGENTS.md` 是唯一的觸發表；8 個主題各管一類規則；腳本核對七項 | [`AGENTS.md`](AGENTS.md)、[`planning/AGENTS.md`](planning/AGENTS.md)、[`planning/.agents/`](planning/.agents/README.md) |
| §6 線的須知；附錄 A 開場提示 | 線的須知寫成主題；開審查線時照附錄 A 寫開場提示，第一句「你是「審查線」」，並把線的須知整段放進去 | [`line.md`](planning/.agents/topics/line.md)、[`records/review/opening-prompt.md`](records/review/opening-prompt.md) |
| §7.1 協定能決定的直接定 | commitment 用 finalized、過期後才重簽、查全歷史、不用 durable nonce，都依 Solana 官方文件直接定，附來源，只向使用者報結論 | [`design.md`](planning/docs/payout-resume/design.md)「依 Solana 官方文件定下的規則」 |
| §7.1 架構問題用形狀卡 | 一張卡：送出前寫紀錄、事後用 reference 搜尋、鏈上保證只執行一次，三種形狀附程式碼，只問一個問題 | [`records/02-shape-card.md`](records/02-shape-card.md) |
| §7.2 條目裁決用決策包 | 5 項、4,407 字；開頭列前提與狀態，每項「情境 → 判據 → 替代方案的前提與代價 → 推薦」，包末附裁決表，不提供一鍵全採納 | [`records/03-decision-pack.md`](records/03-decision-pack.md) |
| §7.3 呈現前檢查 | 沒有 Context 的子代理以讀者身分通讀，回報 6 類問題，其中一個是正確性問題：漏了「rename 之後 fsync 資料夾」；程式形狀先編譯；對照 RPC 文件與 web3testproj1 原始碼 | 決策包文末「呈現前的檢查」 |
| §7.4 沉默不算採納 | 使用者第一次回覆只處理了建 repo 和子代理，沒有回覆形狀卡；沒有把推薦當成採納，用選項再問一次 | [`records/02-shape-card.md`](records/02-shape-card.md)「裁決」 |
| §7.3 確認後驗證 | 確認的形狀寫成可執行的測試；再故意關掉 6 條規則，每條都有測試失敗 | [`records/receipts.md`](records/receipts.md)「實作」 |
| §7.5 基線只寫已確認、回執、待辦帳本 | 設計文件只寫裁決過或依官方文件定下的內容；每次寫回附回執（落實、回覆、未看幾項、新名字）；帳本記停點與未決 | [`design.md`](planning/docs/payout-resume/design.md)、[`receipts.md`](records/receipts.md)、[`planning/ledger/`](planning/ledger/payout-resume.md) |
| §8.1 實作取捨的三個條件 | 8 項實作取捨符合「不改公開類型形狀、不違反已確認文件、能由規則推出」，先做後報 | [`records/receipts.md`](records/receipts.md)「實作取捨」 |
| §8.2 按批合併 | 這一批全部留在 `impl/payout-resume` 分支，批末才開 PR，由使用者 merge | Pull request |
| §8.4 審查包 | 審查線在被審提交的 detached 檢出上唯讀審查，只寫自己的輸出目錄；首頁最多 10 項，附件含偏離紀錄與設計條款的測試覆蓋 | [`records/review/`](records/review/opening-prompt.md) |
| §9 術語表與命名規則 | 七欄術語表；衝突清單（重試、確認、reason）；形狀卡的示意名在結構確認後按規則定名（`Next` → `NextAction` 等）；腳本列出不在表中的公開名字 | [`planning/docs/glossary/`](planning/docs/glossary/README.md) |
| §10.1 按決策主體歸檔、中繼資料 | 本專案的設計放 `payout-resume/`，和 web3testproj1 共同的介面放 `shared/`；每份讀者文件有兩行歸屬中繼資料 | [`planning/docs/`](planning/docs/README.md) |
| §10.2 只留最新基線 | 被否決的方案只在「不採用的方案」留下要解決的問題、缺陷、現行設計怎麼處理 | [`design.md`](planning/docs/payout-resume/design.md)「不採用的方案」 |
| §10.3 工程痕跡 | 設計文件、術語表與程式碼不寫日期、決定編號、工具名稱；腳本檢查 | [`check_traces.py`](planning/.agents/scripts/check_traces.py) |
| §10.4–10.5 完成前的檢查 | cargo 全套檢查加四支腳本；每支腳本都先植入錯誤，確認它會失敗 | [`verification.md`](planning/.agents/topics/verification.md) |
| 附錄 B 交接清單 | 批末照清單寫交接：已裁決未寫回的項、等回覆的卡片、停點、分支狀態 | [`records/handoff.md`](records/handoff.md) |

## 一輪協作的經過

1. 使用者提出請求：用 Notes 構思一個 Web3 範例，放上 GitHub，README 寫清楚怎麼用上 Notes。
2. Agent 先查證 Solana 的現況，發現兩件會改變設計的事：Alpenglow 已在 2026 年 10 月上主網，confirmed 和 finalized 變成同一個狀態；durable nonce 已宣布將移除。
3. 形狀卡：送出前寫紀錄（推薦）、事後用 reference 搜尋、鏈上保證只執行一次。使用者第一次回覆沒有提到形狀卡，Agent 再問一次，使用者選了送出前寫紀錄。
4. 寫回形狀卡結論、建好指令體系與術語表，然後出決策包；決策包給使用者之前，先讓沒有 Context 的子代理通讀，修掉 6 類問題。
5. 使用者回覆「所有問題都按你的推薦方案執行」；Agent 寫回 5 項裁決，實作、測試，再用變異測試確認測試真的擋得住錯誤。
6. 審查線審查，Agent 寫交接，開 PR 給使用者 merge。

## 續接判斷的規則

付款紀錄裡最新的一次嘗試，查詢結果與下一步：

| 最新嘗試的狀況 | 下一步 |
|---|---|
| 還沒有任何嘗試 | 簽新的一筆（第一次） |
| 在 finalized 落地且成功 | 完成 |
| 在 finalized 落地但失敗 | 交給人 |
| 查得到但還沒 finalized，未過期 | 重送同一份 |
| 查得到但還沒 finalized，已過期 | 交給人 |
| 查不到，未過期 | 重送同一份 |
| 查不到，已過期，但節點的帳冊從取得 blockhash 之後才開始 | 交給人 |
| 查不到，已過期，帳冊涵蓋，已有 3 次嘗試 | 交給人 |
| 查不到，已過期，帳冊涵蓋，不到 3 次嘗試 | 簽新的一筆 |

另外，任何查詢失敗或缺少，以及較早的嘗試反而在鏈上查得到，都交給人。完整規則與依據見 [`design.md`](planning/docs/payout-resume/design.md)，程式在 [`source/src/decision.rs`](source/src/decision.rs)。

## 和 web3testproj1 的關係

[web3testproj1](https://github.com/jeffreycheung521hk/web3testproj1) 是一個唯讀的 Solana 付款對帳工具，只核對已知的交易簽章。這個 repo 負責付款送出與續接，把每次嘗試匯出成 web3testproj1 的輸入 CSV，讓它獨立核對：

- 完成的那次排第一列，吻合就是 matched。
- 其他嘗試因為 payment_id 重複，會被標成待查；看 `original_reason` 就能分辨是「沒落地」（正常），還是「也落地而且轉帳成功」（重複付款）。

所以就算續接邏輯有錯，對帳工具也能獨立抓到重複付款。規則見 [`shared/reconcile-export.md`](planning/docs/shared/reconcile-export.md)。範例是離線模擬，匯出的簽章是合成的，只能檢查格式，不能上鏈核對。

## repo 地圖

| 路徑 | 內容 |
|---|---|
| [`AGENTS.md`](AGENTS.md) | 給 Agent 的根路由，只有三條指標 |
| [`planning/AGENTS.md`](planning/AGENTS.md) | 唯一的觸發表 |
| [`planning/.agents/`](planning/.agents/README.md) | 8 個主題與 4 支檢查腳本 |
| [`planning/docs/`](planning/docs/README.md) | 讀者文件：設計基線、跨專案介面、術語表 |
| [`planning/ledger/`](planning/ledger/payout-resume.md) | 待辦帳本 |
| [`source/`](source/) | Rust crate：`record`、`store`、`observation`、`decision`、`export`、`simulation` |
| [`records/`](records/README.md) | 一輪協作的流程紀錄 |

## 和 Notes 不一樣的地方

- **單一 repo**：Notes 要求規劃 Repository 和原始碼 Repository 各自獨立；這裡用目錄代替，讓讀者一個 repo 就看完。
- **沒有獨立的統籌 Session**：統籌由使用者兼任；沒有在終端機開多個 Session，也沒有用到 Notes 的 Session 監控、壓縮與開 Session 的做法（§5.3、§5.5、§5.7）。
- **流程紀錄進了版本控制**：Notes 把卡片、交接、審查包放在不進 Git 的草稿目錄；這裡為了展示放在 `records/`。
- **沒有採用 Notes 裡的數字門檻**：額度 93%、Context 420k／450k token 是原作者環境的經驗值，這裡沒有量測過，所以不寫。
- **工程痕跡的兩個例外**：這份 README 的主題就是工作流程，所以會寫到 Agent 與流程；提交訊息尾端保留 AI 共同作者標記，這是工具的署名規定。
- **設計覆盤還沒做**：§8.5 的縱向切片與設計覆盤，要等更多批次之後再做。

## 限制與下一步

- 只做離線模擬，簽章與位元組是合成的；用 devnet 真的送一筆，留到下一批。
- 三項未決，會放進下一個決策包：付款紀錄遺失時怎麼辦；落地失敗交給人之後，用同一個 payout_id 還是新開；同一筆付款同時被兩個接手者續接。
- 資料夾 fsync 只在 Unix 執行，斷電保證只適用於 Unix。

## Notes 的來源

Notes 指使用者提供的《多 Session AI Agent 協作工作流程》筆記，作者與出處待補。`planning/.agents/` 裡的主題是依這個 repo 改寫的版本，不是原文。

## 授權

尚未選定授權。
