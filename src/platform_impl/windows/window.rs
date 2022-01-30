// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

#![cfg(target_os = "windows")]

use mem::MaybeUninit;
use parking_lot::Mutex;
use raw_window_handle::{RawWindowHandle, Win32Handle};
use std::{
  cell::{Cell, RefCell},
  ffi::OsStr,
  io, mem,
  os::windows::ffi::OsStrExt,
  ptr,
  sync::Arc,
};

use crossbeam_channel as channel;
use windows::Win32::{
  Foundation::{self as win32f, HINSTANCE, HWND, LPARAM, LRESULT, POINT, PWSTR, RECT, WPARAM},
  Graphics::{
    Dwm::{DwmEnableBlurBehindWindow, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND},
    Gdi::*,
  },
  System::{Com::*, LibraryLoader::*, Ole::*},
  UI::{
    Input::{Ime::*, KeyboardAndMouse::*, Touch::*},
    Shell::*,
    WindowsAndMessaging::{self as win32wm, *},
  },
};

use crate::{
  dpi::{PhysicalPosition, PhysicalSize, Position, Size},
  error::{ExternalError, NotSupportedError, OsError as RootOsError},
  icon::Icon,
  menu::MenuType,
  monitor::MonitorHandle as RootMonitorHandle,
  platform_impl::platform::{
    dark_mode::try_theme,
    dpi::{dpi_to_scale_factor, hwnd_dpi},
    drop_handler::FileDropHandler,
    event_loop::{self, EventLoopWindowTarget, DESTROY_MSG_ID},
    icon::{self, IconType},
    menu, monitor, util,
    window_state::{CursorFlags, SavedWindow, WindowFlags, WindowState},
    OsError, Parent, PlatformSpecificWindowBuilderAttributes, WindowId,
  },
  window::{
    CursorIcon, Fullscreen, Theme, UserAttentionType, WindowAttributes, WindowId as RootWindowId,
    BORDERLESS_RESIZE_INSET,
  },
};

struct HMenuWrapper(HMENU);
unsafe impl Send for HMenuWrapper {}
unsafe impl Sync for HMenuWrapper {}

/// The Win32 implementation of the main `Window` object.
pub struct Window {
  /// Main handle for the window.
  window: WindowWrapper,

  /// The current window state.
  window_state: Arc<Mutex<WindowState>>,

  // The events loop proxy.
  thread_executor: event_loop::EventLoopThreadExecutor,

  // The menu associated with the window
  menu: Option<HMenuWrapper>,
}

