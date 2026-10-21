//! Captura de seleção via UI Automation (sem tocar no clipboard e sem foco).

use dictos_selection::{CapturedSelection, ScreenRect, SelectionError, SelectionProvider};
use uiautomation::patterns::UITextPattern;
use uiautomation::UIAutomation;

/// Captura o texto selecionado do elemento com foco via UIA TextPattern,
/// com o retângulo da seleção (âncora) quando o aplicativo o expõe.
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
            let anchor = bounding_rectangles(range.as_ref());
            return Ok(CapturedSelection { text, anchor });
        }
        Err(SelectionError::NoSelection)
    }
}

/// Retângulo da seleção via `IUIAutomationTextRange::GetBoundingRectangles`
/// (o crate `uiautomation` não expõe esse método, mas usa o mesmo crate
/// `windows` que nós — `as_ref()` dá acesso direto à interface COM).
///
/// Retorna `None` quando o aplicativo não informa retângulos ou quando
/// todos são degenerados (seleção rolada para fora da área visível); nesse
/// caso o popup cai no posicionamento junto ao cursor.
#[cfg(windows)]
fn bounding_rectangles(
    range: &windows::Win32::UI::Accessibility::IUIAutomationTextRange,
) -> Option<ScreenRect> {
    use windows::Win32::System::Ole::{
        SafeArrayAccessData, SafeArrayDestroy, SafeArrayGetLBound, SafeArrayGetUBound,
        SafeArrayUnaccessData,
    };

    unsafe {
        let psa = range.GetBoundingRectangles().ok()?;
        if psa.is_null() {
            return None;
        }
        let parsed = (|| {
            let lower = SafeArrayGetLBound(psa, 1).ok()?;
            let upper = SafeArrayGetUBound(psa, 1).ok()?;
            let count = (upper - lower + 1).max(0) as usize;
            if count < 4 {
                return None;
            }
            let mut data = std::ptr::null_mut();
            SafeArrayAccessData(psa, &mut data).ok()?;
            let values = std::slice::from_raw_parts(data as *const f64, count);
            let rect = parse_bounding_rects(values);
            let _ = SafeArrayUnaccessData(psa);
            rect
        })();
        let _ = SafeArrayDestroy(psa);
        parsed
    }
}

/// Interpreta o vetor de `f64` do SAFEARRAY (grupos de 4: left, top, width,
/// height) e devolve o primeiro retângulo não degenerado.
///
/// Função pura: testável sem COM.
fn parse_bounding_rects(values: &[f64]) -> Option<ScreenRect> {
    for chunk in values.chunks_exact(4) {
        let [left, top, width, height] = [chunk[0], chunk[1], chunk[2], chunk[3]];
        if width > 0.0 && height > 0.0 {
            return Some(ScreenRect {
                x: left.round() as i32,
                y: top.round() as i32,
                width: width.round() as i32,
                height: height.round() as i32,
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_primeiro_retangulo_valido() {
        let values = [100.4, 200.6, 54.2, 18.9];
        assert_eq!(
            parse_bounding_rects(&values),
            Some(ScreenRect {
                x: 100,
                y: 201,
                width: 54,
                height: 19
            })
        );
    }

    #[test]
    fn parse_pula_retangulos_degenerados() {
        let values = [0.0, 0.0, 0.0, 0.0, 10.0, 20.0, 30.0, 40.0];
        assert_eq!(
            parse_bounding_rects(&values),
            Some(ScreenRect {
                x: 10,
                y: 20,
                width: 30,
                height: 40
            })
        );
    }

    #[test]
    fn parse_vazio_ou_incompleto() {
        assert_eq!(parse_bounding_rects(&[]), None);
        assert_eq!(parse_bounding_rects(&[1.0, 2.0, 3.0]), None);
        assert_eq!(parse_bounding_rects(&[0.0, 0.0, 0.0, 0.0]), None);
    }
}
