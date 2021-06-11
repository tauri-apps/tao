---
"tao": minor
---

**Breaking change**: New keyboard API, including `Accelerator`.

`WindowEvent::ModifiersChanged` is emitted when a new keyboard modifier is pressed. This is your responsibility to keep a local state. When the modifier is released, `ModifiersState::empty()` is emitted.

`WindowEvent::KeyboardInput` as been refactored and is exposing the event `KeyEvent`.

New modules available: `keyboard` and `accelerator`. Please refer to the docs and examples for more details.