impl Window {
  pub fn new<T: 'static>(
    event_loop: &EventLoopWindowTarget<T>,
    w_attr: WindowAttributes,
    pl_attr: PlatformSpecificWindowBuilderAttributes,
  ) -> Result<Window, RootOsError> {
    // We dispatch an `init` function because of code style.
    // First person to remove the need for cloning here gets a cookie!
    //
    // done. you owe me -- ossi
    unsafe {
      let drag_and_drop = pl_attr.drag_and_drop;
      init(w_attr, pl_attr, event_loop).map(|win| {
        let file_drop_handler = if drag_and_drop {
          // It is ok if the initialize result is `S_FALSE` because it might happen that
          // multiple windows are created on the same thread.
          if let Err(error) = OleInitialize(ptr::null_mut()) {
            match error.code() {
              win32f::OLE_E_WRONGCOMPOBJ => {
                panic!("OleInitialize failed! Result was: `OLE_E_WRONGCOMPOBJ`")
              }
              win32f::RPC_E_CHANGED_MODE => panic!(
                "OleInitialize failed! Result was: `RPC_E_CHANGED_MODE`. \
                Make sure other crates are not using multithreaded COM library \
                on the same thread or disable drag and drop support."
              ),
              _ => (),
            };
          }

          let file_drop_runner = event_loop.runner_shared.clone();
          let file_drop_handler: IDropTarget = FileDropHandler::new(
            win.window.0,
            Box::new(move |event| {
              if let Ok(e) = event.map_nonuser_event() {
                file_drop_runner.send_event(e)
              }
            }),
          )
          .into();

          assert!(RegisterDragDrop(win.window.0, file_drop_handler.clone()).is_ok());
          Some(file_drop_handler)
        } else {
          None
        };

        let subclass_input = event_loop::SubclassInput {
          window_state: win.window_state.clone(),
          event_loop_runner: event_loop.runner_shared.clone(),
          _file_drop_handler: file_drop_handler,
          subclass_removed: Cell::new(false),
          recurse_depth: Cell::new(0),
        };

        event_loop::subclass_window(win.window.0, subclass_input);
        win
      })
    }
  }

  pub fn set_title(&self, text: &str) {
    unsafe {
      SetWindowTextW(self.window.0, text);
    }
  }

  // TODO (lemarier): allow menu update
  pub fn set_menu(&self, _new_menu: Option<menu::Menu>) {}

  #[inline]
  pub fn set_visible(&self, visible: bool) {
    let window = self.window.clone();
    let window_state = Arc::clone(&self.window_state);
    self.thread_executor.execute_in_thread(move || {
      WindowState::set_window_flags(window_state.lock(), window.0, |f| {
        f.set(WindowFlags::VISIBLE, visible)
      });
    });
  }

  #[inline]
  pub fn set_focus(&self) {
    let window = self.window.clone();
    let window_flags = self.window_state.lock().window_flags();

    let is_visible = window_flags.contains(WindowFlags::VISIBLE);
    let is_minimized = window_flags.contains(WindowFlags::MINIMIZED);
    let is_foreground = window.0 == unsafe { GetForegroundWindow() };

    if is_visible && !is_minimized && !is_foreground {
      unsafe { force_window_active(window.0) };
    }
  }

  #[inline]
  pub fn request_redraw(&self) {
    unsafe {
      RedrawWindow(
        self.window.0,
        ptr::null(),
        HRGN::default(),
        RDW_INTERNALPAINT,
      );
    }
  }

  #[inline]
  pub fn outer_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
    util::get_window_rect(self.window.0)
      .map(|rect| Ok(PhysicalPosition::new(rect.left as i32, rect.top as i32)))
      .expect("Unexpected GetWindowRect failure")
  }

  #[inline]
  pub fn inner_position(&self) -> Result<PhysicalPosition<i32>, NotSupportedError> {
    let mut position = POINT::default();
    if !unsafe { ClientToScreen(self.window.0, &mut position) }.as_bool() {
      panic!("Unexpected ClientToScreen failure")
    }
    Ok(PhysicalPosition::new(position.x as i32, position.y as i32))
  }

  #[inline]
  pub fn set_outer_position(&self, position: Position) {
    let (x, y): (i32, i32) = position.to_physical::<i32>(self.scale_factor()).into();

    let window_state = Arc::clone(&self.window_state);
    let window = self.window.clone();
    self.thread_executor.execute_in_thread(move || {
      WindowState::set_window_flags(window_state.lock(), window.0, |f| {
        f.set(WindowFlags::MAXIMIZED, false)
      });
    });

    unsafe {
      SetWindowPos(
        self.window.0,
        HWND::default(),
        x as i32,
        y as i32,
        0,
        0,
        SWP_ASYNCWINDOWPOS | SWP_NOZORDER | SWP_NOSIZE | SWP_NOACTIVATE,
      );
      InvalidateRgn(self.window.0, HRGN::default(), false);
    }
  }

  #[inline]
  pub fn inner_size(&self) -> PhysicalSize<u32> {
    let mut rect = RECT::default();
    if !unsafe { GetClientRect(self.window.0, &mut rect) }.as_bool() {
      panic!("Unexpected GetClientRect failure")
    }
    PhysicalSize::new(
      (rect.right - rect.left) as u32,
      (rect.bottom - rect.top) as u32,
    )
  }

  #[inline]
  pub fn outer_size(&self) -> PhysicalSize<u32> {
    util::get_window_rect(self.window.0)
      .map(|rect| {
        PhysicalSize::new(
          (rect.right - rect.left) as u32,
          (rect.bottom - rect.top) as u32,
        )
      })
      .unwrap()
  }

  #[inline]
  pub fn set_inner_size(&self, size: Size) {
    let scale_factor = self.scale_factor();
    let (width, height) = size.to_physical::<u32>(scale_factor).into();

    let window_state = Arc::clone(&self.window_state);
    let window = self.window.clone();
    self.thread_executor.execute_in_thread(move || {
      WindowState::set_window_flags(window_state.lock(), window.0, |f| {
        f.set(WindowFlags::MAXIMIZED, false)
      });
    });

    util::set_inner_size_physical(self.window.0, width, height);
  }

  #[inline]
  pub fn set_min_inner_size(&self, size: Option<Size>) {
    self.window_state.lock().min_size = size;
    // Make windows re-check the window size bounds.
    let size = self.inner_size();
    self.set_inner_size(size.into());
  }

  #[inline]
  pub fn set_max_inner_size(&self, size: Option<Size>) {
    self.window_state.lock().max_size = size;
    // Make windows re-check the window size bounds.
    let size = self.inner_size();
    self.set_inner_size(size.into());
  }

  #[inline]
  pub fn set_resizable(&self, resizable: bool) {
    let window = self.window.clone();
    let window_state = Arc::clone(&self.window_state);

    self.thread_executor.execute_in_thread(move || {
      WindowState::set_window_flags(window_state.lock(), window.0, |f| {
        f.set(WindowFlags::RESIZABLE, resizable)
      });
    });
  }

  /// Returns the `hwnd` of this window.
  #[inline]
  pub fn hwnd(&self) -> HWND {
    self.window.0
  }

  #[inline]
  pub fn hinstance(&self) -> HINSTANCE {
    HINSTANCE(util::GetWindowLongPtrW(self.hwnd(), GWLP_HINSTANCE))
  }

  #[inline]
  pub fn raw_window_handle(&self) -> RawWindowHandle {
    let mut handle = Win32Handle::empty();
    handle.hwnd = self.window.0 .0 as *mut _;
    handle.hinstance = self.hinstance().0 as *mut _;
    RawWindowHandle::Win32(handle)
  }

  #[inline]
  pub fn set_cursor_icon(&self, cursor: CursorIcon) {
    self.window_state.lock().mouse.cursor = cursor;
    self.thread_executor.execute_in_thread(move || unsafe {
      let cursor = LoadCursorW(HINSTANCE::default(), cursor.to_windows_cursor());
      SetCursor(cursor);
    });
  }

  #[inline]
  pub fn set_cursor_grab(&self, grab: bool) -> Result<(), ExternalError> {
    let window = self.window.clone();
    let window_state = Arc::clone(&self.window_state);
    let (tx, rx) = channel::unbounded();

    self.thread_executor.execute_in_thread(move || {
      let result = window_state
        .lock()
        .mouse
        .set_cursor_flags(window.0, |f| f.set(CursorFlags::GRABBED, grab))
        .map_err(|e| ExternalError::Os(os_error!(OsError::IoError(e))));
      let _ = tx.send(result);
    });
    rx.recv().unwrap()
  }

  #[inline]
  pub fn set_cursor_visible(&self, visible: bool) {
    let window = self.window.clone();
    let window_state = Arc::clone(&self.window_state);
    let (tx, rx) = channel::unbounded();

    self.thread_executor.execute_in_thread(move || {
      let result = window_state
        .lock()
        .mouse
        .set_cursor_flags(window.0, |f| f.set(CursorFlags::HIDDEN, !visible))
        .map_err(|e| e.to_string());
      let _ = tx.send(result);
    });
    rx.recv().unwrap().ok();
  }

  #[inline]
  pub fn scale_factor(&self) -> f64 {
    self.window_state.lock().scale_factor
  }

  #[inline]
  pub fn set_cursor_position(&self, position: Position) -> Result<(), ExternalError> {
    let scale_factor = self.scale_factor();
    let (x, y) = position.to_physical::<i32>(scale_factor).into();

    let mut point = POINT { x, y };
    unsafe {
      if !ClientToScreen(self.window.0, &mut point).as_bool() {
        return Err(ExternalError::Os(os_error!(OsError::IoError(
          io::Error::last_os_error()
        ))));
      }
      if !SetCursorPos(point.x, point.y).as_bool() {
        return Err(ExternalError::Os(os_error!(OsError::IoError(
          io::Error::last_os_error()
        ))));
      }
    }
    Ok(())
  }

  #[inline]
  pub fn drag_window(&self) -> Result<(), ExternalError> {
    let mut pos = POINT::default();
    unsafe {
      GetCursorPos(&mut pos);
      ReleaseCapture();
      PostMessageW(
        self.window.0,
        WM_NCLBUTTONDOWN,
        WPARAM(HTCAPTION as _),
        util::MAKELPARAM(pos.x as i16, pos.y as i16),
      );
    }

    Ok(())
  }

  #[inline]
  pub fn id(&self) -> WindowId {
    WindowId(self.window.0 .0)
  }

  #[inline]
  pub fn set_minimized(&self, minimized: bool) {
    let window = self.window.clone();
    let window_state = Arc::clone(&self.window_state);

    self.thread_executor.execute_in_thread(move || {
      WindowState::set_window_flags(window_state.lock(), window.0, |f| {
        f.set(WindowFlags::MINIMIZED, minimized)
      });
    });
  }

  #[inline]
  pub fn set_maximized(&self, maximized: bool) {
    let window = self.window.clone();
    let window_state = Arc::clone(&self.window_state);

    self.thread_executor.execute_in_thread(move || {
      WindowState::set_window_flags(window_state.lock(), window.0, |f| {
        f.set(WindowFlags::MAXIMIZED, maximized)
      });
    });
  }

  #[inline]
  pub fn is_maximized(&self) -> bool {
    let window_state = self.window_state.lock();
    window_state.window_flags.contains(WindowFlags::MAXIMIZED)
  }

  #[inline]
  pub fn is_resizable(&self) -> bool {
    let window_state = self.window_state.lock();
    window_state.window_flags.contains(WindowFlags::RESIZABLE)
  }

  #[inline]
  pub fn is_decorated(&self) -> bool {
    let window_state = self.window_state.lock();
    window_state.window_flags.contains(WindowFlags::DECORATIONS)
  }

  #[inline]
  pub fn is_visible(&self) -> bool {
    util::is_visible(self.window.0)
  }

  #[inline]
  pub fn fullscreen(&self) -> Option<Fullscreen> {
    let window_state = self.window_state.lock();
    window_state.fullscreen.clone()
  }

  #[inline]
  pub fn set_fullscreen(&self, fullscreen: Option<Fullscreen>) {
    let window = self.window.clone();
    let window_state = Arc::clone(&self.window_state);

    let mut window_state_lock = window_state.lock();
    let old_fullscreen = window_state_lock.fullscreen.clone();
    if window_state_lock.fullscreen == fullscreen {
      return;
    }
    window_state_lock.fullscreen = fullscreen.clone();
    drop(window_state_lock);

    self.thread_executor.execute_in_thread(move || {
      // Change video mode if we're transitioning to or from exclusive
      // fullscreen
      match (&old_fullscreen, &fullscreen) {
        (&None, &Some(Fullscreen::Exclusive(ref video_mode)))
        | (&Some(Fullscreen::Borderless(_)), &Some(Fullscreen::Exclusive(ref video_mode)))
        | (&Some(Fullscreen::Exclusive(_)), &Some(Fullscreen::Exclusive(ref video_mode))) => {
          let monitor = video_mode.monitor();

          let mut display_name = OsStr::new(&monitor.inner.native_identifier())
            .encode_wide()
            .collect::<Vec<_>>();
          // `encode_wide` does not add a null-terminator but
          // `ChangeDisplaySettingsExW` requires a null-terminated
          // string, so add it
          display_name.push(0);

          let native_video_mode = video_mode.video_mode.native_video_mode;

          let res = unsafe {
            ChangeDisplaySettingsExW(
              PWSTR(display_name.as_mut_ptr()),
              &native_video_mode,
              HWND::default(),
              CDS_FULLSCREEN,
              std::ptr::null_mut(),
            )
          };

          debug_assert!(res != DISP_CHANGE_BADFLAGS);
          debug_assert!(res != DISP_CHANGE_BADMODE);
          debug_assert!(res != DISP_CHANGE_BADPARAM);
          debug_assert!(res != DISP_CHANGE_FAILED);
          assert_eq!(res, DISP_CHANGE_SUCCESSFUL);
        }
        (&Some(Fullscreen::Exclusive(_)), &None)
        | (&Some(Fullscreen::Exclusive(_)), &Some(Fullscreen::Borderless(_))) => {
          let res = unsafe {
            ChangeDisplaySettingsExW(
              PWSTR::default(),
              std::ptr::null_mut(),
              HWND::default(),
              CDS_FULLSCREEN,
              std::ptr::null_mut(),
            )
          };

          debug_assert!(res != DISP_CHANGE_BADFLAGS);
          debug_assert!(res != DISP_CHANGE_BADMODE);
          debug_assert!(res != DISP_CHANGE_BADPARAM);
          debug_assert!(res != DISP_CHANGE_FAILED);
          assert_eq!(res, DISP_CHANGE_SUCCESSFUL);
        }
        _ => (),
      }

      unsafe {
        // There are some scenarios where calling `ChangeDisplaySettingsExW` takes long
        // enough to execute that the DWM thinks our program has frozen and takes over
        // our program's window. When that happens, the `SetWindowPos` call below gets
        // eaten and the window doesn't get set to the proper fullscreen position.
        //
        // Calling `PeekMessageW` here notifies Windows that our process is still running
        // fine, taking control back from the DWM and ensuring that the `SetWindowPos` call
        // below goes through.
        let mut msg = MSG::default();
        PeekMessageW(&mut msg, HWND::default(), 0, 0, PM_NOREMOVE);
      }

      // Update window style
      WindowState::set_window_flags(window_state.lock(), window.0, |f| {
        f.set(
          WindowFlags::MARKER_EXCLUSIVE_FULLSCREEN,
          matches!(fullscreen, Some(Fullscreen::Exclusive(_))),
        );
        f.set(
          WindowFlags::MARKER_BORDERLESS_FULLSCREEN,
          matches!(fullscreen, Some(Fullscreen::Borderless(_))),
        );
      });

      // Update window bounds
      match &fullscreen {
        Some(fullscreen) => {
          // Save window bounds before entering fullscreen
          let placement = unsafe {
            let mut placement = WINDOWPLACEMENT::default();
            GetWindowPlacement(window.0, &mut placement);
            placement
          };

          window_state.lock().saved_window = Some(SavedWindow { placement });

          let monitor = match &fullscreen {
            Fullscreen::Exclusive(video_mode) => video_mode.monitor(),
            Fullscreen::Borderless(Some(monitor)) => monitor.clone(),
            Fullscreen::Borderless(None) => RootMonitorHandle {
              inner: monitor::current_monitor(window.0),
            },
          };

          let position: (i32, i32) = monitor.position().into();
          let size: (u32, u32) = monitor.size().into();

          unsafe {
            SetWindowPos(
              window.0,
              HWND::default(),
              position.0,
              position.1,
              size.0 as i32,
              size.1 as i32,
              SWP_ASYNCWINDOWPOS | SWP_NOZORDER,
            );
            InvalidateRgn(window.0, HRGN::default(), false);
          }
        }
        None => {
          let mut window_state_lock = window_state.lock();
          if let Some(SavedWindow { placement }) = window_state_lock.saved_window.take() {
            drop(window_state_lock);
            unsafe {
              SetWindowPlacement(window.0, &placement);
              InvalidateRgn(window.0, HRGN::default(), false);
            }
          }
        }
      }

      unsafe {
        taskbar_mark_fullscreen(window.0, fullscreen.is_some());
      }
    });
  }

  #[inline]
  pub fn set_decorations(&self, decorations: bool) {
    let window = self.window.clone();
    let window_state = Arc::clone(&self.window_state);

    self.thread_executor.execute_in_thread(move || {
      WindowState::set_window_flags(window_state.lock(), window.0, |f| {
        f.set(WindowFlags::DECORATIONS, decorations)
      });
    });
  }

  #[inline]
  pub fn set_always_on_top(&self, always_on_top: bool) {
    let window = self.window.clone();
    let window_state = Arc::clone(&self.window_state);

    self.thread_executor.execute_in_thread(move || {
      WindowState::set_window_flags(window_state.lock(), window.0, |f| {
        f.set(WindowFlags::ALWAYS_ON_TOP, always_on_top)
      });
    });
  }

  #[inline]
  pub fn current_monitor(&self) -> Option<RootMonitorHandle> {
    Some(RootMonitorHandle {
      inner: monitor::current_monitor(self.window.0),
    })
  }

  #[inline]
  pub fn set_window_icon(&self, window_icon: Option<Icon>) {
    if let Some(ref window_icon) = window_icon {
      window_icon
        .inner
        .set_for_window(self.window.0, IconType::Small);
    } else {
      icon::unset_for_window(self.window.0, IconType::Small);
    }
    self.window_state.lock().window_icon = window_icon;
  }

  #[inline]
  pub fn set_taskbar_icon(&self, taskbar_icon: Option<Icon>) {
    if let Some(ref taskbar_icon) = taskbar_icon {
      taskbar_icon
        .inner
        .set_for_window(self.window.0, IconType::Big);
    } else {
      icon::unset_for_window(self.window.0, IconType::Big);
    }
    self.window_state.lock().taskbar_icon = taskbar_icon;
  }

  pub(crate) fn set_ime_position_physical(&self, x: i32, y: i32) {
    if unsafe { GetSystemMetrics(SM_IMMENABLED) } != 0 {
      let composition_form = COMPOSITIONFORM {
        dwStyle: CFS_POINT,
        ptCurrentPos: POINT { x, y },
        rcArea: RECT::default(),
      };
      unsafe {
        let himc = ImmGetContext(self.window.0);
        ImmSetCompositionWindow(himc, &composition_form);
        ImmReleaseContext(self.window.0, himc);
      }
    }
  }

  #[inline]
  pub fn set_ime_position(&self, spot: Position) {
    let (x, y) = spot.to_physical::<i32>(self.scale_factor()).into();
    self.set_ime_position_physical(x, y);
  }

  #[inline]
  pub fn request_user_attention(&self, request_type: Option<UserAttentionType>) {
    let window = self.window.clone();
    let active_window_handle = unsafe { GetActiveWindow() };
    if window.0 == active_window_handle {
      return;
    }

    self.thread_executor.execute_in_thread(move || unsafe {
      let (flags, count) = request_type
        .map(|ty| match ty {
          UserAttentionType::Critical => (FLASHW_ALL | FLASHW_TIMERNOFG, u32::MAX),
          UserAttentionType::Informational => (FLASHW_TRAY | FLASHW_TIMERNOFG, 0),
        })
        .unwrap_or((FLASHW_STOP, 0));

      let flash_info = FLASHWINFO {
        cbSize: mem::size_of::<FLASHWINFO>() as u32,
        hwnd: window.0,
        dwFlags: flags,
        uCount: count,
        dwTimeout: 0,
      };
      FlashWindowEx(&flash_info);
    });
  }

  #[inline]
  pub fn theme(&self) -> Theme {
    self.window_state.lock().current_theme
  }

  #[inline]
  pub fn hide_menu(&self) {
    unsafe {
      SetMenu(self.hwnd(), HMENU::default());
    }
  }

  #[inline]
  pub fn show_menu(&self) {
    if let Some(menu) = &self.menu {
      unsafe {
        SetMenu(self.hwnd(), menu.0);
      }
    }
  }

  #[inline]
  pub fn is_menu_visible(&self) -> bool {
    unsafe { !GetMenu(self.hwnd()).is_invalid() }
  }

  #[inline]
  pub fn reset_dead_keys(&self) {
    // `ToUnicode` consumes the dead-key by default, so we are constructing a fake (but valid)
    // key input which we can call `ToUnicode` with.
    unsafe {
      let vk = u32::from(VK_SPACE);
      let scancode = MapVirtualKeyW(vk, MAPVK_VK_TO_VSC);
      let kbd_state = [0; 256];
      let mut char_buff = [MaybeUninit::uninit(); 8];
      ToUnicode(
        vk,
        scancode,
        kbd_state.as_ptr(),
        PWSTR(char_buff[0].as_mut_ptr()),
        char_buff.len() as i32,
        0,
      );
    }
  }

  #[inline]
  pub fn begin_resize_drag(&self, edge: isize, button: u32, x: i32, y: i32) {
    unsafe {
      let w_param = WPARAM(edge as _);
      let l_param = util::MAKELPARAM(x as i16, y as i16);

      ReleaseCapture();
      PostMessageW(self.hwnd(), button, w_param, l_param);
    }
  }

  #[inline]
  pub(crate) fn set_skip_taskbar(&self, skip: bool) {
    unsafe {
      com_initialized();
      let taskbar_list: ITaskbarList =
        CoCreateInstance(&TaskbarList, None, CLSCTX_SERVER).expect("failed to create TaskBarList");
      if skip {
        taskbar_list
          .DeleteTab(self.hwnd())
          .expect("DeleteTab failed");
      } else {
        taskbar_list.AddTab(self.hwnd()).expect("AddTab failed");
      }
    }
  }
}

