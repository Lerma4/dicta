import ApplicationServices
import AppKit
import Carbon

struct AccessibilityTarget: @unchecked Sendable {
    private let pid: pid_t
    private let element: AXUIElement
    private let rexWindow: AXUIElement?

    static func isFrontmostTerminal() -> Bool {
        guard let app = NSWorkspace.shared.frontmostApplication,
              app.processIdentifier != ProcessInfo.processInfo.processIdentifier,
              let bundleID = app.bundleIdentifier else { return false }
        return TerminalRecovery.supports(bundleID: bundleID)
    }

    static func allowsTerminalRecovery() -> Bool {
        AXIsProcessTrusted() && !IsSecureEventInputEnabled() && isFrontmostTerminal()
    }

    static func captureFocusedExternalTarget() -> AccessibilityTarget? {
        guard AXIsProcessTrusted() else { return nil }
        guard let front = NSWorkspace.shared.frontmostApplication else { return nil }
        let application = AXUIElementCreateApplication(front.processIdentifier)
        var focused: CFTypeRef?
        guard AXUIElementCopyAttributeValue(application, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let element = focused as! AXUIElement? else { return nil }
        var pid: pid_t = 0
        guard AXUIElementGetPid(element, &pid) == .success, pid != ProcessInfo.processInfo.processIdentifier,
              NSWorkspace.shared.frontmostApplication?.processIdentifier == pid else { return nil }
        if front.bundleIdentifier == "com.superlogical.rex" {
            guard !IsSecureEventInputEnabled(), isNonSecureTextArea(element) else { return nil }
            var window: CFTypeRef?
            guard AXUIElementCopyAttributeValue(application, kAXFocusedWindowAttribute as CFString, &window) == .success,
                  let window, CFGetTypeID(window) == AXUIElementGetTypeID() else { return nil }
            return AccessibilityTarget(pid: pid, element: element, rexWindow: (window as! AXUIElement))
        }
        guard isSafeWritableTextField(element) else { return nil }
        return AccessibilityTarget(pid: pid, element: element, rexWindow: nil)
    }

    @MainActor
    func insert(_ text: String) -> Bool {
        guard isStillFocused() else { return false }
        if rexWindow != nil { return RexPaste.send(text, targetStillValid: isStillFocused) }
        guard Self.isSafeWritableTextField(element) else { return false }
        return AXUIElementSetAttributeValue(element, kAXSelectedTextAttribute as CFString, text as CFString) == .success
    }

    private func isStillFocused() -> Bool {
        guard AXIsProcessTrusted(), !IsSecureEventInputEnabled(), NSWorkspace.shared.frontmostApplication?.processIdentifier == pid else { return false }
        let application = AXUIElementCreateApplication(pid)
        var focused: CFTypeRef?
        guard AXUIElementCopyAttributeValue(application, kAXFocusedUIElementAttribute as CFString, &focused) == .success,
              let current = focused as! AXUIElement?, CFEqual(current, element) else { return false }
        var currentPID: pid_t = 0
        guard AXUIElementGetPid(current, &currentPID) == .success, currentPID == pid, currentPID != ProcessInfo.processInfo.processIdentifier else { return false }
        if let rexWindow {
            var window: CFTypeRef?
            guard Self.isNonSecureTextArea(current),
                  AXUIElementCopyAttributeValue(application, kAXFocusedWindowAttribute as CFString, &window) == .success,
                  let window, CFEqual(window, rexWindow) else { return false }
        }
        return true
    }

    private static func isNonSecureTextArea(_ element: AXUIElement) -> Bool {
        var role: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, kAXRoleAttribute as CFString, &role) == .success,
              role as? String == kAXTextAreaRole as String else { return false }
        var subrole: CFTypeRef?
        return AXUIElementCopyAttributeValue(element, kAXSubroleAttribute as CFString, &subrole) != .success ||
            subrole as? String != kAXSecureTextFieldSubrole as String
    }

    private static func isSafeWritableTextField(_ element: AXUIElement) -> Bool {
        var roleValue: CFTypeRef?
        guard AXUIElementCopyAttributeValue(element, kAXRoleAttribute as CFString, &roleValue) == .success,
              let role = roleValue as? String,
              [kAXTextFieldRole as String, kAXTextAreaRole as String, kAXComboBoxRole as String].contains(role) else { return false }
        var subrole: CFTypeRef?
        if AXUIElementCopyAttributeValue(element, kAXSubroleAttribute as CFString, &subrole) == .success,
           (subrole as? String) == (kAXSecureTextFieldSubrole as String) { return false }
        var settable = DarwinBoolean(false)
        guard AXUIElementIsAttributeSettable(element, kAXSelectedTextAttribute as CFString, &settable) == .success, settable.boolValue else { return false }
        return true
    }
}
