use std::f32::consts::TAU;

use objc2::ClassType;
use objc2::rc::Retained;
use objc2::{MainThreadOnly, sel};
use objc2_app_kit::{
    NSControlStateValue, NSControlStateValueOff, NSControlStateValueOn, NSMenu, NSSlider, NSSwitch,
    NSTextAlignment, NSTextField, NSView,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use tray_icon::{
    Icon, TrayIcon, TrayIconBuilder,
    menu::{ContextMenu, Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

use crate::actions::AppActionTarget;
use crate::timer::minutes_text;

#[derive(Debug, Clone, Copy)]
pub enum TrayIconState {
    Disabled,
    Counting { progress: f32 },
    Alert,
}

pub struct MenuController {
    _status_item: MenuItem,
    _enable_item: MenuItem,
    _duration_item: MenuItem,
    _action_target: Retained<AppActionTarget>,
    controls: MenuControls,
    quit_item: MenuItem,
    tray_icon: TrayIcon,
}

struct MenuControls {
    status_label: Retained<NSTextField>,
    switch_button: Retained<NSSwitch>,
    duration_slider: Retained<NSSlider>,
    duration_value_label: Retained<NSTextField>,
}

impl MenuController {
    pub fn new(
        action_target: Retained<AppActionTarget>,
        is_enabled: bool,
        break_minutes: u32,
    ) -> Self {
        let status_item = MenuItem::new("Status: Disabled", false, None);
        let enable_item = MenuItem::new("Enabled", true, None);
        let duration_item = MenuItem::new("Duration", true, None);
        let quit_item = MenuItem::new("Quit", true, None);

        let menu = Menu::new();
        let first_separator = PredefinedMenuItem::separator();
        let second_separator = PredefinedMenuItem::separator();

        menu.append_items(&[
            &status_item,
            &first_separator,
            &enable_item,
            &duration_item,
            &second_separator,
            &quit_item,
        ])
        .expect("failed to build tray menu");

        let controls = attach_custom_controls(&menu, &action_target, is_enabled, break_minutes);
        let initial_tray_icon_state = if is_enabled {
            TrayIconState::Counting { progress: 1.0 }
        } else {
            TrayIconState::Disabled
        };

        let tray_icon = TrayIconBuilder::new()
            .with_tooltip("BreakTime")
            .with_icon(create_tray_icon(initial_tray_icon_state))
            .with_icon_as_template(true)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(true)
            .build()
            .expect("failed to create tray icon");

        let controller = Self {
            _status_item: status_item,
            _enable_item: enable_item,
            _duration_item: duration_item,
            _action_target: action_target,
            controls,
            quit_item,
            tray_icon,
        };
        controller.sync(
            "Status: Disabled",
            is_enabled,
            break_minutes,
            initial_tray_icon_state,
        );
        controller
    }

    pub fn sync(
        &self,
        status_text: &str,
        is_enabled: bool,
        break_minutes: u32,
        tray_icon_state: TrayIconState,
    ) {
        self.controls
            .status_label
            .setStringValue(&NSString::from_str(status_text));
        self.controls
            .switch_button
            .setState(switch_state(is_enabled));
        self.controls
            .duration_slider
            .setDoubleValue(break_minutes as f64);
        self.controls
            .duration_value_label
            .setStringValue(&NSString::from_str(&minutes_text(break_minutes)));
        self.tray_icon
            .set_icon_with_as_template(Some(create_tray_icon(tray_icon_state)), true)
            .expect("failed to update tray icon");
    }

    pub fn is_quit_event(&self, event: &MenuEvent) -> bool {
        event.id == self.quit_item.id()
    }

    pub fn tray_anchor_rect(&self) -> Option<NSRect> {
        let mtm = MainThreadMarker::new().expect("tray anchor lookup must run on the main thread");
        let status_item = self.tray_icon.ns_status_item()?;
        let button = status_item.button(mtm)?;
        let window = button.window()?;
        Some(window.frame())
    }
}

fn attach_custom_controls(
    menu: &Menu,
    action_target: &Retained<AppActionTarget>,
    is_enabled: bool,
    break_minutes: u32,
) -> MenuControls {
    let mtm = MainThreadMarker::new().expect("menu customization must run on the main thread");
    let ns_menu = unsafe { &*(menu.ns_menu() as *mut NSMenu) };
    let status_menu_item = ns_menu
        .itemAtIndex(0)
        .expect("status menu item must exist in tray menu");
    let enable_menu_item = ns_menu
        .itemAtIndex(2)
        .expect("enable menu item must exist in tray menu");
    let duration_menu_item = ns_menu
        .itemAtIndex(3)
        .expect("duration menu item must exist in tray menu");

    let status_container = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(240.0, 24.0)),
    );
    let status_label = NSTextField::labelWithString(&NSString::from_str("Status: Disabled"), mtm);
    status_label.setFrame(NSRect::new(
        NSPoint::new(20.0, 3.0),
        NSSize::new(200.0, 18.0),
    ));

    let switch_container = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(240.0, 28.0)),
    );
    let switch_label = NSTextField::labelWithString(&NSString::from_str("Enabled"), mtm);
    switch_label.setFrame(NSRect::new(
        NSPoint::new(20.0, 5.0),
        NSSize::new(100.0, 18.0),
    ));

    let switch_button = NSSwitch::new(mtm);
    switch_button.setFrame(NSRect::new(
        NSPoint::new(180.0, 3.0),
        NSSize::new(40.0, 22.0),
    ));
    switch_button.setState(switch_state(is_enabled));

    let slider_container = NSView::initWithFrame(
        NSView::alloc(mtm),
        NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(240.0, 62.0)),
    );

    let duration_label = NSTextField::labelWithString(&NSString::from_str("Duration"), mtm);
    duration_label.setFrame(NSRect::new(
        NSPoint::new(20.0, 38.0),
        NSSize::new(110.0, 16.0),
    ));

    let duration_value_label =
        NSTextField::labelWithString(&NSString::from_str(&minutes_text(break_minutes)), mtm);
    duration_value_label.setFrame(NSRect::new(
        NSPoint::new(160.0, 38.0),
        NSSize::new(60.0, 16.0),
    ));
    duration_value_label.setAlignment(NSTextAlignment::Right);

    let duration_slider = unsafe {
        NSSlider::sliderWithValue_minValue_maxValue_target_action(
            break_minutes as f64,
            1.0,
            20.0,
            Some(action_target.as_super().as_super()),
            Some(sel!(sliderChanged:)),
            mtm,
        )
    };
    duration_slider.setFrame(NSRect::new(
        NSPoint::new(20.0, 12.0),
        NSSize::new(200.0, 20.0),
    ));
    duration_slider.setContinuous(true);

    unsafe {
        status_menu_item.setView(Some(&status_container));
        status_container.addSubview(&status_label);

        switch_button.setTarget(Some(action_target.as_super().as_super()));
        switch_button.setAction(Some(sel!(toggleEnabled:)));
        switch_container.addSubview(&switch_label);
        switch_container.addSubview(&switch_button);
        enable_menu_item.setView(Some(&switch_container));

        slider_container.addSubview(&duration_label);
        slider_container.addSubview(&duration_value_label);
        slider_container.addSubview(&duration_slider);
        duration_menu_item.setView(Some(&slider_container));
    }

    MenuControls {
        status_label,
        switch_button,
        duration_slider,
        duration_value_label,
    }
}