impl Drop for Window {
  #[inline]
  fn drop(&mut self) {
    unsafe {
      // The window must be destroyed from the same thread that created it, so we send a
      // custom message to be handled by our callback to do the actual work.
      PostMessageW(self.window.0, *DESTROY_MSG_ID, WPARAM(0), LPARAM(0));
    }
  }
}

/// A simple non-owning wrapper around a window.
#[doc(hidden)]
#[derive(Clone)]
pub struct WindowWrapper(HWND);

// Send and Sync are not implemented for HWND and HDC, we have to wrap it and implement them manually.
// For more info see:
// https://github.com/retep998/winapi-rs/issues/360
// https://github.com/retep998/winapi-rs/issues/396
unsafe impl Sync for WindowWrapper {}
unsafe impl Send for WindowWrapper {}

unsafe fn init<T: 'static>(
  attributes: WindowAttributes,
  pl_attribs: PlatformSpecificWindowBuilderAttributes,
  event_loop: &EventLoopWindowTarget<T>,
) -> Result<Window, RootOsError> {
  // registering the window class
  let mut class_name = register_window_class(&attributes.window_icon, &pl_attribs.taskbar_icon);

  let mut window_flags = WindowFlags::empty();
  window_flags.set(WindowFlags::DECORATIONS, attributes.decorations);
  window_flags.set(WindowFlags::ALWAYS_ON_TOP, attributes.always_on_top);
  window_flags.set(
    WindowFlags::NO_BACK_BUFFER,
    pl_attribs.no_redirection_bitmap,
  );
  window_flags.set(WindowFlags::TRANSPARENT, attributes.transparent);
  // WindowFlags::VISIBLE and MAXIMIZED are set down below after the window has been configured.
  window_flags.set(WindowFlags::RESIZABLE, attributes.resizable);

  let parent = match pl_attribs.parent {
    Parent::ChildOf(parent) => {
      window_flags.set(WindowFlags::CHILD, true);
      if pl_attribs.menu.is_some() {
        warn!("Setting a menu on a child window is unsupported");
      }
      Some(parent)
    }
    Parent::OwnedBy(parent) => {
      window_flags.set(WindowFlags::POPUP, true);
      Some(parent)
    }
    Parent::None => {
      window_flags.set(WindowFlags::ON_TASKBAR, true);
      None
    }
  };

  // creating the real window this time, by using the functions in `extra_functions`
  let real_window = {
    let (style, ex_style) = window_flags.to_window_styles();
    let handle = CreateWindowExW(
      ex_style,
      PWSTR(class_name.as_mut_ptr()),
      attributes.title.as_str(),
      style,
      CW_USEDEFAULT,
      CW_USEDEFAULT,
      CW_USEDEFAULT,
      CW_USEDEFAULT,
      parent.unwrap_or_default(),
      pl_attribs.menu.unwrap_or_default(),
      GetModuleHandleW(PWSTR::default()),
      Box::into_raw(Box::new(window_flags)) as _,
    );

    if handle.is_invalid() {
      return Err(os_error!(OsError::IoError(io::Error::last_os_error())));
    }

    WindowWrapper(handle)
  };

  // Register for touch events if applicable
  {
    let digitizer = GetSystemMetrics(SM_DIGITIZER) as u32;
    if digitizer & NID_READY != 0 {
      RegisterTouchWindow(real_window.0, TWF_WANTPALM);
    }
  }

  let dpi = hwnd_dpi(real_window.0);
  let scale_factor = dpi_to_scale_factor(dpi);

  // making the window transparent
  if attributes.transparent && !pl_attribs.no_redirection_bitmap {
    // Empty region for the blur effect, so the window is fully transparent
    let region = CreateRectRgn(0, 0, -1, -1);

    let bb = DWM_BLURBEHIND {
      dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
      fEnable: true.into(),
      hRgnBlur: region,
      fTransitionOnMaximized: false.into(),
    };

    let _ = DwmEnableBlurBehindWindow(real_window.0, &bb);
    DeleteObject(region);
  }

  // If the system theme is dark, we need to set the window theme now
  // before we update the window flags (and possibly show the
  // window for the first time).
  let current_theme = try_theme(real_window.0, pl_attribs.preferred_theme);

  let window_state = {
    let window_state = WindowState::new(
      &attributes,
      pl_attribs.taskbar_icon,
      scale_factor,
      current_theme,
      pl_attribs.preferred_theme,
    );
    let window_state = Arc::new(Mutex::new(window_state));
    WindowState::set_window_flags(window_state.lock(), real_window.0, |f| *f = window_flags);
    window_state
  };

  let mut win = Window {
    window: real_window,
    window_state,
    thread_executor: event_loop.create_thread_executor(),
    menu: None,
  };

  win.set_skip_taskbar(pl_attribs.skip_taskbar);

  let dimensions = attributes
    .inner_size
    .unwrap_or_else(|| PhysicalSize::new(800, 600).into());
  win.set_inner_size(dimensions);
  if attributes.maximized {
    // Need to set MAXIMIZED after setting `inner_size` as
    // `Window::set_inner_size` changes MAXIMIZED to false.
    win.set_maximized(true);
  }
  win.set_visible(attributes.visible);

  if attributes.fullscreen.is_some() {
    win.set_fullscreen(attributes.fullscreen);
    force_window_active(win.window.0);
  }

  if let Some(position) = attributes.position {
    win.set_outer_position(position);
  }

  if let Some(window_menu) = attributes.window_menu {
    let event_loop_runner = event_loop.runner_shared.clone();
    let window_id = RootWindowId(win.id());
    let menu_handler = menu::MenuHandler::new(
      Box::new(move |event| {
        if let Ok(e) = event.map_nonuser_event() {
          event_loop_runner.send_event(e)
        }
      }),
      MenuType::MenuBar,
      Some(window_id),
    );

    win.menu = Some(HMenuWrapper(menu::initialize(
      window_menu,
      win.hwnd(),
      menu_handler,
    )));
  }

  Ok(win)
}

