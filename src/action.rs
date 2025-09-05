use crate::app::{AppState, Focus};

#[derive(Debug, Clone)]
pub enum Action {
    Quit,
    ToggleLegend,
    InitLegend {
        hide: bool,
        label_show: String,
        label_hide: String,
    },
    SetFocus(Focus),
    CursorUp,
    CursorDown,
    CursorLeft,
    CursorRight,
    ToggleExpansion,
    ToggleAllExpansion,
    FlipCaseSensitive,
    FlipWholeWord,
    FlipIgnoreCase,
    FlipUntracked,
    FlipNoIndex,
    FlipNoRecursive,
    FlipExtendedRegexp,
    FlipFixedStrings,
    FlipPerlRegexp,
    DecreaseContext,
    IncreaseContext,
    DeleteChar,
    DeleteBackward,
    InsertChar,
    MoveToStart,
    MoveToEnd,
    MoveForward,
    MoveBackward,
    DeleteToEnd,
    AcceptInput,
}

impl Action {
    pub fn is_flag_set(&self, state: &AppState) -> bool {
        match self {
            // Git grep flags that can be toggled
            Action::FlipIgnoreCase => state.grep.ignore_case,
            Action::FlipUntracked => state.grep.untracked,
            Action::FlipNoIndex => state.grep.no_index,
            Action::FlipNoRecursive => state.grep.no_recursive,
            Action::FlipWholeWord => state.grep.word_regexp,
            Action::FlipFixedStrings => state.grep.fixed_strings,
            Action::FlipExtendedRegexp => state.grep.extended_regexp,
            Action::FlipPerlRegexp => state.grep.perl_regexp,

            // All other actions don't represent toggleable flags
            _ => false,
        }
    }

    pub fn is_applicable(&self, state: &AppState) -> bool {
        match self {
            // Always applicable actions
            Action::Quit
            | Action::ToggleLegend
            | Action::InitLegend { .. }
            | Action::SetFocus(_)
            | Action::FlipIgnoreCase
            | Action::FlipUntracked
            | Action::FlipNoIndex
            | Action::FlipNoRecursive
            | Action::FlipWholeWord => true,

            // Actions that depend on current focus
            Action::AcceptInput
            | Action::InsertChar
            | Action::DeleteBackward
            | Action::DeleteChar
            | Action::DeleteToEnd
            | Action::MoveToStart
            | Action::MoveToEnd
            | Action::MoveForward
            | Action::MoveBackward => state.focus.is_editing(),

            // Navigation actions that depend on search results
            Action::CursorUp => state.can_cursor_up(),
            Action::CursorDown => state.can_cursor_down(),
            Action::CursorLeft => state.cursor.is_line_level(),
            Action::CursorRight => state.cursor.is_file_level(),

            // Toggle actions that depend on cursor position
            Action::ToggleExpansion => state.cursor.is_file_level(),
            Action::ToggleAllExpansion => !state.search_result.is_empty(),

            // Context actions that depend on cursor being at line level
            Action::IncreaseContext => {
                state.cursor.is_line_level()
                    && state.grep.context_lines < crate::git::ContextLines::MAX
            }
            Action::DecreaseContext => {
                state.cursor.is_line_level()
                    && state.grep.context_lines > crate::git::ContextLines::MIN
            }

            // Regex flag actions with mutual exclusions
            Action::FlipFixedStrings => !(state.grep.perl_regexp || state.grep.extended_regexp),
            Action::FlipExtendedRegexp => !(state.grep.fixed_strings || state.grep.perl_regexp),
            Action::FlipPerlRegexp => !(state.grep.fixed_strings || state.grep.extended_regexp),

            // Deprecated/unused actions
            Action::FlipCaseSensitive => false,
        }
    }
}

impl mame::action::Action for Action {}

impl<'text, 'raw> TryFrom<nojson::RawJsonValue<'text, 'raw>> for Action {
    type Error = nojson::JsonParseError;

    fn try_from(value: nojson::RawJsonValue<'text, 'raw>) -> Result<Self, Self::Error> {
        let ty = value.to_member("type")?.required()?;

        match ty.to_unquoted_string_str()?.as_ref() {
            "quit" => Ok(Self::Quit),
            "toggle-legend" => Ok(Self::ToggleLegend),
            "init-legend" => {
                let hide = value
                    .to_member("hide")?
                    .map(bool::try_from)?
                    .unwrap_or_default();
                let labels = value.to_member("labels")?.required()?;
                let label_show = labels.to_member("show")?.required()?.try_into()?;
                let label_hide = labels.to_member("hide")?.required()?.try_into()?;
                Ok(Self::InitLegend {
                    hide,
                    label_show,
                    label_hide,
                })
            }
            "set-focus" => {
                let focus_str = value.to_member("focus")?.required()?;
                let focus = match focus_str.to_unquoted_string_str()?.as_ref() {
                    "search-result" => Focus::SearchResult,
                    "pattern" => Focus::Pattern,
                    "and-pattern" => Focus::AndPattern,
                    "not-pattern" => Focus::NotPattern,
                    "revision" => Focus::Revision,
                    "path" => Focus::Path,
                    _ => return Err(focus_str.invalid("unknown focus")),
                };
                Ok(Self::SetFocus(focus))
            }
            "cursor-up" => Ok(Self::CursorUp),
            "cursor-down" => Ok(Self::CursorDown),
            "cursor-left" => Ok(Self::CursorLeft),
            "cursor-right" => Ok(Self::CursorRight),
            "toggle-expansion" => Ok(Self::ToggleExpansion),
            "toggle-all-expansion" => Ok(Self::ToggleAllExpansion),
            "flip-case-sensitive" => Ok(Self::FlipCaseSensitive),
            "flip-whole-word" => Ok(Self::FlipWholeWord),
            "flip-ignore-case" => Ok(Self::FlipIgnoreCase),
            "flip-untracked" => Ok(Self::FlipUntracked),
            "flip-no-index" => Ok(Self::FlipNoIndex),
            "flip-no-recursive" => Ok(Self::FlipNoRecursive),
            "flip-extended-regexp" => Ok(Self::FlipExtendedRegexp),
            "flip-fixed-strings" => Ok(Self::FlipFixedStrings),
            "flip-perl-regexp" => Ok(Self::FlipPerlRegexp),
            "decrease-context" => Ok(Self::DecreaseContext),
            "increase-context" => Ok(Self::IncreaseContext),
            "delete-char" => Ok(Self::DeleteChar),
            "delete-backward" => Ok(Self::DeleteBackward),
            "insert-char" => Ok(Self::InsertChar),
            "move-to-start" => Ok(Self::MoveToStart),
            "move-to-end" => Ok(Self::MoveToEnd),
            "move-forward" => Ok(Self::MoveForward),
            "move-backward" => Ok(Self::MoveBackward),
            "delete-to-end" => Ok(Self::DeleteToEnd),
            "accept-input" => Ok(Self::AcceptInput),
            type_name => Err(ty.invalid(format!("unknown action type: {type_name:?}"))),
        }
    }
}
