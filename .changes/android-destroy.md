---
"tao": patch
---

Trigger `WindowEvent::Destroyed` when the Android activity is destroyed. In this case, the app should either exit by setting the control flow to `ControlFlow::ExitWithCode` or NOT call the create() external function when the activity is recreated and `onCreate` is called, handling how to recreate the app window via a separate hook that can leverage the existing tao event loop.