unsafe fn register_window_class(
  window_icon: &Option<Icon>,
  taskbar_icon: &Option<Icon>,
) -> Vec<u16> {
  let mut class_name = util::to_wstring("Window Class");

  let h_icon = taskbar_icon
    .as_ref()
    .map(|icon| icon.inner.as_raw_handle())
    .unwrap_or_default();
  let h_icon_small = window_icon
    .as_ref()
    .map(|icon| icon.inner.as_raw_handle())
    .unwrap_or_default();

  let class = WNDCLASSEXW {
    cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
    style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
    lpfnWndProc: Some(window_proc),
    cbClsExtra: 0,
    cbWndExtra: 0,
    hInstance: GetModuleHandleW(PWSTR::default()),
    hIcon: h_icon,
    hCursor: HCURSOR::default(), // must be null in order for cursor state to work properly
    hbrBackground: HBRUSH::default(),
    lpszMenuName: PWSTR::default(),
    lpszClassName: PWSTR(class_name.as_mut_ptr()),
    hIconSm: h_icon_small,
  };

  // We ignore errors because registering the same window class twice would trigger
  //  an error, and because errors here are detected during CreateWindowEx anyway.
  // Also since there is no weird element in the struct, there is no reason for this
  //  call to fail.
  RegisterClassExW(&class);

  class_name
}

