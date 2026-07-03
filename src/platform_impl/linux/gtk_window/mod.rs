mod imp;

use std::{rc::Rc, sync::atomic::AtomicI32};

use gtk::{
  gio,
  glib::{
    self, object::ObjectExt, subclass::types::ObjectSubclassIsExt, Object, RustClosure,
    SignalHandlerId,
  },
};

use crate::window::WindowAttributes;

use super::{Parent, PlatformSpecificWindowBuilderAttributes};

// Libadwaita support - conditional Application type
#[cfg(feature = "libadwaita")]
use libadwaita as adw;

#[cfg(feature = "libadwaita")]
type AppType = adw::Application;
#[cfg(not(feature = "libadwaita"))]
type AppType = gtk::Application;

#[cfg(feature = "libadwaita")]
glib::wrapper! {
    pub struct ApplicationWindow(ObjectSubclass<imp::ApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[cfg(not(feature = "libadwaita"))]
glib::wrapper! {
    pub struct ApplicationWindow(ObjectSubclass<imp::ApplicationWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

impl ApplicationWindow {
  pub fn new(
    app: &AppType,
    attributes: &WindowAttributes,
    pl_attribs: &PlatformSpecificWindowBuilderAttributes,
  ) -> Self {
    // Create new window
    let mut window_builder = Object::builder()
      .property("application", app)
      .property("title", &attributes.title)
      .property("deletable", attributes.closable)
      .property("decorated", attributes.decorations);

    if let Parent::ChildOf(parent) = &pl_attribs.parent {
      window_builder = window_builder.property("transient-for", parent);
    }

    window_builder.build()
  }

  pub fn inner_size(&self) -> &Rc<(AtomicI32, AtomicI32)> {
    &self.imp().inner_size
  }
  pub fn outer_size(&self) -> &Rc<(AtomicI32, AtomicI32)> {
    &self.imp().outer_size
  }

  pub fn connect_resized(&self, f: RustClosure) -> SignalHandlerId {
    self.connect_closure("resized", false, f)
  }
}
