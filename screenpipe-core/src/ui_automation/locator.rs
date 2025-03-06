use crate::ui_automation::platforms::AccessibilityEngine;
use crate::ui_automation::{AutomationError, Selector, UIElement};
use std::sync::Arc;
use std::time::Duration;

use super::UIElementAttributes;

/// A high-level API for finding and interacting with UI elements
pub struct Locator {
    engine: Arc<dyn AccessibilityEngine>,
    selector: Selector,
    timeout: Duration,
    root: Option<UIElement>,
}

impl Locator {
    /// Create a new locator with the given selector
    pub(crate) fn new(engine: Arc<dyn AccessibilityEngine>, selector: Selector) -> Self {
        Self {
            engine,
            selector,
            timeout: Duration::from_secs(30),
            root: None,
        }
    }

    /// Set timeout for waiting operations
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the root element for this locator
    pub fn within(mut self, element: UIElement) -> Self {
        self.root = Some(element);
        self
    }

    /// Get the first element matching this locator
    pub fn first(&self) -> Result<Option<UIElement>, AutomationError> {
        let elements = self
            .engine
            .find_elements(&self.selector, self.root.as_ref())?;
        Ok(elements.into_iter().next())
    }

    /// Get all elements matching this locator
    pub fn all(&self) -> Result<Vec<UIElement>, AutomationError> {
        self.engine
            .find_elements(&self.selector, self.root.as_ref())
    }

    /// Wait for an element to be available
    pub async fn wait(&self) -> Result<UIElement, AutomationError> {
        let start = std::time::Instant::now();

        while start.elapsed() < self.timeout {
            if let Some(element) = self.first()? {
                return Ok(element);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Err(AutomationError::Timeout(format!(
            "Timed out waiting for selector: {:?}",
            self.selector
        )))
    }

    /// Get a nested locator
    pub fn locator(&self, selector: impl Into<Selector>) -> Locator {
        let selector = selector.into();
        Locator {
            engine: self.engine.clone(),
            selector: Selector::Chain(vec![self.selector.clone(), selector]),
            timeout: self.timeout,
            root: self.root.clone(),
        }
    }

    /// Filter the current locator
    pub fn filter<F>(&self, predicate: F) -> Locator
    where
        F: Fn(&UIElementAttributes) -> bool + Send + Sync + 'static,
    {
        let filter_id =
            super::selector::FilterPredicate::register(self.selector.clone(), Box::new(predicate));

        Locator {
            engine: self.engine.clone(),
            selector: Selector::Filter(filter_id),
            timeout: self.timeout,
            root: self.root.clone(),
        }
    }

    // Convenience methods for common actions

    /// Click on the first matching element
    pub async fn click(&self) -> Result<(), AutomationError> {
        self.wait().await?.click()
    }

    /// Type text into the first matching element
    pub async fn type_text(&self, text: &str) -> Result<(), AutomationError> {
        self.wait().await?.type_text(text)
    }

    /// Press a key on the first matching element
    pub async fn press_key(&self, key: &str) -> Result<(), AutomationError> {
        self.wait().await?.press_key(key)
    }

    /// Get text from the first matching element
    pub async fn text(&self) -> Result<String, AutomationError> {
        self.wait().await?.text()
    }
}
