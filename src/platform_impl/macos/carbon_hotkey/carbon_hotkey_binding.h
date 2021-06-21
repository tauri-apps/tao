typedef void (*HotkeyCallback)(int, void *);

void *install_event_handler(HotkeyCallback callback, void *data);
int uninstall_event_handler(void *event_handler_ref);
void *register_hotkey(int id, int modifier, int key);
int unregister_hotkey(void *hotkey_ref);
