# workspace：每個 Session 都適用

- 載入：每個 Session 開工前讀一次；Session 被壓縮後再讀一次。
- 不載入：無。

## repo 邊界

- 這個 repo 同時扮演 Notes 裡的工作區根目錄、規劃 Repository 和一個原始碼 Repository：
  - `planning/`：設計文件、術語表、待辦帳本、指令體系。
  - `source/`：Rust crate `solana-payout-resume`。Cargo 指令在 `source/` 裡執行，根目錄不是 Cargo workspace。
  - `records/`：流程紀錄。
- 同一次提交不同時改 `planning/` 和 `source/`。
- 原因：Notes 要求各 Repository 獨立、各自提交。這個範例要讓讀者一個 repo 就看完，所以用目錄代替 Repository，但保留「提交不跨邊界」這條，讓設計寫回和程式碼改動在歷史裡分得開。

## 從根目錄啟動

- 要求：Session 從 repo 根目錄啟動；不在 `planning/` 或 `source/` 另放給 Agent 的入口檔案。
- 原因：根路由只有從根目錄啟動時才保證被載入。

## 交流語言

- 要求：和使用者交流一律用台灣繁體中文，技術與金融名詞保留英文；不管使用者這一則用什麼語言、工具輸出是什麼語言。
- 程式碼識別符、指令與提交訊息用英文；提交訊息用 Conventional Commits。
