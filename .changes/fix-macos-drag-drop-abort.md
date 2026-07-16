---
"tao": patch
---

On macOS, fix an abort (`SIGABRT`) when a drag entered or was dropped on a window while carrying
no `NSFilenamesPboardType` entry, which is the case for text, URLs and promised files (for example
a track dragged out of Music.app). The pasteboard was unwrapped unconditionally inside the
`draggingEntered:` and `performDragOperation:` callbacks, and since those cannot unwind, the
resulting panic aborted the process instead of propagating. Such drags now report no paths.
