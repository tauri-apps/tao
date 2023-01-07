// Copyright 2014-2021 The winit contributors
// Copyright 2021-2023 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

typedef void (*HotkeyCallback)(int, void*);

void* install_event_handler(HotkeyCallback callback, void* data);
int uninstall_event_handler(void* event_handler_ref);
void* register_hotkey(int id, int modifier, int key);
int unregister_hotkey(void* hotkey_ref);
