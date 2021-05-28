use cocoa::{
  appkit::{
    NSImageNameStatusAvailable, NSImageNameStatusPartiallyAvailable, NSImageNameStatusUnavailable,
  },
  base::id,
};

use crate::menu::MenuIcon;

impl MenuIcon {
  /// # Safety
  // todo: add more icons (but should be supported on windows and linux too?)
  // complete list available here:
  // https://developer.apple.com/documentation/appkit/nsimagename?language=objc
  pub unsafe fn get_ns_image(self) -> id {
    match self {
      MenuIcon::StatusAvailable => NSImageNameStatusAvailable,
      MenuIcon::StatusUnavailable => NSImageNameStatusUnavailable,
      MenuIcon::StatusPartiallyAvailable => NSImageNameStatusPartiallyAvailable,
    }
  }
}
