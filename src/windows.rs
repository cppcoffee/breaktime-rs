use objc2::ClassType;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSColor, NSFont, NSPanel, NSScreen, NSScreenSaverWindowLevel,
    NSStatusWindowLevel, NSTextAlignment, NSTextField, NSVisualEffectBlendingMode,
    NSVisualEffectMaterial, NSVisualEffectState, NSVisualEffectView, NSWindow,
    NSWindowCollectionBehavior, NSWindowStyleMask, NSWindowTitleVisibility,
};
use objc2_foundation::{MainThreadMarker, NSArray, NSPoint, NSRect, NSSize, NSString};

use crate::actions::AppActionTarget;
use crate::timer::{countdown_text, minutes_text};

const COUNTDOWN_BUBBLE_SIZE: NSSize = NSSize::new(160.0, 86.0);
const DIALOG_SIZE: NSSize = NSSize::new(420.0, 220.0);
const SCREEN_MARGIN: f64 = 18.0;
const TRAY_BUBBLE_GAP: f64 = 8.0;

pub struct WindowController {
    bubble: CountdownBubble,
    alert: AlertWindows,
}

impl WindowController {
    pub fn new(action_target: Retained<AppActionTarget>) -> Self {
        let mtm = MainThreadMarker::new().expect("windows must be created on the main thread");
        Self {
            bubble: CountdownBubble::new(mtm),
            alert: AlertWindows::new(mtm, action_target),
        }
    }

    pub fn show_bubble(&mut self, seconds: u32, tray_anchor: Option<NSRect>) {
        self.alert.hide();
        self.bubble.show(seconds, tray_anchor);
    }

    pub fn hide_bubble(&mut self) {
        self.bubble.hide();
    }

    pub fn show_alert(&mut self, break_minutes: u32) {
        self.bubble.hide();
        self.alert.show(break_minutes);
    }

    pub fn hide_alert(&mut self) {
        self.alert.hide();
    }

    pub fn hide_all(&mut self) {
        self.bubble.hide();
        self.alert.hide();
    }
}

struct CountdownBubble {
    panel: Retained<NSPanel>,
    countdown_label: Retained<NSTextField>,
    visible: bool,
}

impl CountdownBubble {
    fn new(mtm: MainThreadMarker) -> Self {
        let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
            NSPanel::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), COUNTDOWN_BUBBLE_SIZE),
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        );
        unsafe { panel.setReleasedWhenClosed(false) };
        panel.setBackgroundColor(Some(&NSColor::clearColor()));
        panel.setOpaque(false);
        panel.setHasShadow(true);
        panel.setLevel(NSStatusWindowLevel + 1);
        panel.setHidesOnDeactivate(false);
        panel.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::Transient,
        );
        panel.setIgnoresMouseEvents(true);

        let content = NSVisualEffectView::initWithFrame(
            NSVisualEffectView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), COUNTDOWN_BUBBLE_SIZE),
        );
        content.setMaterial(NSVisualEffectMaterial::HUDWindow);
        content.setBlendingMode(NSVisualEffectBlendingMode::BehindWindow);
        content.setState(NSVisualEffectState::Active);
        let title_label = make_label(
            "Break starts in",
            NSRect::new(NSPoint::new(16.0, 50.0), NSSize::new(128.0, 18.0)),
            13.0,
            false,
            Some(&NSColor::colorWithWhite_alpha(1.0, 0.68)),
            mtm,
        );
        let countdown_label = make_label(
            &countdown_text(10),
            NSRect::new(NSPoint::new(16.0, 14.0), NSSize::new(128.0, 34.0)),
            28.0,
            true,
            Some(&NSColor::whiteColor()),
            mtm,
        );

        content.addSubview(&title_label);
        content.addSubview(&countdown_label);
        panel.setContentView(Some(&content));
        panel.orderOut(None);

        Self {
            panel,
            countdown_label,
            visible: false,
        }
    }

    fn show(&mut self, seconds: u32, tray_anchor: Option<NSRect>) {
        self.countdown_label
            .setStringValue(&NSString::from_str(&countdown_text(seconds)));
        position_countdown_bubble(&self.panel, tray_anchor);
        self.panel.orderFrontRegardless();
        self.visible = true;
    }

    fn hide(&mut self) {
        if self.visible {
            self.panel.orderOut(None);
            self.visible = false;
        }
    }
}

struct AlertWindows {
    _action_target: Retained<AppActionTarget>,
    mask_windows: Vec<Retained<NSWindow>>,
    dialog_window: Retained<NSWindow>,
    subtitle_label: Retained<NSTextField>,
    visible: bool,
}

