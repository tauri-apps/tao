use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::WindowsAndMessaging::*;

fn main() {
  unsafe {
    let class = WNDCLASSEXW {
      cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
      style: CS_HREDRAW | CS_VREDRAW | CS_OWNDC,
      lpfnWndProc: Some(window_proc),
      cbClsExtra: 0,
      cbWndExtra: 0,
      hInstance: GetModuleHandleW(PWSTR::default()),
      hCursor: HCURSOR::default(), // must be null in order for cursor state to work properly
      hbrBackground: HBRUSH::default(),
      lpszMenuName: PWSTR::default(),
      lpszClassName: PWSTR("class_name".as_ptr() as _),
      ..Default::default()
    };

    RegisterClassExW(&class);

    let handle = CreateWindowExW(
      WS_EX_ACCEPTFILES | WS_EX_WINDOWEDGE | WS_EX_APPWINDOW,
      PWSTR("class_name".as_ptr() as _),
      "window",
      WS_CLIPSIBLINGS
        | WS_CLIPCHILDREN
        | WS_SYSMENU
        | WS_CAPTION
        | WS_MINIMIZEBOX
        | WS_THICKFRAME
        | WS_MAXIMIZEBOX
        | WS_BORDER
        | WS_VISIBLE,
      CW_USEDEFAULT,
      CW_USEDEFAULT,
      CW_USEDEFAULT,
      CW_USEDEFAULT,
      HWND::default(),
      HMENU::default(),
      GetModuleHandleW(PWSTR::default()),
      0 as _,
    );
    let region = CreateRectRgn(0, 0, -1, -1);

    let bb = DWM_BLURBEHIND {
      dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION,
      fEnable: true.into(),
      hRgnBlur: region,
      fTransitionOnMaximized: false.into(),
    };

    let _ = DwmEnableBlurBehindWindow(handle, &bb);
    DeleteObject(region);

    let mut msg = MSG::default();

    'main: loop {
      if !GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
        break 'main;
      }

      TranslateMessage(&msg);
      DispatchMessageW(&msg);
    }
  }
}

unsafe extern "system" fn window_proc(
  window: HWND,
  msg: u32,
  wparam: WPARAM,
  lparam: LPARAM,
) -> LRESULT {
  DefWindowProcW(window, msg, wparam, lparam)
}
