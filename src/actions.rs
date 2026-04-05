use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::{MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSSlider;
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol};
use tray_icon::menu::MenuEvent;
use winit::event_loop::EventLoopProxy;

use crate::timer::{MAX_BREAK_MINUTES, MIN_BREAK_MINUTES};

thread_local! {
    static EVENT_PROXY: RefCell<Option<EventLoopProxy<UserEvent>>> = const { RefCell::new(None) };
}

#[derive(Debug)]
pub enum UserEvent {
    Menu(MenuEvent),
    ToggleEnabled,
    SetBreakMinutes(u32),
    SnoozeOneMinute,
    CompleteBreak,
}

pub fn set_event_proxy(proxy: EventLoopProxy<UserEvent>) {
    EVENT_PROXY.with(|stored_proxy| {
        *stored_proxy.borrow_mut() = Some(proxy);
    });
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    pub struct AppActionTarget;

    unsafe impl NSObjectProtocol for AppActionTarget {}

    impl AppActionTarget {
        #[unsafe(method(toggleEnabled:))]
        fn toggle_enabled(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            send_event(UserEvent::ToggleEnabled);
        }

        #[unsafe(method(sliderChanged:))]
        fn slider_changed(&self, sender: Option<&objc2::runtime::AnyObject>) {
            if let Some(slider) = sender.and_then(|sender| sender.downcast_ref::<NSSlider>()) {
                let minutes = slider
                    .doubleValue()
                    .round()
                    .clamp(MIN_BREAK_MINUTES as f64, MAX_BREAK_MINUTES as f64)
                    as u32;
                send_event(UserEvent::SetBreakMinutes(minutes));
            }
        }

        #[unsafe(method(snoozeOneMinute:))]
        fn snooze_one_minute(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            send_event(UserEvent::SnoozeOneMinute);
        }

        #[unsafe(method(completeBreak:))]
        fn complete_break(&self, _sender: Option<&objc2::runtime::AnyObject>) {
            send_event(UserEvent::CompleteBreak);
        }
    }
);

impl AppActionTarget {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        unsafe { msg_send![Self::alloc(mtm), init] }
    }
}

fn send_event(event: UserEvent) {
    EVENT_PROXY.with(|proxy| {
        if let Some(proxy) = proxy.borrow().as_ref() {
            let _ = proxy.send_event(event);
        }
    });
}
