use crate::{action::ActionBindingSystem, app::AppState};

#[derive(Debug, Default)]
pub struct LegendWidget {
    pub label_show: String,
    pub label_hide: String,
    pub hide: bool,
}

impl LegendWidget {
    pub fn render(
        &self,
        frame: &mut mame::terminal::UnicodeTerminalFrame,
        bindings: &ActionBindingSystem,
        state: &AppState,
    ) -> std::fmt::Result {
        let legend = mame::legend::Legend::new(self.title(), self.items(bindings, state));
        legend.render(frame)?;
        Ok(())
    }

    pub fn remaining_cols(
        &self,
        frame_size: tuinix::TerminalSize,
        bindings: &ActionBindingSystem,
        state: &AppState,
    ) -> usize {
        if self.hide {
            return frame_size.cols;
        }

        let legend_size = mame::legend::Legend::new(self.title(), self.items(bindings, state)).size();
        frame_size
            .cols
            .checked_sub(legend_size.cols)
            .unwrap_or(frame_size.cols)
    }

    fn title(&self) -> &str {
        if self.hide {
            &self.label_show
        } else {
            &self.label_hide
        }
    }

    fn items<'a>(
        &'a self,
        bindings: &'a ActionBindingSystem,
        _state: &'a AppState,
    ) -> impl 'a + Iterator<Item = String> {
        bindings
            .current_bindings()
            .iter()
            .filter(|_| !self.hide)
            // TODO: .filter(|b| b.action.as_ref().is_some_and(|a| a.is_applicable(tree)))
            .filter_map(|b| b.label.as_ref())
            .map(|s| {
                if s.starts_with(' ') {
                    s.to_owned()
                } else {
                    let style = tuinix::TerminalStyle::new().bold();
                    let reset = tuinix::TerminalStyle::RESET;
                    format!("{style}{s}{reset}")
                }
            })
    }
}
