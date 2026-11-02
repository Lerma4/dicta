import Carbon
import Foundation

@MainActor
final class GlobalShortcut {
    private var hotKey: EventHotKeyRef?
    private var handler: EventHandlerRef?
    private var retainedContext: UnsafeMutableRawPointer?
    private var action: (@MainActor () -> Void)?

    func register(action: @escaping @MainActor () -> Void) -> OSStatus {
        unregister()

        self.action = action
        let context = Unmanaged.passRetained(self).toOpaque()
        retainedContext = context
        var specification = EventTypeSpec(eventClass: OSType(kEventClassKeyboard), eventKind: UInt32(kEventHotKeyPressed))
        let installStatus = InstallEventHandler(GetEventDispatcherTarget(), { _, event, userData in
            guard let userData, let event else { return OSStatus(eventNotHandledErr) }
            var identifier = EventHotKeyID()
            guard GetEventParameter(event, EventParamName(kEventParamDirectObject), EventParamType(typeEventHotKeyID), nil, MemoryLayout<EventHotKeyID>.size, nil, &identifier) == noErr,
                  identifier.signature == OSType("DICT".utf16.reduce(0) { ($0 << 8) + FourCharCode($1) }), identifier.id == 1 else { return OSStatus(eventNotHandledErr) }
            let shortcut = Unmanaged<GlobalShortcut>.fromOpaque(userData).takeUnretainedValue()
            Task { @MainActor in shortcut.action?() }
            return noErr
        }, 1, &specification, context, &handler)
        guard installStatus == noErr else {
            unregister()
            return installStatus
        }
        let key = EventHotKeyID(signature: OSType(fourCharCodeFrom("DICT")), id: 1)
        let status = RegisterEventHotKey(UInt32(kVK_Space), UInt32(cmdKey | shiftKey), key, GetEventDispatcherTarget(), 0, &hotKey)
        if status != noErr { unregister() }
        return status
    }

    func unregister() {
        if let hotKey { UnregisterEventHotKey(hotKey); self.hotKey = nil }
        if let handler { RemoveEventHandler(handler); self.handler = nil }
        if let retainedContext {
            Unmanaged<GlobalShortcut>.fromOpaque(retainedContext).release()
            self.retainedContext = nil
        }
        action = nil
    }


    private func fourCharCodeFrom(_ value: String) -> FourCharCode {
        value.utf16.reduce(0) { ($0 << 8) + FourCharCode($1) }
    }
}
