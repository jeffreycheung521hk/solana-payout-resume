# planning/AGENTS.md

## 觸發表

同時命中幾行就全部載入；沒命中的不載入。

| 任務觸發條件 | 必須載入的主題 |
|---|---|
| 被開成一條線、派子代理、寫交接，或被壓縮後續接 | `.agents/topics/line.md` |
| 討論設計、出形狀卡或決策包、寫回設計文件、更新待辦帳本 | `.agents/topics/design-collab.md` |
| 新增、修改、移動或刪除 `planning/docs/` 下的讀者文件 | `.agents/topics/docs.md` |
| 討論、寫文件或給程式碼取公開名稱時用到承載概念的名字；修改術語表 | `.agents/topics/terminology.md` |
| 修改 `source/` 的程式碼、報合併、產出審查包 | `.agents/topics/code-review.md` |
| 宣告完成或建立提交之前 | `.agents/topics/verification.md` |
| 修改任何 `AGENTS.md` 或 `.agents/` | `.agents/topics/instructions.md` |

## 始終適用

- 流程紀錄（卡片原文、使用者的裁決、回執、交接、審查包）寫在 `records/`，不寫進 `planning/docs/` 或 `source/`。
