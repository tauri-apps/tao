use winapi::shared::minwindef::{FALSE, UINT};
use winapi::shared::ntdef::{CHAR, HANDLE, LPWSTR, WCHAR};
use winapi::shared::winerror::ERROR_SUCCESS;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::winbase::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
use winapi::um::winuser::{
    CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
    GetClipboardFormatNameA, IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatA,
    SetClipboardData, CF_UNICODETEXT,
};
use crate::clipboard::ClipboardFormat;

#[derive(Debug, Clone, Default)]
pub struct Clipboard;

impl Clipboard {
   pub fn put_string(&mut self, s: impl AsRef<str>) {
       let s = s.as_ref();
       let format: ClipboardFormat = s.into();
       self.put_formats(&[format])
   }

   pub fn get_string(&self) -> Option<String> {
       with_clipboard(|| unsafe {
           let handle = GetClipboardData(CF_UNICODETEXT);
           if handle.is_null() {
               None
           } else {
               let unic_str = GlobalLock(handle) as LPWSTR;
               let mut len = 0;
               while *unic_str.offset(len) != 0 {
                     len += 1;
               }
               let utf16_slice = slice::from_raw_parts(*unic_str, len as usize);
               let result = String::from_utf16(utf16_slice);
               GlobalUnlock(handle);
               result
           }
       })
       .flatten()
   }

   pub fn put_formats(&mut self, formats: &[ClipboardFormat]) {
      with_clipboard(|| unsafe {
          EmptyClipboard();

          for format in formats {
              let handle = make_handle(&format);
              let format_id = match get_format_id(&format.identifier) {
                  Some(id) => id,
                  None => {
                      tracing::warn!(
                          "failed to register clipboard format {}",
                          &format.identifier
                      );
                      continue;
                  }
              };
              let result = SetClipboardData(format_id, handle);
              if result.is_null() {
                  tracing::warn!(
                      "failed to set clipboard for fmt {}, error: {}",
                      &format.identifier,
                      GetLastError()
                  );
              }
          }
      });
  }

}

fn get_format_id(format: FormatId) -> Option<UINT> {
   if let Some((id, _)) = STANDARD_FORMATS.iter().find(|(_, s)| s == &format) {
       return Some(*id);
   }
   match format {
       ClipboardFormat::TEXT => Some(CF_UNICODETEXT),
       other => register_identifier(other),
   }
}


unsafe fn make_handle(format: &ClipboardFormat) -> HANDLE {
   if format.identifier == ClipboardFormat::TEXT {
       let s = std::str::from_utf8_unchecked(&format.data);
       let wstr = s.to_wide();
       let handle = GlobalAlloc(GMEM_MOVEABLE, wstr.len() * mem::size_of::<WCHAR>());
       let locked = GlobalLock(handle) as LPWSTR;
       ptr::copy_nonoverlapping(wstr.as_ptr(), locked, wstr.len());
       GlobalUnlock(handle);
       handle
   } else {
       let handle = GlobalAlloc(GMEM_MOVEABLE, format.data.len() * mem::size_of::<CHAR>());
       let locked = GlobalLock(handle) as *mut u8;
       ptr::copy_nonoverlapping(format.data.as_ptr(), locked, format.data.len());
       GlobalUnlock(handle);
       handle
   }
}

fn with_clipboard<V>(f: impl FnOnce() -> V) -> Option<V> {
   unsafe {
       if OpenClipboard(ptr::null_mut()) == FALSE {
           return None;
       }

       let result = f();

       CloseClipboard();

       Some(result)
   }
}

// https://docs.microsoft.com/en-ca/windows/win32/dataxchg/standard-clipboard-formats
static STANDARD_FORMATS: &[(UINT, &str)] = &[
    (1, "CF_TEXT"),
    (2, "CF_BITMAP"),
    (3, "CF_METAFILEPICT"),
    (4, "CF_SYLK"),
    (5, "CF_DIF"),
    (6, "CF_TIFF"),
    (7, "CF_OEMTEXT"),
    (8, "CF_DIB"),
    (9, "CF_PALETTE"),
    (10, "CF_PENDATA"),
    (11, "CF_RIFF"),
    (12, "CF_WAVE"),
    (13, "CF_UNICODETEXT"),
    (14, "CF_ENHMETAFILE"),
    (15, "CF_HDROP"),
    (16, "CF_LOCALE"),
    (17, "CF_DIBV5"),
    (0x0080, "CF_OWNERDISPLAY"),
    (0x0081, "CF_DSPTEXT"),
    (0x0082, "CF_DSPBITMAP"),
    (0x0083, "CF_DSPMETAFILEPICT"),
    (0x008E, "CF_DSPENHMETAFILE"),
    (0x0200, "CF_PRIVATEFIRST"),
    (0x02FF, "CF_PRIVATELAST"),
    (0x0300, "CF_GDIOBJFIRST"),
    (0x03FF, "CF_GDIOBJLAST"),
];