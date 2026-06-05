use super::*;

#[tokio::test]
async fn idle_chatwidget_render_fills_full_background() {
    let (chat, _rx, _op_rx) = make_chatwidget_manual(/*model_override*/ None).await;
    let area = Rect::new(0, 0, 120, 36);
    let mut buf = ratatui::buffer::Buffer::empty(area);

    chat.render(area, &mut buf);

    for y in 0..area.height {
        for x in 0..area.width {
            assert!(
                buf[(x, y)].style().bg.is_some(),
                "expected background fill at ({x}, {y})"
            );
        }
    }
}
