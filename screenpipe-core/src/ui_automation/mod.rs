//! Desktop UI automation through accessibility APIs
//!
//! This module provides a cross-platform API for automating desktop applications
//! through accessibility APIs, inspired by Playwright's web automation model.

mod actions;
mod element;
mod errors;
mod locator;
mod platforms;
mod selector;

pub use actions::{click, press_key, scroll, type_text};
pub use element::{UIElement, UIElementAttributes};
pub use errors::AutomationError;
pub use locator::Locator;
pub use selector::{Selector, SelectorEngine};

/// The main entry point for UI automation
pub struct Desktop {
    engine: Box<dyn platforms::AccessibilityEngine>,
}

impl Desktop {
    /// Create a new instance with the default platform-specific implementation
    pub fn new() -> Result<Self, AutomationError> {
        Ok(Self {
            engine: platforms::create_engine()?,
        })
    }

    /// Get the root UI element representing the entire desktop
    pub fn root(&self) -> UIElement {
        self.engine.get_root_element()
    }

    /// Create a locator to find elements matching the given selector
    pub fn locator(&self, selector: impl Into<Selector>) -> Locator {
        Locator::new(self.engine.as_ref(), selector.into())
    }

    /// Get an element by its accessibility ID
    pub fn element_by_id(&self, id: &str) -> Result<UIElement, AutomationError> {
        self.engine.get_element_by_id(id)
    }

    /// Get the currently focused element
    pub fn focused_element(&self) -> Result<UIElement, AutomationError> {
        self.engine.get_focused_element()
    }

    /// List all running applications
    pub fn applications(&self) -> Result<Vec<UIElement>, AutomationError> {
        self.engine.get_applications()
    }

    /// Find an application by name
    pub fn application(&self, name: &str) -> Result<UIElement, AutomationError> {
        self.engine.get_application_by_name(name)
    }
}
