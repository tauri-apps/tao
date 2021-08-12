---
tao: patch
---

Fix missing `Sync` trait on EventLoopProxy. This commit also introduces `crossbeam-channel` crate which could also improve the performance.
