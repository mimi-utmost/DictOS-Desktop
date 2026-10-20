//! Popup de consulta do DictOS.
//!
//! O modelo de view ([`LookupView`]) e o trait [`Popup`] são independentes de
//! toolkit; a implementação atual usa Slint (licença royalty-free 2.0, com a
//! atribuição "Built with Slint" exibida no rodapé do popup).

use dictos_core::LookupResult;

pub use slint;

slint::include_modules!();

/// Uma acepção formatada para exibição.
#[derive(Debug, Clone)]
pub struct SenseView {
    pub pos: String,
    pub definition: String,
    pub examples: Vec<String>,
}

/// Resultado de consulta pronto para exibição, independente de toolkit.
#[derive(Debug, Clone)]
pub struct LookupView {
    pub word: String,
    pub diacritic_fallback: bool,
    pub senses: Vec<SenseView>,
    pub translations: Vec<(String, String)>,
}

impl From<&LookupResult> for LookupView {
    fn from(result: &LookupResult) -> Self {
        let entry = &result.entry;
        LookupView {
            word: entry.word.clone(),
            diacritic_fallback: result.diacritic_fallback,
            senses: entry
                .senses
                .iter()
                .map(|s| SenseView {
                    pos: s.pos.clone(),
                    definition: s.definition.clone(),
                    examples: s.examples.clone(),
                })
                .collect(),
            translations: entry
                .translations
                .iter()
                .map(|t| (t.lang.clone(), t.word.clone()))
                .collect(),
        }
    }
}

/// Interface de popup: cada plataforma tem sua implementação.
pub trait Popup {
    /// Exibe o resultado de uma consulta.
    fn show_lookup(&self, view: &LookupView);
    /// Exibe uma mensagem (palavra desconhecida, falha de captura etc.).
    fn show_message(&self, title: &str, message: &str);
    /// Esconde o popup.
    fn hide(&self);
}

/// Formata o corpo textual do popup a partir do view model.
pub fn format_body(view: &LookupView) -> String {
    let mut body = String::new();
    for (i, sense) in view.senses.iter().enumerate() {
        body.push_str(&format!("{}. ({})\n{}\n", i + 1, sense.pos, sense.definition));
        for example in &sense.examples {
            body.push_str(&format!("   • {example}\n"));
        }
        body.push('\n');
    }
    if !view.translations.is_empty() {
        body.push_str("Traduções:\n");
        for (lang, word) in &view.translations {
            body.push_str(&format!("  {lang}: {word}\n"));
        }
    }
    body.trim_end().to_string()
}

/// Implementação Slint do popup.
pub struct SlintPopup {
    ui: DictosPopup,
    _click_timer: slint::Timer,
}

impl SlintPopup {
    /// Cria o popup oculto. Deve ser chamado na thread do event loop Slint
    /// (a thread principal).
    pub fn new() -> Result<Self, slint::PlatformError> {
        let ui = DictosPopup::new()?;

        let weak = ui.as_weak();
        ui.on_close(move || {
            if let Some(ui) = weak.upgrade() {
                let _ = ui.window().hide();
            }
        });

        // Esconde o popup quando o usuário clica fora dele. O popup pode não
        // receber foco (o Windows impede processos em segundo plano de tomar
        // o foreground), então a detecção é por clique global: polling do
        // botão esquerdo + posição do cursor fora do retângulo da janela.
        let click_timer = slint::Timer::default();
        let weak = ui.as_weak();
        click_timer.start(
            slint::TimerMode::Repeated,
            std::time::Duration::from_millis(100),
            move || {
                let Some(ui) = weak.upgrade() else { return };
                if ui.window().is_visible() && clicked_outside(&ui) {
                    let _ = ui.window().hide();
                }
            },
        );

        Ok(Self {
            ui,
            _click_timer: click_timer,
        })
    }

    /// Handle fraco para uso a partir de outras threads via
    /// `slint::invoke_from_event_loop`.
    pub fn as_weak(&self) -> slint::Weak<DictosPopup> {
        self.ui.as_weak()
    }
}

impl Popup for SlintPopup {
    fn show_lookup(&self, view: &LookupView) {
        show_lookup_ui(&self.ui, view);
    }

    fn show_message(&self, title: &str, message: &str) {
        show_message_ui(&self.ui, title, message);
    }