impl AlertWindows {
    fn new(mtm: MainThreadMarker, action_target: Retained<AppActionTarget>) -> Self {
        let dialog_window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                NSRect::new(NSPoint::new(0.0, 0.0), DIALOG_SIZE),
                NSWindowStyleMask::Titled | NSWindowStyleMask::FullSizeContentView,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe { dialog_window.setReleasedWhenClosed(false) };
        dialog_window.setTitle(&NSString::from_str("Break Time"));
        dialog_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
        dialog_window.setTitlebarAppearsTransparent(true);
        dialog_window.setBackgroundColor(Some(&NSColor::clearColor()));
        dialog_window.setLevel(NSScreenSaverWindowLevel + 1);
        dialog_window.setHasShadow(true);
        dialog_window.setHidesOnDeactivate(false);
        dialog_window.setMovableByWindowBackground(false);
        dialog_window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Stationary
                | NSWindowCollectionBehavior::Transient,
        );

        let content = NSVisualEffectView::initWithFrame(
            NSVisualEffectView::alloc(mtm),
            NSRect::new(NSPoint::new(0.0, 0.0), DIALOG_SIZE),
        );
        content.setMaterial(NSVisualEffectMaterial::WindowBackground);
        content.setBlendingMode(NSVisualEffectBlendingMode::WithinWindow);
        content.setState(NSVisualEffectState::FollowsWindowActiveState);
        let title_label = make_label(
            "Time's up",
            NSRect::new(NSPoint::new(40.0, 140.0), NSSize::new(340.0, 34.0)),
            30.0,
            true,
            Some(&NSColor::labelColor()),
            mtm,
        );
        let subtitle_label = make_label(
            "Your timer has finished.",
            NSRect::new(NSPoint::new(40.0, 104.0), NSSize::new(340.0, 42.0)),
            15.0,
            false,
            Some(&NSColor::secondaryLabelColor()),
            mtm,
        );
        let one_more_minute_button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str("+1 min"),
                Some(action_target.as_super().as_super()),
                Some(sel!(snoozeOneMinute:)),
                mtm,
            )
        };
        one_more_minute_button.setFrame(NSRect::new(
            NSPoint::new(76.0, 44.0),
            NSSize::new(128.0, 32.0),
        ));
        one_more_minute_button.setKeyEquivalent(&NSString::from_str("\u{1b}"));
        let done_button = unsafe {
            NSButton::buttonWithTitle_target_action(
                &NSString::from_str("Done"),
                Some(action_target.as_super().as_super()),
                Some(sel!(completeBreak:)),
                mtm,
            )
        };
        done_button.setFrame(NSRect::new(
            NSPoint::new(216.0, 44.0),
            NSSize::new(128.0, 32.0),
        ));
        done_button.setKeyEquivalent(&NSString::from_str("\r"));

        content.addSubview(&title_label);
        content.addSubview(&subtitle_label);
        content.addSubview(&one_more_minute_button);
        content.addSubview(&done_button);
        dialog_window.setContentView(Some(&content));
        dialog_window.orderOut(None);

        Self {
            _action_target: action_target,
            mask_windows: Vec::new(),
            dialog_window,
            subtitle_label,
            visible: false,
        }
    }

    fn show(&mut self, break_minutes: u32) {
        self.subtitle_label
            .setStringValue(&NSString::from_str(&format!(
                "Your {} timer has finished.",
                minutes_text(break_minutes)
            )));

        if self.visible {
            return;
        }

        self.rebuild_masks();

        for mask in &self.mask_windows {
            mask.orderFrontRegardless();
        }

        position_dialog(&self.dialog_window);
        self.dialog_window.makeKeyAndOrderFront(None);
        self.dialog_window.orderFrontRegardless();
        self.visible = true;
    }

    fn hide(&mut self) {
        if self.visible {
            for mask in &self.mask_windows {
                mask.orderOut(None);
            }
            self.dialog_window.orderOut(None);
            self.visible = false;
        }
    }

    fn rebuild_masks(&mut self) {
        for mask in &self.mask_windows {
            mask.orderOut(None);
        }
        self.mask_windows.clear();

        let mtm = MainThreadMarker::new().expect("mask windows must be created on the main thread");
        let screens = NSScreen::screens(mtm);
        for index in 0..screens.count() {
            let screen = screens.objectAtIndex(index);
            let mask = create_mask_window(mtm, screen.frame());
            self.mask_windows.push(mask);
        }
    }
}

