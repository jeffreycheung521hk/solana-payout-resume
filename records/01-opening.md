# 開場

## 原始請求

使用者在先前另一段對話裡提出這個請求，當時沒有完成：

> 能不能用這個Notes ，構思一個Web3相關的範例 ，放上我的GitHub ，Readme中你要寫清楚，如何利用了Notes 中的Skill

這一輪開頭，使用者問：

> 如果我開放GitHub 權限給你，你能幫我完成未完的任務嗎？

並補充：

> 沒有關係，但你也可以考慮善用web3testproj1

## 使用者的選擇（2026-11-19，選項式提問）

| 問題 | 使用者選擇 |
|---|---|
| 範例情境 | USDC 付款中斷續接：付款送出後、確認前 Agent 中斷；新 Session 靠交接檔續接、不重複付款；結果可交給 web3testproj1 對帳 |
| 裁決方式 | 真實跑一輪：Agent 在聊天裡出形狀卡和決策包，使用者裁決 |
| 放在哪裡 | 新開 repo：使用者建立 `solana-payout-resume`，Agent 推到分支開 PR，使用者審過再 merge |
| README 語言 | 台灣繁中主文，開頭附英文摘要 |
| 審查是否用子代理 | 同意：決策包呈現前的無 Context 通讀、批末的審查線 |
| 推送權限 | 使用者把這個 repo 加進 Claude GitHub App 的存取清單 |

## 本輪分工

- 使用者：使用者兼統籌，負責裁決與按下 merge。
- 這個 Session：一條線，名為「USDC 付款續接線」，負責設計討論、寫回、實作與交接；分支 `impl/payout-resume`。
- 子代理：只做通讀與審查，不寫設計結論。

## 和 Notes 的差異

- 沒有獨立的統籌 Session，也沒有在終端機開多個 Session；統籌由使用者兼任。
- 沒有統籌寫的開場提示；由上面的原始請求與選擇代替。
- 規劃 Repository 與原始碼 Repository 合在同一個 repo，用 `planning/`、`source/` 兩個目錄代替。