fn switch_state(is_enabled: bool) -> NSControlStateValue {
    if is_enabled {
        NSControlStateValueOn
    } else {
        NSControlStateValueOff
    }
}

fn create_tray_icon(state: TrayIconState) -> Icon {
    let size = 22u32;
    let mut rgba = vec![0; (size * size * 4) as usize];
    let center_x = (size as f32) / 2.0;
    let center_y = 12.45;
    let body_radius = 7.2;
    let body_stroke = 1.45;
    let elapsed_fill_radius = body_radius - body_stroke * 0.15;

    let (
        alpha_base,
        outline_weight,
        top_button_weight,
        elapsed_fill_weight,
        alert_weight,
        progress,
    ) = match state {
        TrayIconState::Disabled => (110.0, 0.98, 0.9, 0.0, 0.0, 1.0),
        TrayIconState::Counting { progress } => (
            255.0,
            1.0,
            0.88,
            1.0,
            0.0,
            progress.clamp(0.0, 1.0),
        ),
        TrayIconState::Alert => (255.0, 1.0, 0.92, 0.0, 1.0, 0.0),
    };
    let elapsed = (1.0 - progress).clamp(0.0, 1.0);

    for y in 0..size {
        for x in 0..size {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - center_x;
            let dy = py - center_y;
            let dist = (dx * dx + dy * dy).sqrt();

            let body = circle_stroke_intensity(dist, body_radius, body_stroke) * outline_weight;
            let elapsed_fill = sector_fill_intensity(
                dx,
                dy,
                dist,
                elapsed_fill_radius,
                elapsed * TAU,
            ) * elapsed_fill_weight;
            let top_button = capsule_intensity(
                px,
                py,
                center_x - 1.85,
                center_y - 8.8,
                center_x + 1.85,
                center_y - 8.8,
                0.95,
            ) * top_button_weight;
            let alert_bar = capsule_intensity(
                px,
                py,
                center_x,
                center_y - 2.9,
                center_x,
                center_y + 0.7,
                0.7,
            ) * alert_weight;
            let alert_dot =
                circle_fill_intensity(distance(px, py, center_x, center_y + 3.2), 0.85)
                    * alert_weight;

            let intensity = body
                .max(elapsed_fill)
                .max(top_button)
                .max(alert_bar)
                .max(alert_dot);
            if intensity > 0.0 {
                let index = ((y * size + x) * 4) as usize;
                rgba[index] = 0;
                rgba[index + 1] = 0;
                rgba[index + 2] = 0;
                rgba[index + 3] = (intensity * alpha_base) as u8;
            }
        }
    }

    Icon::from_rgba(rgba, size, size).expect("failed to create tray icon pixels")
}

