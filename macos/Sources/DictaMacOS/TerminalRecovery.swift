enum TerminalRecovery {

    static func supports(bundleID: String) -> Bool {
        ["com.superlogical.rex", "com.mitchellh.ghostty", "com.apple.Terminal", "com.googlecode.iterm2"].contains(bundleID)
    }
}
