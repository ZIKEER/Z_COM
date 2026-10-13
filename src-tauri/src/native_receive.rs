use crate::native_terminal::Line;
use eframe::egui::{self, Color32, Sense};
use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Position {
    line: u64,
    character: usize,
}

const LINE_HEIGHT: f32 = 18.0;

#[derive(Default)]
pub(crate) struct ReceiveView {
    first_line: u64,
    /// 每行自动换行后的高度，首项对应 `heights_first` 行
    heights: VecDeque<f32>,
    heights_first: u64,
    layout_key: Option<(i32, bool)>,
    anchor: Option<Position>,
    cursor: Option<Position>,
    dragging: bool,
    focused: bool,
}

impl ReceiveView {
    pub fn trimmed(&mut self) {
        self.first_line += 1;
    }
    pub fn clear(&mut self) {
        *self = Self::default();
    }
    pub fn select_all(&mut self, lines: &VecDeque<Line>) {
        if let Some(last) = lines.back() {
            self.anchor = Some(Position {
                line: self.first_line,
                character: 0,
            });
            self.cursor = Some(Position {
                line: self.first_line + lines.len() as u64 - 1,
                character: last.plain().chars().count(),
            });
        }
    }
    pub fn selected(&self, lines: &VecDeque<Line>) -> String {
        let Some((start, end)) = self.range() else {
            return String::new();
        };
        let mut output = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            let line_number = self.first_line + index as u64;
            if line_number < start.line || line_number > end.line {
                continue;
            }
            let plain = line.plain();
            let begin = if line_number == start.line {
                start.character
            } else {
                0
            };
            let finish = if line_number == end.line {
                end.character
            } else {
                plain.chars().count()
            };
            output.push(
                plain
                    .chars()
                    .skip(begin)
                    .take(finish.saturating_sub(begin))
                    .collect::<String>(),
            );
        }
        output.join("\n")
    }
    fn range(&self) -> Option<(Position, Position)> {
        let anchor = self.anchor?;
        let cursor = self.cursor?;
        Some((anchor.min(cursor), anchor.max(cursor)))
    }

    fn galley(
        ui: &egui::Ui,
        line: &Line,
        ansi: bool,
        width: f32,
    ) -> std::sync::Arc<egui::Galley> {
        let mut job = crate::native_terminal::layout(line, ansi);
        job.wrap.max_width = width;
        ui.fonts(|fonts| fonts.layout_job(job))
    }

    /// 同步每行换行后的高度缓存；宽度或 ANSI 开关变化时整体重算
    fn sync_heights(&mut self, ui: &egui::Ui, lines: &VecDeque<Line>, width: f32, ansi: bool) {
        let key = (width.round() as i32, ansi);
        if self.layout_key != Some(key) {
            self.layout_key = Some(key);
            self.heights.clear();
        }
        while self.heights_first < self.first_line && !self.heights.is_empty() {
            self.heights.pop_front();
            self.heights_first += 1;
        }
        if self.heights.is_empty() {
            self.heights_first = self.first_line;
        }
        self.heights.truncate(lines.len());
        for line in lines.iter().skip(self.heights.len()) {
            let height = Self::galley(ui, line, ansi, width).size().y;
            self.heights.push_back(height.max(LINE_HEIGHT));
        }
    }

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        lines: &VecDeque<Line>,
        ansi: &mut bool,
        auto_scroll: bool,
    ) -> bool {
        let context = ui.ctx().clone();
        let mut clear = false;
        let viewport = ui.available_rect_before_wrap();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .stick_to_bottom(auto_scroll && !self.dragging)
            .show_viewport(ui, |ui, visible| {
                let width = ui.available_width().max(40.0);
                self.sync_heights(ui, lines, width, *ansi);
                let origin = ui.max_rect().min;
                let total: f32 = self.heights.iter().sum();
                ui.set_min_size(egui::vec2(width, total));
                let pointer = context.input(|input| input.pointer.interact_pos());
                let mut top = 0.0;
                let heights: Vec<f32> = self.heights.iter().copied().collect();
                for (index, height) in heights.into_iter().enumerate() {
                    let bottom = top + height;
                    if bottom < visible.top() {
                        top = bottom;
                        continue;
                    }
                    if top > visible.bottom() {
                        break;
                    }
                    let galley = Self::galley(ui, &lines[index], *ansi, width);
                    let rect = egui::Rect::from_min_size(
                        origin + egui::vec2(0.0, top),
                        egui::vec2(width, height),
                    );
                    top = bottom;
                    let line_number = self.first_line + index as u64;
                    let response =
                        ui.interact(rect, ui.id().with(line_number), Sense::click_and_drag());
                    let position_at = |pointer: egui::Pos2| Position {
                        line: line_number,
                        character: galley.cursor_from_pos(pointer - rect.min).ccursor.index,
                    };
                    if response.drag_started() || response.clicked() {
                        if let Some(pointer) = pointer {
                            let position = position_at(pointer);
                            if !context.input(|input| input.modifiers.shift)
                                || self.anchor.is_none()
                            {
                                self.anchor = Some(position);
                            }
                            self.cursor = Some(position);
                            self.dragging = response.drag_started();
                            self.focused = true;
                        }
                    }
                    if self.dragging && context.input(|input| input.pointer.primary_down()) {
                        if let Some(pointer) = pointer {
                            if pointer.y >= rect.top() && pointer.y < rect.bottom() {
                                self.cursor = Some(position_at(pointer));
                            }
                        }
                    }
                    if let Some((start, end)) = self.range() {
                        if line_number >= start.line && line_number <= end.line {
                            let begin = if line_number == start.line {
                                start.character
                            } else {
                                0
                            };
                            let finish = if line_number == end.line {
                                end.character
                            } else {
                                usize::MAX
                            };
                            // 换行后按每一显示行分别绘制选区
                            let mut row_start = 0;
                            for row in &galley.rows {
                                let row_end = row_start + row.char_count_excluding_newline();
                                let from = begin.max(row_start);
                                let to = finish.min(row_end);
                                if from < to {
                                    let left = row.x_offset(from - row_start);
                                    let right = row.x_offset(to - row_start);
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_max(
                                            rect.min + egui::vec2(left, row.min_y()),
                                            rect.min + egui::vec2(right, row.max_y()),
                                        ),
                                        0.0,
                                        Color32::from_rgb(190, 220, 255),
                                    );
                                }
                                row_start += row.char_count_including_newline();
                            }
                        }
                    }
                    ui.painter()
                        .galley(rect.min, galley.clone(), Color32::from_gray(32));
                    response.context_menu(|ui| {
                        if ui.button("复制选区").clicked() {
                            context.copy_text(self.selected(lines));
                            ui.close_menu();
                        }
                        if ui.button("全选").clicked() {
                            self.select_all(lines);
                            ui.close_menu();
                        }
                        if ui.button("复制全部").clicked() {
                            context.copy_text(
                                lines.iter().map(Line::plain).collect::<Vec<_>>().join("\n"),
                            );
                            ui.close_menu();
                        }
                        ui.checkbox(ansi, "ANSI 颜色显示");
                        if ui.button("清空并重置计数").clicked() {
                            clear = true;
                            ui.close_menu();
                        }
                    });
                }
                if self.dragging {
                    if let Some(pointer) = pointer {
                        if pointer.y < viewport.top() {
                            ui.scroll_with_delta(egui::vec2(0.0, LINE_HEIGHT));
                        }
                        if pointer.y > viewport.bottom() {
                            ui.scroll_with_delta(egui::vec2(0.0, -LINE_HEIGHT));
                        }
                    }
                }
            });
        if !context.input(|input| input.pointer.primary_down()) {
            self.dragging = false;
        }
        if context.input(|input| {
            input.pointer.any_pressed()
                && input
                    .pointer
                    .interact_pos()
                    .is_some_and(|position| !viewport.contains(position))
        }) {
            self.focused = false;
        }
        if self.focused {
            if context.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::A))
            {
                self.select_all(lines);
            }
            let copy = context.input(|input| {
                input
                    .events
                    .iter()
                    .any(|event| matches!(event, egui::Event::Copy))
            });
            if copy {
                context.copy_text(self.selected(lines));
            }
        }
        clear
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_survives_scrolling_and_trimming_and_uses_unicode_positions() {
        let mut view = ReceiveView::default();
        let mut lines =
            VecDeque::from([Line::data("abc"), Line::data("中文def"), Line::data("last")]);
        view.anchor = Some(Position {
            line: 0,
            character: 1,
        });
        view.cursor = Some(Position {
            line: 1,
            character: 2,
        });
        assert_eq!(view.selected(&lines), "bc\n中文");
        lines.pop_front();
        view.trimmed();
        assert_eq!(view.selected(&lines), "中文");
        view.select_all(&lines);
        assert_eq!(view.selected(&lines), "中文def\nlast");
    }
}