unsafe extern "system" fn window_proc(
  window: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
) -> LRESULT {
  // This window procedure is only needed until the subclass procedure is attached.
  // we need this because we need to respond to WM_NCCALCSIZE as soon as possible
  // in order to make the window borderless if needed.
  match msg {
    win32wm::WM_NCCALCSIZE => {
      let userdata = util::GetWindowLongPtrW(window, GWL_USERDATA);
      if userdata != 0 {
        let win_flags = WindowFlags::from_bits_unchecked(userdata as _);
        if !win_flags.contains(WindowFlags::DECORATIONS) {
          // adjust the maximized borderless window so it doesn't cover the taskbar
          if util::is_maximized(window) {
            let monitor = monitor::current_monitor(window);
            if let Ok(monitor_info) = monitor::get_monitor_info(monitor.hmonitor()) {
              let params = &mut *(lparam.0 as *mut NCCALCSIZE_PARAMS);
              params.rgrc[0] = monitor_info.monitorInfo.rcWork;
            }
          }
          return LRESULT(0); // return 0 here to make the window borderless
        }
      }
      DefWindowProcW(window, msg, wparam, lparam)
    }
    win32wm::WM_NCCREATE => {
      let userdata = util::GetWindowLongPtrW(window, GWL_USERDATA);
      if userdata == 0 {
        let createstruct = &*(lparam.0 as *const CREATESTRUCTW);
        let userdata = createstruct.lpCreateParams;
        let window_flags = Box::from_raw(userdata as *mut WindowFlags);
        util::SetWindowLongPtrW(window, GWL_USERDATA, window_flags.bits() as _);
      }
      DefWindowProcW(window, msg, wparam, lparam)
    }
    _ => DefWindowProcW(window, msg, wparam, lparam),
  }
}

