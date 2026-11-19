# 交接：USDC 付款續接線

照附錄 B 的交接清單寫，給下一個接手這條線的 Session。

## 1. 已裁決、還沒寫回的項

無。形狀卡與第一個決策包的裁決都已寫回 `planning/docs/payout-resume/design.md` 與 `planning/docs/shared/reconcile-export.md`。

## 2. 等使用者回覆的事

- 審查包首頁的事項：見文末「審查包的處理」。
- Notes 的作者與出處：README 目前寫「作者與出處待補」。
- 授權：README 目前寫「尚未選定授權」；web3testproj1 用的是 MIT，可以參考。
- GitHub App 權限：Claude 的 GitHub App 還沒拿到這個 repo 的權限，推送被拒（HTTP 403）。使用者把 repo 加進 App 的存取清單後，才能推送 `impl/payout-resume`。

## 3. 停點和下一步

- 停點：第一批（指令體系、設計基線、crate、審查）完成，等使用者 merge。
- 下一步，依序：
  1. 推送分支、開 PR；使用者 merge 後，這一批結束。
  2. 出下一個決策包，處理三項未決：付款紀錄遺失時怎麼辦；落地失敗交給人之後，用同一個 payout_id 還是新開；同一筆付款同時被兩個接手者續接。
  3. 問使用者要不要做 devnet 實跑。
  4. 若干批之後做設計覆盤（code-review 主題「已確認設計本身的複核」）。

## 4. 分支狀態

- 分支：`impl/payout-resume`，從 `main` 的初始提交開出。
- 未報的提交：這一批全部的提交都還沒進 `main`；PR 尚未開。
- 未提交的改動：無（寫完這份交接後提交）。
- 唯讀審查用的檢出 `/home/claude/review-checkout` 是這個工作環境裡的臨時 worktree，收線時刪除。

## 5. 跨線要轉告的事

- 只有這一條線，沒有要轉告給其他線的事。
- 和 web3testproj1 的介面：如果 web3testproj1 之後改了輸入 CSV 格式，或開始支援 reference，`planning/docs/shared/reconcile-export.md` 與 `source/src/export.rs` 要跟著改。

## 6. 統籌的交接

本範例由使用者兼任統籌：

- 使用者已定、還沒做的事：把 repo 加進 Claude GitHub App 的存取清單；merge PR。
- 各線狀態：只有「USDC 付款續接線」，交接就是這份文件。

## 審查包的處理

審查線進行中；完成後在這裡記下首頁各項的處理方式。
