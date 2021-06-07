  
#include "carbon_hotkey_binding.h"

#include <Carbon/Carbon.h>

HotkeyCallback saved_callback = NULL;
void *saved_closure = NULL;

int hotkey_handler(
    EventHandlerCallRef next_handler, EventRef event, void *user_data)
{
    (void)(next_handler);
    (void)(user_data);
    EventHotKeyID event_hotkey;

    int result = GetEventParameter(event, kEventParamDirectObject, typeEventHotKeyID, NULL, sizeof(event_hotkey), NULL, &event_hotkey);
    if (result == noErr && saved_callback && saved_closure)
    {
        saved_callback(event_hotkey.id, saved_closure);
    }
    return noErr;
}

void *install_event_handler(HotkeyCallback callback, void *data)
{
    if (!callback || !data)
        return NULL;
    saved_callback = callback;
    saved_closure = data;
    EventTypeSpec event_type;
    event_type.eventClass = kEventClassKeyboard;
    event_type.eventKind = kEventHotKeyPressed;
    EventHandlerRef handler_ref;
    int result = InstallEventHandler(GetApplicationEventTarget(), &hotkey_handler, 1, &event_type, data, &handler_ref);

    if (result == noErr)
    {
        return handler_ref;
    }

    return NULL;
}

int uninstall_event_handler(void *handler_ref)
{
    return RemoveEventHandler(handler_ref);
}

void *register_hotkey(int id, int modifier, int key)
{
    EventHotKeyRef hotkey_ref;
    EventHotKeyID hotkey_id;
    hotkey_id.signature = 'htrs';
    hotkey_id.id = id;
    int result = RegisterEventHotKey(key, modifier, hotkey_id,
                                     GetApplicationEventTarget(), 0, &hotkey_ref);
    if (result == noErr)
        return hotkey_ref;

    return NULL;
}

int unregister_hotkey(void *hotkey_ref)
{
    return UnregisterEventHotKey(hotkey_ref);
}
