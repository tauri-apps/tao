---
"tao": patch
---

Windows: make the event loop runner handle's reference count atomic (`Rc` -> `Arc`). Downstream stores it behind a type declared `Send + Sync` and clones that type from other threads; a non-atomic count loses updates under that race and eventually wraps past zero, aborting the process with `STATUS_ILLEGAL_INSTRUCTION` and no unwind. The runner itself remains main-thread-only.
