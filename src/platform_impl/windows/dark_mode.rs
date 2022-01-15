// Copyright 2019-2021 Tauri Programme within The Commons Conservancy
// SPDX-License-Identifier: Apache-2.0

/// This is a simple implementation of support for Windows Dark Mode,
/// which is inspired by the solution in https://github.com/ysc3839/win32-darkmode
use windows::Win32::{
  Foundation::{BOOL, HWND, PSTR, PWSTR},
  System::LibraryLoader::*,
  UI::{Accessibility::*, Controls::*, WindowsAndMessaging::*},
};

use std::ffi::c_void;

use crate::{platform_impl::platform::util, window::Theme};

lazy_static! {
    static ref WIN10_BUILD_VERSION: Option<u32> = {
        // FIXME: RtlGetVersion is a documented windows API,
        // should be part of win32metadata!

        #[allow(non_snake_case)]
        #[repr(C)]
        struct OSVERSIONINFOW {
            dwOSVersionInfoSize: u32,
            dwMajorVersion: u32,
            dwMinorVersion: u32,
            dwBuildNumber: u32,
            dwPlatformId: u32,
            szCSDVersion: [u16; 128],
        }

        type RtlGetVersion = unsafe extern "system" fn (*mut OSVERSIONINFOW) -> i32;
        let handle = get_function!("ntdll.dll", RtlGetVersion);

        if let Some(rtl_get_version) = handle {
            unsafe {
                let mut vi = OSVERSIONINFOW {
                    dwOSVersionInfoSize: 0,
                    dwMajorVersion: 0,
                    dwMinorVersion: 0,
                    dwBuildNumber: 0,
                    dwPlatformId: 0,
                    szCSDVersion: [0; 128],
                };

                let status = (rtl_get_version)(&mut vi as _);

                if status >= 0 && vi.dwMajorVersion == 10 && vi.dwMinorVersion == 0 {
                    Some(vi.dwBuildNumber)
                } else {
                    None
                }
            }
        } else {
            None
        }
    };

    static ref DARK_MODE_SUPPORTED: bool = {
        // We won't try to do anything for windows versions < 17763
        // (Windows 10 October 2018 update)
        match *WIN10_BUILD_VERSION {
            Some(v) => v >= 17763,
            None => false
        }
    };

    static ref DARK_THEME_NAME: Vec<u16> = util::to_wstring("DarkMode_Explorer");
    static ref LIGHT_THEME_NAME: Vec<u16> = util::to_wstring("");
}

/// Attempt to set a theme on a window, if necessary.
/// Returns the theme that was picked
pub fn try_theme(hwnd: HWND, preferred_theme: Option<Theme>) -> Theme {
  if *DARK_MODE_SUPPORTED {
    let is_dark_mode = match preferred_theme {
      Some(theme) => theme == Theme::Dark,
      None => should_use_dark_mode(),
    };

    let theme = if is_dark_mode {
      Theme::Dark
    } else {
      Theme::Light
    };
    let theme_name = PWSTR(
      match theme {
        Theme::Dark => DARK_THEME_NAME.clone(),
        Theme::Light => LIGHT_THEME_NAME.clone(),
      }
      .as_mut_ptr(),
    );

    let status = unsafe { SetWindowTheme(hwnd, theme_name, PWSTR::default()) };

    if status.is_ok() && set_dark_mode_for_window(hwnd, is_dark_mode) {
      return theme;
    }
  }

  Theme::Light
}

fn set_dark_mode_for_window(hwnd: HWND, is_dark_mode: bool) -> bool {
  // Uses Windows undocumented API SetWindowCompositionAttribute,
  // as seen in win32-darkmode example linked at top of file.

  type SetWindowCompositionAttribute =
    unsafe extern "system" fn(HWND, *mut WINDOWCOMPOSITIONATTRIBDATA) -> BOOL;

  #[allow(non_snake_case)]
  type WINDOWCOMPOSITIONATTRIB = u32;
  const WCA_USEDARKMODECOLORS: WINDOWCOMPOSITIONATTRIB = 26;

  #[allow(non_snake_case)]
  #[repr(C)]
  struct WINDOWCOMPOSITIONATTRIBDATA {
    Attrib: WINDOWCOMPOSITIONATTRIB,
    pvData: *mut c_void,
    cbData: usize,
  }

  lazy_static! {
    static ref SET_WINDOW_COMPOSITION_ATTRIBUTE: Option<SetWindowCompositionAttribute> =
      get_function!("user32.dll", SetWindowCompositionAttribute);
  }

  if let Some(set_window_composition_attribute) = *SET_WINDOW_COMPOSITION_ATTRIBUTE {
    unsafe {
      // SetWindowCompositionAttribute needs a bigbool (i32), not bool.
      let mut is_dark_mode_bigbool: BOOL = is_dark_mode.into();

      let mut data = WINDOWCOMPOSITIONATTRIBDATA {
        Attrib: WCA_USEDARKMODECOLORS,
        pvData: &mut is_dark_mode_bigbool as *mut _ as _,
        cbData: std::mem::size_of_val(&is_dark_mode_bigbool) as _,
      };

      let status = set_window_composition_attribute(hwnd, &mut data as *mut _);

      status.as_bool()
    }
  } else {
    false
  }
}

fn should_use_dark_mode() -> bool {
  should_apps_use_dark_mode() && !is_high_contrast()
}

fn should_apps_use_dark_mode() -> bool {
  type ShouldAppsUseDarkMode = unsafe extern "system" fn() -> bool;
  lazy_static! {
    static ref SHOULD_APPS_USE_DARK_MODE: Option<ShouldAppsUseDarkMode> = {
      unsafe {
        const UXTHEME_SHOULDAPPSUSEDARKMODE_ORDINAL: u16 = 132;

        let module = LoadLibraryA("uxtheme.dll");

        if module.is_invalid() {
          return None;
        }

        let handle = GetProcAddress(
          module,
          PSTR(UXTHEME_SHOULDAPPSUSEDARKMODE_ORDINAL as usize as *mut _),
        );

        handle.map(|handle| std::mem::transmute(handle))
      }
    };
  }

  SHOULD_APPS_USE_DARK_MODE
    .map(|should_apps_use_dark_mode| unsafe { (should_apps_use_dark_mode)() })
    .unwrap_or(false)
}

const HCF_HIGHCONTRASTON: u32 = 1;

fn is_high_contrast() -> bool {
  let mut hc = HIGHCONTRASTA {
    cbSize: 0,
    dwFlags: 0,
    lpszDefaultScheme: PSTR::default(),
  };

  let ok = unsafe {
    SystemParametersInfoA(
      SPI_GETHIGHCONTRAST,
      std::mem::size_of_val(&hc) as _,
      &mut hc as *mut _ as _,
      0,
    )
  };

  ok.as_bool() && (HCF_HIGHCONTRASTON & hc.dwFlags) != 0
}
