#import <AppKit/AppKit.h>
#import <AVFoundation/AVFoundation.h>
#import <ApplicationServices/ApplicationServices.h>
#include <stdatomic.h>
#include <stdbool.h>
#include <unistd.h>
#import <dispatch/dispatch.h>

static NSMutableDictionary<NSNumber *, id> *targets;
static NSMutableArray *system_observers;
static _Atomic(uint64_t) next_target = 1;

static void on_main(dispatch_block_t block) {
    if (NSThread.isMainThread) block();
    else dispatch_sync(dispatch_get_main_queue(), block);
}

static AXUIElementRef focused_element(pid_t pid) {
    AXUIElementRef app = AXUIElementCreateApplication(pid);
    if (!app) return NULL;
    CFTypeRef value = NULL;
    AXError err = AXUIElementCopyAttributeValue(app, kAXFocusedUIElementAttribute, &value);
    CFRelease(app);
    return err == kAXErrorSuccess ? (AXUIElementRef)value : NULL;
}

static CFTypeRef attribute(AXUIElementRef element, CFStringRef name) {
    CFTypeRef value = NULL;
    if (AXUIElementCopyAttributeValue(element, name, &value) != kAXErrorSuccess) return NULL;
    return value;
}

static BOOL secure_field(AXUIElementRef element) {
    CFTypeRef subrole = attribute(element, kAXSubroleAttribute);
    BOOL result = subrole && CFEqual(subrole, kAXSecureTextFieldSubrole);
    if (subrole) CFRelease(subrole);
    return result;
}

static BOOL copy_string(CFTypeRef value, char *out, size_t cap) {
    if (!value || CFGetTypeID(value) != CFStringGetTypeID() || cap == 0) { if (cap) out[0] = 0; return NO; }
    return CFStringGetCString((CFStringRef)value, out, cap, kCFStringEncodingUTF8);
}

int dicta_microphone_status(void) {
    switch ([AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio]) {
        case AVAuthorizationStatusAuthorized: return 3;
        case AVAuthorizationStatusDenied: return 2;
        case AVAuthorizationStatusRestricted: return 1;
        case AVAuthorizationStatusNotDetermined: return 0;
    }
    return -1;
}

void dicta_request_microphone(void) {
    on_main(^{
        if ([AVCaptureDevice authorizationStatusForMediaType:AVMediaTypeAudio] == AVAuthorizationStatusNotDetermined)
            [AVCaptureDevice requestAccessForMediaType:AVMediaTypeAudio completionHandler:^(__unused BOOL granted) {}];
    });
}

bool dicta_accessibility_trusted(bool prompt) {
    if (!prompt) return AXIsProcessTrusted();
    NSDictionary *options = @{(__bridge id)kAXTrustedCheckOptionPrompt: @YES};
    return AXIsProcessTrustedWithOptions((__bridge CFDictionaryRef)options);
}

static int capture_target_main(uint64_t *token, int32_t *pid_out, char *role_out, size_t role_len, char *title_out, size_t title_len) {
    NSRunningApplication *front = NSWorkspace.sharedWorkspace.frontmostApplication;
    if (!front) return 5;
    pid_t pid = front.processIdentifier;
    if (pid == getpid()) return 3;
    if (!AXIsProcessTrusted()) return 1;
    AXUIElementRef element = focused_element(pid);
    if (!element) return 2;
    if (secure_field(element)) { CFRelease(element); return 4; }
    CFTypeRef role = attribute(element, kAXRoleAttribute);
    if (!role) { CFRelease(element); return 2; }
    Boolean settable = false;
    AXError can_insert = AXUIElementIsAttributeSettable(element, kAXSelectedTextAttribute, &settable);
    if (can_insert != kAXErrorSuccess || !settable) { CFRelease(role); CFRelease(element); return 2; }
    if (!copy_string(role, role_out, role_len)) { CFRelease(role); CFRelease(element); return 2; }
    CFRelease(role);
    CFTypeRef window = attribute(element, kAXWindowAttribute);
    CFTypeRef title = window ? attribute((AXUIElementRef)window, kAXTitleAttribute) : NULL;
    copy_string(title, title_out, title_len);
    if (title) CFRelease(title);
    if (window) CFRelease(window);
    uint64_t id = atomic_fetch_add(&next_target, 1);
    if (!targets) targets = [NSMutableDictionary dictionary];
    @synchronized (targets) { targets[@(id)] = CFBridgingRelease(element); }
    *token = id;
    *pid_out = (int32_t)pid;
    return 0;
}