struct ComInitialized(Option<()>);
impl Drop for ComInitialized {
  fn drop(&mut self) {
    if let Some(()) = self.0.take() {
      unsafe { CoUninitialize() };
    }
  }
}

thread_local! {
    static COM_INITIALIZED: ComInitialized = {
        unsafe {
            ComInitialized(match CoInitializeEx(ptr::null_mut(), COINIT_APARTMENTTHREADED) {
              Ok(()) => Some(()),
              Err(_) => None,
            })
        }
    };

    static TASKBAR_LIST: RefCell<Option<ITaskbarList2>> = RefCell::new(None);
}

pub fn com_initialized() {
  COM_INITIALIZED.with(|_| {});
}

// Reference Implementation:
// https://github.com/chromium/chromium/blob/f18e79d901f56154f80eea1e2218544285e62623/ui/views/win/fullscreen_handler.cc
//
// As per MSDN marking the window as fullscreen should ensure that the
// taskbar is moved to the bottom of the Z-order when the fullscreen window
// is activated. If the window is not fullscreen, the Shell falls back to
// heuristics to determine how the window should be treated, which means
// that it could still consider the window as fullscreen. :(
unsafe fn taskbar_mark_fullscreen(handle: HWND, fullscreen: bool) {
  com_initialized();

  TASKBAR_LIST.with(|task_bar_list_ptr| {
    let mut task_bar_list = task_bar_list_ptr.borrow().clone();

    if task_bar_list.is_none() {
      let result: windows::core::Result<ITaskbarList2> =
        CoCreateInstance(&TaskbarList, None, CLSCTX_ALL);
      if let Ok(created) = result {
        if let Ok(()) = created.HrInit() {
          task_bar_list = Some(created);
        }
      }

      if task_bar_list.is_none() {
        return;
      }

      *task_bar_list_ptr.borrow_mut() = task_bar_list.clone();
    }

    let _ = task_bar_list
      .unwrap()
      .MarkFullscreenWindow(handle, fullscreen);
  })
}