fn create_mask_window(mtm: MainThreadMarker, frame: NSRect) -> Retained<NSWindow> {
    let window = unsafe {
        NSWindow::initWithContentRect_styleMask_backing_defer(
            NSWindow::alloc(mtm),
            frame,
            NSWindowStyleMask::Borderless,
            NSBackingStoreType::Buffered,
            false,
        )
    };
    unsafe { window.setReleasedWhenClosed(false) };
    window.setBackgroundColor(Some(&NSColor::colorWithSRGBRed_green_blue_alpha(
        0.02, 0.03, 0.04, 0.55,
    )));
    window.setOpaque(false);
    window.setHasShadow(false);
    window.setIgnoresMouseEvents(false);
    window.setLevel(NSScreenSaverWindowLevel);
    window.setCollectionBehavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary,
    );
    window
}

fn make_label(
    text: &str,
    frame: NSRect,
    font_size: f64,
    bold: bool,
    text_color: Option<&NSColor>,
    mtm: MainThreadMarker,
) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFrame(frame);
    label.setAlignment(NSTextAlignment::Center);
    if let Some(color) = text_color {
        label.setTextColor(Some(color));
    }

    let font = if bold {
        NSFont::boldSystemFontOfSize(font_size)
    } else {
        NSFont::systemFontOfSize(font_size)
    };
    label.setFont(Some(&font));
    label
}

fn position_countdown_bubble(panel: &NSPanel, tray_anchor: Option<NSRect>) {
    let mtm =
        MainThreadMarker::new().expect("countdown bubble must be positioned on the main thread");
    let (target_screen, anchor_mid_x, anchor_bottom_y) = match tray_anchor {
        Some(anchor) => (
            screen_for_rect(mtm, anchor).unwrap_or_else(|| {
                NSScreen::mainScreen(mtm).unwrap_or_else(|| first_screen(mtm))
            }),
            Some(anchor.origin.x + anchor.size.width / 2.0),
            Some(anchor.origin.y),
        ),
        None => (
            NSScreen::mainScreen(mtm).unwrap_or_else(|| first_screen(mtm)),
            None,
            None,
        ),
    };
    let visible = target_screen.visibleFrame();

    let x = if let Some(anchor_mid_x) = anchor_mid_x {
        (anchor_mid_x - COUNTDOWN_BUBBLE_SIZE.width / 2.0).clamp(
            visible.origin.x + SCREEN_MARGIN,
            visible.origin.x + visible.size.width - COUNTDOWN_BUBBLE_SIZE.width - SCREEN_MARGIN,
        )
    } else {
        visible.origin.x + visible.size.width - COUNTDOWN_BUBBLE_SIZE.width - SCREEN_MARGIN
    };
    let y = if let Some(anchor_bottom_y) = anchor_bottom_y {
        (anchor_bottom_y - COUNTDOWN_BUBBLE_SIZE.height - TRAY_BUBBLE_GAP).clamp(
            visible.origin.y + SCREEN_MARGIN,
            visible.origin.y + visible.size.height - COUNTDOWN_BUBBLE_SIZE.height - SCREEN_MARGIN,
        )
    } else {
        visible.origin.y + visible.size.height - COUNTDOWN_BUBBLE_SIZE.height - SCREEN_MARGIN
    };

    panel.setFrame_display(NSRect::new(NSPoint::new(x, y), COUNTDOWN_BUBBLE_SIZE), true);
}

fn position_dialog(window: &NSWindow) {
    let mtm = MainThreadMarker::new().expect("dialog window must be positioned on the main thread");
    let target_screen = NSScreen::mainScreen(mtm).unwrap_or_else(|| first_screen(mtm));
    let visible = target_screen.visibleFrame();
    let origin = NSPoint::new(
        visible.origin.x + (visible.size.width - DIALOG_SIZE.width) / 2.0,
        visible.origin.y + (visible.size.height - DIALOG_SIZE.height) / 2.0,
    );

    window.setFrame_display(NSRect::new(origin, DIALOG_SIZE), true);
}

fn screen_for_rect(mtm: MainThreadMarker, rect: NSRect) -> Option<Retained<NSScreen>> {
    let center_x = rect.origin.x + rect.size.width / 2.0;
    let center_y = rect.origin.y + rect.size.height / 2.0;

    let screens = NSScreen::screens(mtm);
    for index in 0..screens.count() {
        let screen = screens.objectAtIndex(index);
        let frame = screen.frame();
        if point_in_rect(center_x, center_y, frame) {
            return Some(screen);
        }
    }

    None
}

fn point_in_rect(x: f64, y: f64, rect: NSRect) -> bool {
    x >= rect.origin.x
        && x <= rect.origin.x + rect.size.width
        && y >= rect.origin.y
        && y <= rect.origin.y + rect.size.height
}

fn first_screen(mtm: MainThreadMarker) -> Retained<NSScreen> {
    let screens: Retained<NSArray<NSScreen>> = NSScreen::screens(mtm);
    screens.objectAtIndex(0)
}
