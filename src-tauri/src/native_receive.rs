use eframe::egui::{self, Color32, Sense};
use std::collections::VecDeque;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Position {
    line: u64,
    character: usize,
}

#[derive(Default)]
pub(crate) struct ReceiveView {
    first_line: u64,
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
    pub fn select_all(&mut self, lines: &VecDeque<String>) {
        if let Some(last) = lines.back() {
            self.anchor = Some(Position {
                line: self.first_line,
                character: 0,
            });
            self.cursor = Some(Position {
                line: self.first_line + lines.len() as u64 - 1,
                character: crate::native_terminal::plain(last).chars().count(),
            });
        }
    }
    pub fn selected(&self, lines: &VecDeque<String>) -> String {
        let Some((start, end)) = self.range() else {
            return String::new();
        };
        let mut output = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            let line_number = self.first_line + index as u64;
            if line_number < start.line || line_number > end.line {
                continue;
            }
            let plain = crate::native_terminal::plain(line);
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

    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        lines: &VecDeque<String>,
        ansi: &mut bool,
        auto_scroll: bool,
    ) -> bool {
        let context = ui.ctx().clone();
        let mut clear = false;
        let viewport = ui.clip_rect();
        egui::ScrollArea::both()
            .stick_to_bottom(auto_scroll && !self.dragging)
            .show_rows(ui, 18.0, lines.len(), |ui, range| {
                for index in range {
                    let job = crate::native_terminal::layout(&lines[index], *ansi);
                    let galley = ui.fonts(|fonts| fonts.layout_job(job));
                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(galley.size().x.max(ui.available_width()), 18.0),
                        Sense::click_and_drag(),
                    );
                    let line_number = self.first_line + index as u64;
                    let pointer = context.input(|input| input.pointer.interact_pos());
                    if response.drag_started() || response.clicked() {
                        if let Some(pointer) = pointer {
                            let position = Position {
                                line: line_number,
                                character: galley.cursor_from_pos(pointer - rect.min).ccursor.index,
                            };
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
                                self.cursor = Some(Position {
                                    line: line_number,
                                    character: galley
                                        .cursor_from_pos(pointer - rect.min)
                                        .ccursor
                                        .index,
                                });
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
                                galley.text().chars().count()
                            };
                            let begin_x = galley
                                .pos_from_ccursor(egui::text::CCursor::new(begin))
                                .left();
                            let finish_x = galley
                                .pos_from_ccursor(egui::text::CCursor::new(finish))
                                .left();
                            ui.painter().rect_filled(
                                egui::Rect::from_min_max(
                                    rect.min + egui::vec2(begin_x, 0.0),
                                    rect.min + egui::vec2(finish_x.max(begin_x), 18.0),
                                ),
                                0.0,
                                Color32::from_rgb(190, 220, 255),
                            );
                        }
                    }
                    ui.painter()
                        .galley(rect.min, galley, Color32::from_gray(32));
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
                                lines
                                    .iter()
                                    .map(|line| crate::native_terminal::plain(line))
                                    .collect::<Vec<_>>()
                                    .join("\n"),
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
                    if let Some(pointer) = context.input(|input| input.pointer.interact_pos()) {
                        if pointer.y < viewport.top() {
                            ui.scroll_with_delta(egui::vec2(0.0, 18.0));
                        }
                        if pointer.y > viewport.bottom() {
                            ui.scroll_with_delta(egui::vec2(0.0, -18.0));
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
        let mut lines = VecDeque::from(["abc".into(), "中文def".into(), "last".into()]);
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
