use std::{error::Error, time::Instant};

use objc2_foundation::MainThreadMarker;
use tray_icon::{MouseButtonState, TrayIconEvent, menu::MenuEvent};
use winit::{
    application::ApplicationHandler,
    event::{StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    platform::macos::{ActivationPolicy, EventLoopBuilderExtMacOS},
    window::WindowId,
};

use crate::{
    actions::{AppActionTarget, UserEvent, set_event_proxy},
    menu::{MenuController, TrayIconState},
    timer::{BreakTimer, DEFAULT_BREAK_MINUTES, TimerMode},
    windows::WindowController,
};

pub fn run() -> Result<(), Box<dyn Error>> {
    let mut builder = EventLoop::<UserEvent>::with_user_event();
    builder.with_default_menu(false);
    builder.with_activation_policy(ActivationPolicy::Accessory);

    let event_loop = builder.build()?;
    let proxy = event_loop.create_proxy();
    set_event_proxy(proxy.clone());

    TrayIconEvent::set_event_handler(Some(|event| {
        if matches!(
            event,
            TrayIconEvent::Click {
                button_state: MouseButtonState::Down,
                ..
            }
        ) {
            activate_app();
        }
    }));

    MenuEvent::set_event_handler(Some(move |event| {
        let _ = proxy.send_event(UserEvent::Menu(event));
    }));

    let mut app = App::new();
    event_loop.run_app(&mut app)?;
    Ok(())
}

fn activate_app() {
    let mtm = MainThreadMarker::new().expect("activation must happen on the main thread");
    let app = objc2_app_kit::NSApplication::sharedApplication(mtm);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
}

struct App {
    last_tray_icon_step: Option<u32>,
    menu: Option<MenuController>,
    windows: Option<WindowController>,
    timer: BreakTimer,
    is_enabled: bool,
    break_minutes: u32,
}

impl App {
    fn new() -> Self {
        Self {
            last_tray_icon_step: None,
            menu: None,
            windows: None,
            timer: BreakTimer::new(DEFAULT_BREAK_MINUTES),
            is_enabled: false,
            break_minutes: DEFAULT_BREAK_MINUTES,
        }
    }

    fn ensure_components(&mut self) {
        if self.menu.is_some() && self.windows.is_some() {
            return;
        }

        let mtm =
            MainThreadMarker::new().expect("UI components must be created on the main thread");
        let menu_target = AppActionTarget::new(mtm);
        let window_target = AppActionTarget::new(mtm);

        self.menu = Some(MenuController::new(
            menu_target,
            self.is_enabled,
            self.break_minutes,
        ));
        self.windows = Some(WindowController::new(window_target));
        let now = Instant::now();
        self.sync_menu(now);
        self.sync_windows(now);
    }

    fn sync_menu(&mut self, now: Instant) {
        self.last_tray_icon_step = self.timer.tray_icon_step(now);
        let tray_icon_state = self.tray_icon_state(now);
        if let Some(menu) = &self.menu {
            menu.sync(
                &self.status_text(),
                self.is_enabled,
                self.break_minutes,
                tray_icon_state,
            );
        }
    }

    fn sync_windows(&mut self, now: Instant) {
        let tray_anchor = self.menu.as_ref().and_then(|menu| menu.tray_anchor_rect());
        let Some(windows) = self.windows.as_mut() else {
            return;
        };

        match self.timer.mode() {
            TimerMode::Alert => {
                windows.hide_bubble();
                windows.show_alert(self.break_minutes);
            }
            _ if !self.is_enabled => windows.hide_all(),
            TimerMode::Idle => windows.hide_all(),
            TimerMode::Counting => {
                windows.hide_alert();
                if let Some(seconds) = self.timer.bubble_seconds(now) {
                    windows.show_bubble(seconds, tray_anchor);
                } else {
                    windows.hide_bubble();
                }
            }
        }
    }

    fn toggle_enabled(&mut self) {
        self.is_enabled = !self.is_enabled;
        let now = Instant::now();

        if self.is_enabled {
            self.timer.start(now);
        } else {
            self.timer.stop();
        }

        self.sync_menu(now);
        self.sync_windows(now);
    }

    fn set_break_minutes(&mut self, minutes: u32) {
        self.break_minutes = minutes;
        let now = Instant::now();
        self.timer.set_duration_minutes(minutes, now);
        self.sync_menu(now);
        self.sync_windows(now);
    }

    fn snooze_one_minute(&mut self) {
        let now = Instant::now();
        self.is_enabled = true;
        self.timer.start_snooze(now);
        self.sync_menu(now);
        self.sync_windows(now);
    }

    fn complete_break(&mut self) {
        let now = Instant::now();
        self.is_enabled = true;
        self.timer.start(now);
        self.sync_menu(now);
        self.sync_windows(now);
    }

    fn refresh_timer(&mut self) {
        let now = Instant::now();
        let did_reach_alert = self.is_enabled && self.timer.tick(now);
        if did_reach_alert {
            self.is_enabled = false;
        }
        let tray_icon_step = self.timer.tray_icon_step(now);

        if did_reach_alert || tray_icon_step != self.last_tray_icon_step {
            self.sync_menu(now);
        }
        self.sync_windows(now);
    }

    fn tray_icon_state(&self, now: Instant) -> TrayIconState {
        match self.timer.mode() {
            TimerMode::Alert => TrayIconState::Alert,
            _ if !self.is_enabled => TrayIconState::Disabled,
            TimerMode::Idle => TrayIconState::Counting { progress: 1.0 },
            TimerMode::Counting => TrayIconState::Counting {
                progress: self.timer.tray_icon_progress(now).unwrap_or(1.0),
            },
        }
    }

    fn status_text(&self) -> String {
        match self.timer.mode() {
            TimerMode::Alert => "Status: Time's up".to_string(),
            _ if !self.is_enabled => "Status: Disabled".to_string(),
            _ => "Status: Enabled".to_string(),
        }
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if matches!(cause, StartCause::Init) {
            self.ensure_components();
        }

        self.refresh_timer();
    }

    fn resumed(&mut self, _event_loop: &ActiveEventLoop) {}

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Menu(event) => {
                if self
                    .menu
                    .as_ref()
                    .is_some_and(|menu| menu.is_quit_event(&event))
                {
                    event_loop.exit();
                }
            }
            UserEvent::ToggleEnabled => self.toggle_enabled(),
            UserEvent::SetBreakMinutes(minutes) => self.set_break_minutes(minutes),
            UserEvent::SnoozeOneMinute => self.snooze_one_minute(),
            UserEvent::CompleteBreak => self.complete_break(),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.refresh_timer();
        event_loop.set_control_flow(self.timer.next_control_flow(Instant::now()));
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        _event: WindowEvent,
    ) {
    }
}