fn circle_stroke_intensity(distance_from_center: f32, radius: f32, stroke_width: f32) -> f32 {
    let half_width = stroke_width / 2.0;
    let signed_distance = half_width - (distance_from_center - radius).abs();
    soft_edge_alpha(signed_distance, 0.8)
}

fn circle_fill_intensity(distance_from_center: f32, radius: f32) -> f32 {
    soft_edge_alpha(radius - distance_from_center, 0.8)
}

fn sector_fill_intensity(
    dx: f32,
    dy: f32,
    distance_from_center: f32,
    radius: f32,
    sweep: f32,
) -> f32 {
    let fill = circle_fill_intensity(distance_from_center, radius);
    if fill <= 0.0 {
        return 0.0;
    }

    let sweep = sweep.clamp(0.0, TAU);
    if sweep <= 0.0 {
        return 0.0;
    }
    if sweep >= TAU - 0.001 {
        return fill;
    }

    let angle = normalized_angle_from_top(dx, dy);
    let angular_softness = (0.9 / distance_from_center.max(1.0)).clamp(0.03, 0.35);
    let boundary_alpha = soft_edge_alpha(sweep - angle, angular_softness);
    fill * boundary_alpha
}

fn capsule_intensity(px: f32, py: f32, x0: f32, y0: f32, x1: f32, y1: f32, radius: f32) -> f32 {
    let pa_x = px - x0;
    let pa_y = py - y0;
    let ba_x = x1 - x0;
    let ba_y = y1 - y0;
    let ba_len = ba_x * ba_x + ba_y * ba_y;
    let projection = if ba_len > 0.0 {
        ((pa_x * ba_x + pa_y * ba_y) / ba_len).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let dx = pa_x - ba_x * projection;
    let dy = pa_y - ba_y * projection;
    let distance_from_segment = (dx * dx + dy * dy).sqrt();
    soft_edge_alpha(radius - distance_from_segment, 0.8)
}

fn distance(x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let dx = x0 - x1;
    let dy = y0 - y1;
    (dx * dx + dy * dy).sqrt()
}

fn normalized_angle_from_top(dx: f32, dy: f32) -> f32 {
    let angle = dx.atan2(-dy);
    if angle < 0.0 { angle + TAU } else { angle }
}

fn soft_edge_alpha(signed_distance: f32, softness: f32) -> f32 {
    if softness <= 0.0 {
        return if signed_distance >= 0.0 { 1.0 } else { 0.0 };
    }

    let t = ((signed_distance / softness) + 1.0) * 0.5;
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
