---
"macOS": patch
---

Revert `nextResponder` call because this will bring key beep sound regression. We'll call the key equivalent in wry instead.

