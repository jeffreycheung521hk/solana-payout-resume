# 名稱分表

> 文件歸屬：solana-payout-resume
> 涉及主體：solana-payout-resume

## 本專案自造

| 中文術語 | 英文名／識別符 | 概念 | 來源 | 狀態 | 規範位置 | 別名與舊叫法 |
|---|---|---|---|---|---|---|
|  | `solana-payout-resume` | 本範例的 repo 與 Rust crate 名稱 | 自造 | 已確認（repo 由使用者建立，2026-11-19） | 根目錄 `README.md` | 無 |

## 外部既有

| 中文術語 | 英文名／識別符 | 概念 | 來源 | 狀態 | 規範位置 | 別名與舊叫法 |
|---|---|---|---|---|---|---|
|  | Solana | 區塊鏈網路 | 外部：<https://solana.com> | 已確認（外部既有名稱） | 無 | 無 |
|  | SPL Token | Solana 上的代幣程式與代幣標準 | 外部：Solana | 已確認（外部既有名稱） | 無 | token |
|  | USDC | Circle 發行的美元穩定幣；在 Solana 上以 SPL Token 形式存在 | 外部：Circle | 已確認（外部既有名稱） | 無 | 無 |
|  | Alpenglow | Solana 的共識協定；上線後 confirmed 與 finalized 描述同一個狀態 | 外部：<https://solana.com/upgrades/alpenglow> | 已確認（外部既有名稱） | `payout-resume/design.md`「依 Solana 官方文件定下的規則」 | 無 |
|  | Solana Pay | Solana 的付款請求規格，含 reference key | 外部：<https://docs.solanapay.com/spec> | 已確認（外部既有名稱） | `payout-resume/design.md`「不採用的方案」 | 無 |
|  | web3testproj1 | 唯讀的 Solana SPL Token 付款對帳 CLI，只核對已知簽章 | 外部：<https://github.com/jeffreycheung521hk/web3testproj1> | 已確認（外部既有名稱） | 該 repo 的 `README.md` | payment-reconcile（CLI 名稱） |