    fn hide(&self) {
        let _ = self.ui.window().hide();
    }
}

/// Exibe um resultado de consulta num componente já criado (usável de
/// qualquer lugar com acesso ao componente, ex.: dentro de
/// `slint::invoke_from_event_loop`).
pub fn show_lookup_ui(ui: &DictosPopup, view: &LookupView) {
    let note = if view.diacritic_fallback {
        "encontrado por busca sem acentos"
    } else {
        ""
    };
    present(ui, &view.word, format_body(view), note);
}

/// Exibe uma mensagem (palavra desconhecida, falha de captura etc.).
pub fn show_message_ui(ui: &DictosPopup, title: &str, message: &str) {
    present(ui, title, message.to_string(), "");
}

/// Preenche o conteúdo, posiciona junto ao cursor e mostra.
fn present(ui: &DictosPopup, word: &str, body: String, note: &str) {
    ui.set_word(word.into());
    ui.set_body(body.into());
    ui.set_note(note.into());

    let (x, y) = popup_position(ui.window().size());
    ui.window().set_position(slint::PhysicalPosition::new(x, y));
    let _ = ui.window().show();
}

/// Posição do popup: abaixo e à direita do cursor, limitado à tela primária.
#[cfg(windows)]
fn popup_position(window_size: slint::PhysicalSize) -> (i32, i32) {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    let mut point = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut point);
        let screen_w = GetSystemMetrics(SM_CXSCREEN);
        let screen_h = GetSystemMetrics(SM_CYSCREEN);
        let x = (point.x + 12).min(screen_w - window_size.width as i32 - 8).max(0);
        let y = (point.y + 16).min(screen_h - window_size.height as i32 - 8).max(0);
        (x, y)
    }
}

#[cfg(not(windows))]
fn popup_position(_window_size: slint::PhysicalSize) -> (i32, i32) {
    (100, 100)
}

/// Verdadeiro quando houve um clique esquerdo fora do retângulo do popup
/// (detecção de borda: só na transição solto → pressionado).
#[cfg(windows)]
fn clicked_outside(ui: &DictosPopup) -> bool {
    use std::sync::atomic::{AtomicBool, Ordering};
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    static WAS_DOWN: AtomicBool = AtomicBool::new(false);

    unsafe {
        let down = GetAsyncKeyState(VK_LBUTTON.0 as i32) < 0;
        let was_down = WAS_DOWN.swap(down, Ordering::Relaxed);
        if !down || was_down {
            return false;
        }

        let mut point = POINT::default();
        if GetCursorPos(&mut point).is_err() {
            return false;
        }
        let pos = ui.window().position();
        let size = ui.window().size();
        let inside = point.x >= pos.x
            && point.x < pos.x + size.width as i32
            && point.y >= pos.y
            && point.y < pos.y + size.height as i32;
        !inside
    }
}

#[cfg(not(windows))]
fn clicked_outside(_ui: &DictosPopup) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_body_com_senses_e_traducoes() {
        let view = LookupView {
            word: "apple".into(),
            diacritic_fallback: false,
            senses: vec![SenseView {
                pos: "noun".into(),
                definition: "A round fruit.".into(),
                examples: vec!["She ate an apple.".into()],
            }],
            translations: vec![("pt".into(), "maçã".into())],
        };
        let body = format_body(&view);
        assert!(body.contains("1. (noun)"));
        assert!(body.contains("A round fruit."));
        assert!(body.contains("• She ate an apple."));
        assert!(body.contains("pt: maçã"));
    }

    #[test]
    fn lookup_view_from_result() {
        use dictos_core::{DictionaryEntry, Sense, Translation};
        let result = LookupResult {
            entry: DictionaryEntry {
                word: "run".into(),
                lang: "en".into(),
                senses: vec![Sense {
                    pos: "verb".into(),
                    definition: "To move swiftly.".into(),
                    examples: vec![],
                }],
                translations: vec![Translation {
                    lang: "pt".into(),
                    word: "correr".into(),
                }],
            },
            diacritic_fallback: false,
        };
        let view = LookupView::from(&result);
        assert_eq!(view.word, "run");
        assert_eq!(view.senses.len(), 1);
        assert_eq!(view.translations, vec![("pt".to_string(), "correr".to_string())]);
    }
}
