// Copyright 2016-2025 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0
// SPDX-License-Identifier: MIT

//! DWM-anchored shadows for undecorated windows.
//!
//! The shadow of an undecorated window is anchored by extending the DWM frame
//! one pixel into the client area on every edge, instead of keeping the native
//! frame styles alive and carving the client back in through `WM_NCCALCSIZE`
//! insets. The client area therefore always equals the window rect, and the
//! window gets a symmetric border on every edge — including the top edge on
//! Windows 10, which the inset approach can never draw (any top non-client
//! strip there makes the system paint a full native titlebar).
//!
//! - `border_transparent = false` (default) reserves one pixel of glass on
//!   every edge: Windows 11 composites the system border above the client
//!   content, while on Windows 10 the lines are drawn inside the client area
//!   and stay visible only if the content leaves those pixels uncovered (see
//!   `Window::undecorated_frame_inset`).
//! - `border_transparent = true` keeps the shadow but drops the visible
//!   border: on Windows 11 the frame extension still reserves the pixel on
//!   every edge and `DWMWA_BORDER_COLOR` is set to `COLOR_NONE`. On Windows
//!   10 the top margin stays at zero and the glass lines on the other three
//!   edges remain in the client area, where the content covers them.
//!
//! `DWMWA_BORDER_COLOR` is a Windows 11 attribute; the call is expected to
//! fail on older systems and is deliberately not logged.

use std::mem::size_of;

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::{
  DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMNCRP_ENABLED, DWMNCRP_USEWINDOWSTYLE,
  DWMWA_BORDER_COLOR, DWMWA_NCRENDERING_POLICY,
};
use windows::Win32::Graphics::Gdi::{FillRect, GetStockObject, BLACK_BRUSH, HBRUSH, HDC};
use windows::Win32::UI::Controls::MARGINS;

use super::util;

/// Restores the system default border outline (`DWMWA_COLOR_DEFAULT`).
const BORDER_COLOR_DEFAULT: u32 = u32::MAX;
/// Draws no border outline at all (`DWMWA_COLOR_NONE`, Windows 11+).
const BORDER_COLOR_NONE: u32 = 0xFFFF_FFFE;

/// Width of the client-area ring the frame extension reserves, in physical
/// pixels. Shared by the DWM margins, the erase handler that paints the ring
/// black, and `Window::undecorated_frame_inset`.
pub(crate) const RING_PX: i32 = 1;

/// Whether the running Windows composites the border above the client content
/// (Windows 11+), which decides both the reserved margins and whether content
/// must keep the ring uncovered. Every build check for the ring goes through
/// this so the sites cannot drift.
pub(crate) fn composites_border_above_content() -> bool {
  util::WIN_VERSION.build >= 22000
}

/// Applies or resets the DWM-anchored shadow of an undecorated window.
///
/// `enabled` must already account for the window state (decorated, maximized
/// and fullscreen windows get `false`); this function only translates the two
/// booleans into DWM attributes.
///
/// Returns whether the essential call (`DwmExtendFrameIntoClientArea`)
/// succeeded, so callers can avoid caching a failed apply. The expected
/// pre-Windows-11 failure of `DWMWA_BORDER_COLOR` is not part of this result.
pub(crate) fn set_undecorated_shadow(hwnd: HWND, enabled: bool, border_transparent: bool) -> bool {
  let (policy, border_color, margins) = if enabled {
    let border_color = if border_transparent {
      BORDER_COLOR_NONE
    } else {
      BORDER_COLOR_DEFAULT
    };
    let margins = {
      let mut margins = MARGINS {
        cxLeftWidth: RING_PX,
        cyTopHeight: RING_PX,
        cxRightWidth: RING_PX,
        cyBottomHeight: RING_PX,
      };
      if border_transparent && !composites_border_above_content() {
        // Windows 10: a top frame extension shows as a visible glass line, so
        // the top stays at zero and the other three lines end up underneath
        // the client content.
        margins.cyTopHeight = 0;
      }
      margins
    };
    (DWMNCRP_ENABLED.0, border_color, margins)
  } else {
    // Restore the window-style driven DWM defaults. Used for decorated
    // windows, maximized/fullscreen windows and undecorated windows without
    // a shadow.
    (
      DWMNCRP_USEWINDOWSTYLE.0,
      BORDER_COLOR_DEFAULT,
      MARGINS::default(),
    )
  };

  // SAFETY: `hwnd` is a live window handle and `policy`, `border_color` and
  // `margins` are initialized values whose sizes match what the DWM APIs
  // expect for those attributes.
  unsafe {
    if let Err(e) = DwmSetWindowAttribute(
      hwnd,
      DWMWA_NCRENDERING_POLICY,
      &policy as *const _ as _,
      size_of::<i32>() as u32,
    ) {
      log::warn!("failed to set the DWM nonclient rendering policy: {e}");
    }
    // Deliberately not logged: the attribute does not exist before Windows 11,
    // so a failure here is the expected path on older systems.
    let _ = DwmSetWindowAttribute(
      hwnd,
      DWMWA_BORDER_COLOR,
      &border_color as *const _ as _,
      size_of::<u32>() as u32,
    );
    match DwmExtendFrameIntoClientArea(hwnd, &margins) {
      Ok(()) => true,
      Err(e) => {
        log::warn!("failed to extend the DWM frame into the client area: {e}");
        false
      }
    }
  }
}

/// Paints the one-pixel black border ring inside `rc` (the four edge strips
/// the frame extension reserves). The frame lines only show over black
/// pixels, so this must run whenever the ring's content is not otherwise
/// covered. Returns whether every strip was painted.
///
/// # Safety
///
/// `hdc` must be a valid device context and `rc` must lie within the surface
/// it paints.
pub(crate) unsafe fn paint_border_ring(hdc: HDC, rc: &RECT) -> bool {
  let black = HBRUSH(GetStockObject(BLACK_BRUSH).0);
  let ring = RING_PX;
  let strips = [
    RECT {
      left: rc.left,
      top: rc.top,
      right: rc.left + ring,
      bottom: rc.bottom,
    },
    RECT {
      left: rc.left,
      top: rc.top,
      right: rc.right,
      bottom: rc.top + ring,
    },
    RECT {
      left: rc.right - ring,
      top: rc.top,
      right: rc.right,
      bottom: rc.bottom,
    },
    RECT {
      left: rc.left,
      top: rc.bottom - ring,
      right: rc.right,
      bottom: rc.bottom,
    },
  ];
  // Paint every strip even if one fails: short-circuiting would leave the
  // remaining strips as stale pixels.
  let mut painted = true;
  for strip in &strips {
    painted = FillRect(hdc, strip, black) != 0 && painted;
  }
  painted
}
