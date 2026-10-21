//! Posicionamento do popup junto à seleção (lógica pura, testável).

use dictos_selection::ScreenRect;

/// Margem entre o popup e a seleção/borda da work area, em pixels físicos.
const MARGIN: i32 = 8;

/// Onde posicionar o popup (pixels físicos), dado o retângulo da seleção,
/// o tamanho do popup e a work area do monitor.
///
/// Preferência: logo abaixo da seleção. Se não couber, acima. Se não couber
/// nem abaixo nem acima, fica abaixo mesmo, clampado à work area.
pub fn place_popup(
    anchor: ScreenRect,
    popup_size: (i32, i32),
    work_area: ScreenRect,
) -> (i32, i32) {
    let (pw, ph) = popup_size;
    let below_y = anchor.y + anchor.height + MARGIN;
    let above_y = anchor.y - ph - MARGIN;

    let fits_below = below_y + ph <= work_area.y + work_area.height;
    let y = if fits_below {
        below_y
    } else if above_y >= work_area.y {
        above_y
    } else {
        below_y
    };

    (
        clamp(anchor.x, work_area.x, work_area.x + work_area.width - pw),
        clamp(y, work_area.y, work_area.y + work_area.height - ph),
    )
}

fn clamp(value: i32, min: i32, max: i32) -> i32 {
    value.max(min).min(max.max(min))
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: ScreenRect = ScreenRect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1040, // 1080 menos barra de tarefas
    };
    const SIZE: (i32, i32) = (380, 200);

    #[test]
    fn preferencia_abaixo_da_selecao() {
        let anchor = ScreenRect { x: 100, y: 300, width: 80, height: 20 };
        assert_eq!(place_popup(anchor, SIZE, WORK), (100, 328));
    }

    #[test]
    fn vai_para_cima_quando_nao_cabe_embaixo() {
        let anchor = ScreenRect { x: 100, y: 1000, width: 80, height: 20 };
        // abaixo: 1028 + 200 > 1040 → acima: 1000 - 200 - 8 = 792
        assert_eq!(place_popup(anchor, SIZE, WORK), (100, 792));
    }

    #[test]
    fn clamp_horizontal_na_borda_direita() {
        let anchor = ScreenRect { x: 1900, y: 300, width: 80, height: 20 };
        // 1900 > 1920 - 380 = 1540 → clamp para 1540
        assert_eq!(place_popup(anchor, SIZE, WORK), (1540, 328));
    }

    #[test]
    fn selecao_no_topo_sem_espaco_em_cima_nem_embaixo() {
        let anchor = ScreenRect { x: 100, y: 100, width: 80, height: 20 };
        let popup_alto = (380, 1200); // maior que a work area
        // não cabe em cima (100-1200-8 < 0) nem embaixo → abaixo, clampado a 0
        assert_eq!(place_popup(anchor, popup_alto, WORK), (100, 0));
    }
}
