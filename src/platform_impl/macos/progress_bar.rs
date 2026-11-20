use std::cell::Cell;

use dispatch2::DispatchQueue;
use objc2::{define_class, msg_send, rc::Retained, DefinedClass, MainThreadMarker, MainThreadOnly};
use objc2_app_kit::{
  NSApplication, NSBezierPath, NSColor, NSDockTile, NSImageView, NSProgressIndicator, NSResponder,
  NSView,
};
use objc2_foundation::{NSInsetRect, NSObject, NSPoint, NSRect, NSSize};

use crate::window::{ProgressBarState, ProgressState};

/// Set progress indicator in the Dock.
pub fn set_progress_indicator(progress_state: ProgressBarState) {
  DispatchQueue::main().exec_async(move || {
    let mtm = unsafe { MainThreadMarker::new_unchecked() };
    let app = NSApplication::sharedApplication(mtm);
    let dock_tile = unsafe { app.dockTile() };
    // check progress indicator is already set or create new one
    let progress_indicator = get_existing_progress_indicator(&dock_tile, mtm)
      .unwrap_or_else(|| create_progress_indicator(&app, &dock_tile, mtm));

    if let Some(progress) = progress_state.progress {
      unsafe { progress_indicator.setDoubleValue(progress.clamp(0, 100) as f64) };
      progress_indicator.setHidden(false);
    }
    if let Some(state) = progress_state.state {
      progress_indicator.ivars().set(state);
      progress_indicator.setHidden(matches!(state, ProgressState::None));
    }

    unsafe { dock_tile.display() };
  });
}

fn create_progress_indicator(
  app: &NSApplication,
  dock_tile: &NSDockTile,
  mtm: MainThreadMarker,
) -> Retained<TaoProgressIndicator> {
  let content_view = unsafe {
    dock_tile.contentView(mtm).unwrap_or_else(|| {
      // create new dock tile view with current app icon
      let app_icon_image = app.applicationIconImage().unwrap();
      let image_view = NSImageView::imageViewWithImage(&app_icon_image, mtm);
      dock_tile.setContentView(Some(&image_view));
      image_view.into_super().into_super()
    })
  };

  let frame = NSRect::new(
    NSPoint::new(0.0, 0.0),
    NSSize::new(unsafe { dock_tile.size() }.width, 15.0),
  );
  let progress_indicator =
    TaoProgressIndicator::alloc(mtm).set_ivars(Cell::new(ProgressState::None));
  let progress_indicator: Retained<TaoProgressIndicator> =
    unsafe { msg_send![super(progress_indicator), initWithFrame: frame] };

  // set progress indicator to the dock tile
  unsafe { content_view.addSubview(&progress_indicator) };

  progress_indicator
}

fn get_existing_progress_indicator(
  dock_tile: &NSDockTile,
  mtm: MainThreadMarker,
) -> Option<Retained<TaoProgressIndicator>> {
  unsafe { dock_tile.contentView(mtm)?.subviews() }
    .into_iter()
    .find_map(|view| view.downcast::<TaoProgressIndicator>().ok())
}

define_class!(
  #[unsafe(super(NSProgressIndicator, NSView, NSResponder, NSObject))]
  #[thread_kind = MainThreadOnly]
  #[name = "TaoProgressIndicator"]
  #[ivars = Cell<ProgressState>]
  struct TaoProgressIndicator;

  impl TaoProgressIndicator {
    #[unsafe(method(drawRect:))]
    fn draw_progress_bar(&self, rect: NSRect) {
      let bar = NSRect::new(
        NSPoint { x: 0.0, y: 4.0 },
        NSSize {
          width: rect.size.width,
          height: 8.0,
        },
      );
      let bar_inner = NSInsetRect(bar, 0.5, 0.5);
      let mut bar_progress = NSInsetRect(bar, 1.0, 1.0);

      // set progress width
      let current_progress = unsafe { self.doubleValue() };
      let normalized_progress = (current_progress / 100.0).clamp(0.0, 1.0);
      bar_progress.size.width *= normalized_progress;

    // draw outer bar
      unsafe {
        let bg_color = NSColor::colorWithWhite_alpha(1.0, 0.05);
        bg_color.set();
      }
      draw_rounded_rect(bar);
      // draw inner bar
      draw_rounded_rect(bar_inner);

      // draw progress
      let progress_color = match self.ivars().get() {
        ProgressState::Paused => unsafe { NSColor::systemYellowColor() },
        ProgressState::Error => unsafe { NSColor::systemRedColor() },
        _ => unsafe { NSColor::systemBlueColor() },
      };
      unsafe { progress_color.set() };
      draw_rounded_rect(bar_progress);
    }
  }
);

fn draw_rounded_rect(rect: NSRect) {
  let radius = rect.size.height / 2.0;
  unsafe { NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(rect, radius, radius).fill() };
}
