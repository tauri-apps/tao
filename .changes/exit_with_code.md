---
"tao": minor
---

**Breaking:** Rename the `Exit` variant of `ControlFlow` to `ExitWithCode`, which holds a value to control the exit code after running. Add an `Exit` constant which aliases to `ExitWithCode(0)` instead to avoid major breakage. This shouldn't affect most existing programs.