unsafe fn force_window_active(handle: HWND) {
  // In some situation, calling SetForegroundWindow could not bring up the window,
  // This is a little hack which can "steal" the foreground window permission
  // We only call this function in the window creation, so it should be fine.
  // See : https://stackoverflow.com/questions/10740346/setforegroundwindow-only-working-while-visual-studio-is-open
  let alt_sc = MapVirtualKeyW(u32::from(VK_MENU), MAPVK_VK_TO_VSC);

  let mut inputs: [INPUT; 2] = mem::zeroed();
  inputs[0].r#type = INPUT_KEYBOARD;
  inputs[0].Anonymous.ki.wVk = VK_LMENU as _;
  inputs[0].Anonymous.ki.wScan = alt_sc as _;
  inputs[0].Anonymous.ki.dwFlags = KEYEVENTF_EXTENDEDKEY;

  inputs[1].r#type = INPUT_KEYBOARD;
  inputs[1].Anonymous.ki.wVk = VK_LMENU as _;
  inputs[1].Anonymous.ki.wScan = alt_sc as _;
  inputs[1].Anonymous.ki.dwFlags = KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP;

  // Simulate a key press and release
  SendInput(
    inputs.len() as _,
    inputs.as_mut_ptr(),
    mem::size_of::<INPUT>() as _,
  );

  SetForegroundWindow(handle);
}

