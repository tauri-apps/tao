---
"tao": patch
---

Avoid panicking (and aborting the application) on Windows when the event loop is driven re-entrantly or after it has been destroyed. Re-entrant dispatches now skip the event and late state transitions from the `Destroyed` state are ignored instead of hitting `panic!("cannot move state from Destroyed")` or the "event handler is re-entrant" expectation.
