# verification：完成前的檢查

- 載入：宣告完成或建立提交之前。
- 不載入：無。

## 規則

- 要求：宣告完成前，跑與這次改動相符的檢查，讀退出狀態和輸出，用當次的結果；不拿上一次的結果或目測代替。
- 把檢查串在提交前時用 `&&`，不接管線：`腳本 | tail` 的退出狀態是 `tail` 的，失敗會被吞掉。
- 腳本只做字面比對。名字是否承載概念、條目狀態是否和所屬文件一致、文件是否按討論順序組織，都靠人工核對；腳本通過不代替人工核對。
- 每完成一輪修改，跑完檢查、看過差異後建立提交，再報完成。提交只含本輪相關內容，不等於推送，也不改變結論狀態。

## 檢查表

在 repo 根目錄執行：

| 檢查 | 指令 | 何時跑 |
|---|---|---|
| 讀者文件 | `python3 planning/.agents/scripts/check_reader_docs.py` | 改了 `planning/docs/` |
| 術語表 | `python3 planning/.agents/scripts/check_glossary.py` | 用到或改動承載概念的名字，包括 `source/` 的公開名字 |
| 工程痕跡 | `python3 planning/.agents/scripts/check_traces.py` | 提交設計文件或程式碼之前 |
| 指令體系 | `python3 planning/.agents/scripts/check_instructions.py` | 改了任何 `AGENTS.md` 或 `.agents/` |
| Rust | 在 `source/` 執行 `cargo fmt --check && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked && RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps` | 改了 `source/` |

`check_glossary.py` 只在術語表格式有問題時失敗；`source/` 裡不在表中的公開名字只列清單，其中承載概念的按 `terminology.md` 定名入表。