int dicta_capture_target(uint64_t *token, int32_t *pid_out, char *role_out, size_t role_len, char *title_out, size_t title_len) {
    __block int result = 5;
    on_main(^{ result = capture_target_main(token, pid_out, role_out, role_len, title_out, title_len); });
    return result;
}

void dicta_release_target(uint64_t token) {
    if (!targets) return;
    @synchronized (targets) { [targets removeObjectForKey:@(token)]; }
}

static int insert_text_main(uint64_t token, int32_t pid, const char *utf8) {
    if (!AXIsProcessTrusted()) return 1;
    if (NSWorkspace.sharedWorkspace.frontmostApplication.processIdentifier != pid) return 2;
    AXUIElementRef current = focused_element(pid);
    if (!current) return 3;
    id saved = nil;
    @synchronized (targets) { saved = targets[@(token)]; }
    BOOL same = saved && CFEqual(current, (__bridge AXUIElementRef)saved);
    CFRelease(current);
    if (!same) return 3;
    AXUIElementRef target = (__bridge AXUIElementRef)saved;
    if (secure_field(target)) return 5;
    Boolean settable = false;
    if (AXUIElementIsAttributeSettable(target, kAXSelectedTextAttribute, &settable) != kAXErrorSuccess || !settable) return 4;
    NSString *text = [NSString stringWithUTF8String:utf8 ?: ""];
    if (!text) return 6;
    AXError result = AXUIElementSetAttributeValue(target, kAXSelectedTextAttribute, (__bridge CFStringRef)text);
    return result == kAXErrorSuccess ? 0 : 4;
}

int dicta_insert_text(uint64_t token, int32_t pid, const char *utf8) {
    __block int result = 6;
    on_main(^{ result = insert_text_main(token, pid, utf8); });
    return result;
}

static int copy_recovery_main(const char *utf8) {
    NSString *text = [NSString stringWithUTF8String:utf8 ?: ""];
    if (!text) return 1;
    NSPasteboard *pasteboard = NSPasteboard.generalPasteboard;
    [pasteboard clearContents];
    return [pasteboard setString:text forType:NSPasteboardTypeString] ? 0 : 1;
}

int dicta_copy_recovery(const char *utf8) {
    __block int result = 1;
    on_main(^{ result = copy_recovery_main(utf8); });
    return result;
}

static int active_display_bounds_main(double *out) {
    NSScreen *screen = nil;
    NSPoint mouse = NSEvent.mouseLocation;
    for (NSScreen *candidate in NSScreen.screens) {
        if (NSPointInRect(mouse, candidate.frame)) { screen = candidate; break; }
    }
    if (!screen) screen = NSScreen.mainScreen;
    NSScreen *primary = NSScreen.screens.firstObject;
    if (!screen || !primary) return 1;
    NSRect frame = screen.frame;
    out[0] = frame.origin.x;
    out[1] = NSMaxY(primary.frame) - NSMaxY(frame);
    out[2] = frame.size.width;
    out[3] = frame.size.height;
    return 0;
}

int dicta_active_display_bounds(double *out) {
    __block int result = 1;
    on_main(^{ result = active_display_bounds_main(out); });
    return result;
}

static int configure_overlay_main(void *window_ptr) {
    if (!window_ptr) return 1;
    NSWindow *window = (__bridge NSWindow *)window_ptr;
    window.collectionBehavior |= NSWindowCollectionBehaviorCanJoinAllSpaces | NSWindowCollectionBehaviorFullScreenAuxiliary;
    window.ignoresMouseEvents = NO;
    return 0;
}

int dicta_configure_overlay(void *window_ptr) {
    __block int result = 1;
    on_main(^{ result = configure_overlay_main(window_ptr); });
    return result;
}

extern void dicta_system_event(int32_t kind);

void dicta_register_system_monitors(void) {
    on_main(^{
        static dispatch_once_t once;
        dispatch_once(&once, ^{
            system_observers = [NSMutableArray array];
            NSNotificationCenter *center = NSWorkspace.sharedWorkspace.notificationCenter;
            NSArray *names = @[NSWorkspaceWillSleepNotification, NSWorkspaceScreensDidSleepNotification,
                               NSWorkspaceDidWakeNotification, NSWorkspaceScreensDidWakeNotification];
            [names enumerateObjectsUsingBlock:^(NSNotificationName name, NSUInteger index, __unused BOOL *stop) {
                id observer = [center addObserverForName:name object:nil queue:nil usingBlock:^(__unused NSNotification *note) {
                    dicta_system_event((int32_t)index);
                }];
                [system_observers addObject:observer];
            }];
        });
    });
}

void dicta_play_sound(void) { on_main(^{ NSBeep(); }); }
