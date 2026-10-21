//! Captura de seleção via UI Automation (sem tocar no clipboard e sem foco).

use dictos_selection::{CapturedSelection, SelectionError, SelectionProvider};
use uiautomation::patterns::UITextPattern;
use uiautomation::UIAutomation;

/// Captura o texto selecionado do elemento com foco via UIA TextPattern.
pub struct UiaSelectionProvider;

impl SelectionProvider for UiaSelectionProvider {
    fn capture(&self) -> Result<CapturedSelection, SelectionError> {
        let automation = UIAutomation::new()
            .map_err(|e| SelectionError::CaptureFailed(format!("UIA indisponível: {e}")))?;
        let focused = automation
            .get_focused_element()
            .map_err(|_| SelectionError::NoSelection)?;
        let pattern: UITextPattern = focused
            .get_pattern()
            .map_err(|_| SelectionError::NoSelection)?;
        let ranges = pattern
            .get_selection()
            .map_err(|_| SelectionError::NoSelection)?;

        for range in &ranges {
            let text = range
                .get_text(-1)
                .map_err(|_| SelectionError::NoSelection)?;
            if text.trim().is_empty() {
                continue;
            }
            // A crate `uiautomation` não expõe IUIAutomationTextRange::
            // GetBoundingRectangles; a âncora precisa (retângulo da seleção)
            // entra na Etapa 4 via bindings Win32 diretos. No MVP o popup se
            // posiciona junto ao cursor do mouse.
            return Ok(CapturedSelection { text, anchor: None });
        }
        Err(SelectionError::NoSelection)
    }
}
