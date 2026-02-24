use crate::app::{App, FocusState};
use crate::clusters::display_template;
use crate::ui::{centered_rect, render_scrollbar};
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

/// Draw the clusters overlay popup
pub fn draw_clusters(frame: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 70, frame.area());
    frame.render_widget(Clear, area);

    let (selected, scroll_offset) = match &mut app.focus {
        FocusState::Clusters {
            selected,
            scroll_offset,
        } => (selected, scroll_offset),
        _ => return,
    };

    let total = app.clusters.len();
    let title = if app.clusters_loading {
        " Clusters ".to_string()
    } else {
        format!(" Clusters ({total}) ")
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan))
        .title(title);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.clusters_loading {
        // Show loading placeholder
        let msg = "Detecting clusters...";
        let x = inner.x + inner.width.saturating_sub(msg.len() as u16) / 2;
        let y = inner.y + inner.height / 2;
        if y < inner.y + inner.height {
            let loading_area = Rect {
                x,
                y,
                width: msg.len() as u16,
                height: 1,
            };
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    msg,
                    Style::default().fg(Color::DarkGray),
                ))),
                loading_area,
            );
        }
        return;
    }

    if total == 0 || inner.height < 2 {
        return;
    }

    // Reserve 1 row for footer
    let list_height = (inner.height - 1) as usize;

    // Ensure selected is visible
    if *selected < *scroll_offset {
        *scroll_offset = *selected;
    }
    if *selected >= *scroll_offset + list_height {
        *scroll_offset = *selected - list_height + 1;
    }

    // Render cluster rows
    for (i, cluster) in app
        .clusters
        .iter()
        .enumerate()
        .skip(*scroll_offset)
        .take(list_height)
    {
        let y = (i - *scroll_offset) as u16;
        let is_selected = i == *selected;

        let display = display_template(&cluster.template);

        let (prefix, preview) = if cluster.sequence_len > 1 {
            // Sequence cluster: "2x [15 lines] first_template..."
            let prefix = format!("{:>4}x [{} lines] ", cluster.count, cluster.sequence_len);
            let max_w = (inner.width as usize).saturating_sub(prefix.len());
            let preview: String = display.chars().take(max_w).collect();
            (prefix, preview)
        } else {
            // Single-line cluster: "4x template..."
            let prefix = format!("{:>4}x ", cluster.count);
            let max_w = (inner.width as usize).saturating_sub(prefix.len());
            let preview: String = display.chars().take(max_w).collect();
            (prefix, preview)
        };

        let style = if is_selected {
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let seq_color = if cluster.sequence_len > 1 {
            Color::Magenta
        } else {
            Color::Yellow
        };

        let line = Line::from(vec![
            Span::styled(
                prefix,
                if is_selected {
                    style
                } else {
                    Style::default().fg(seq_color)
                },
            ),
            Span::styled(preview, style),
        ]);

        let row_area = Rect {
            x: inner.x,
            y: inner.y + y,
            width: inner.width,
            height: 1,
        };

        frame.render_widget(Paragraph::new(line).style(style), row_area);
    }

    // Footer help
    let footer_area = Rect {
        x: inner.x,
        y: inner.y + inner.height - 1,
        width: inner.width,
        height: 1,
    };
    let footer = Line::from(vec![Span::styled(
        " Enter: jump │ j/k: navigate │ Esc: close",
        Style::default().fg(Color::DarkGray),
    )]);
    frame.render_widget(Paragraph::new(footer), footer_area);

    // Scrollbar
    let scroll_area = Rect {
        height: inner.height - 1,
        ..inner
    };
    render_scrollbar(frame, scroll_area, *scroll_offset, total);
}
