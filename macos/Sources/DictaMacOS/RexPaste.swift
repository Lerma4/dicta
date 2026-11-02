import AppKit
import Carbon

@MainActor
enum RexPaste {
    static func send(_ text: String, targetStillValid: () -> Bool) -> Bool {
        guard !text.isEmpty,
              text.unicodeScalars.allSatisfy({ !CharacterSet.controlCharacters.contains($0) && !CharacterSet.newlines.contains($0) }),
              !IsSecureEventInputEnabled(), targetStillValid(),
              let down = CGEvent(keyboardEventSource: nil, virtualKey: 9, keyDown: true),
              let up = CGEvent(keyboardEventSource: nil, virtualKey: 9, keyDown: false) else { return false }
        let pasteboard = NSPasteboard.general
        var saved: [NSPasteboardItem] = []
        for item in pasteboard.pasteboardItems ?? [] {
            let copy = NSPasteboardItem()
            for type in item.types {
                guard let data = item.data(forType: type), copy.setData(data, forType: type) else { return false }
            }
            saved.append(copy)
        }
        pasteboard.clearContents()
        guard pasteboard.setString(text, forType: .string) else {
            if !saved.isEmpty { pasteboard.writeObjects(saved) }
            return false
        }
        let changeCount = pasteboard.changeCount
        func restoreIfUnchanged() {
            guard pasteboard.changeCount == changeCount else { return }
            pasteboard.clearContents()
            if !saved.isEmpty { pasteboard.writeObjects(saved) }
        }
        guard !IsSecureEventInputEnabled(), targetStillValid() else { restoreIfUnchanged(); return false }
        down.flags = .maskCommand
        up.flags = .maskCommand
        down.post(tap: .cghidEventTap)
        up.post(tap: .cghidEventTap)
        // shortcut: asynchronous paste has no acknowledgement; verify target app behavior before changing the delay.
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.8) { restoreIfUnchanged() }
        return true
    }
}
