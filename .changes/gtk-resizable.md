---
"tao": patch
---

Fix resize doesn't work when calling with resizable. Also add platform specific note to `set_resizable`.
On Linux, most size methods like maximized are async and do not work well with calling 
sequentailly. For setting inner or outer size, you don't need to set resizable to true before
it. It can resize no matter what. But if you insist to do so, it has a `100, 100` minimum
limitation somehow. For maximizing, it requires resizable is true. If you really want to set
resizable to false after it. You might need a mechanism to check the window is really
maximized.