pub fn hit_test(hwnd: HWND, cx: i32, cy: i32) -> LRESULT {
  let mut window_rect = RECT::default();
  unsafe {
    if GetWindowRect(hwnd, <*mut _>::cast(&mut window_rect)).as_bool() {
      const CLIENT: isize = 0b0000;
      const LEFT: isize = 0b0001;
      const RIGHT: isize = 0b0010;
      const TOP: isize = 0b0100;
      const BOTTOM: isize = 0b1000;
      const TOPLEFT: isize = TOP | LEFT;
      const TOPRIGHT: isize = TOP | RIGHT;
      const BOTTOMLEFT: isize = BOTTOM | LEFT;
      const BOTTOMRIGHT: isize = BOTTOM | RIGHT;

      let RECT {
        left,
        right,
        bottom,
        top,
      } = window_rect;

      #[rustfmt::skip]
      let result = (LEFT * (if cx < (left + BORDERLESS_RESIZE_INSET) { 1 } else { 0 }))
        | (RIGHT * (if cx >= (right - BORDERLESS_RESIZE_INSET) { 1 } else { 0 }))
        | (TOP * (if cy < (top + BORDERLESS_RESIZE_INSET) { 1 } else { 0 }))
        | (BOTTOM * (if cy >= (bottom - BORDERLESS_RESIZE_INSET) { 1 } else { 0 }));

      LRESULT(match result {
        CLIENT => HTCLIENT,
        LEFT => HTLEFT,
        RIGHT => HTRIGHT,
        TOP => HTTOP,
        BOTTOM => HTBOTTOM,
        TOPLEFT => HTTOPLEFT,
        TOPRIGHT => HTTOPRIGHT,
        BOTTOMLEFT => HTBOTTOMLEFT,
        BOTTOMRIGHT => HTBOTTOMRIGHT,
        _ => HTNOWHERE,
      } as _)
    } else {
      LRESULT(HTNOWHERE as _)
    }
  }
}
