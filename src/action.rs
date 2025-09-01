use crate::app::Focus;

pub type Config = mame::action::ActionConfig<Action>;

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
    FlipExtendedRegexp,
    FlipFixedStrings,
    FlipPerlRegexp,
    FlipContextBefore, // TODO: rename
    FlipContext,       // TODO: rename
    ClearArg,
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

impl Action {}

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
            "flip-extended-regexp" => Ok(Self::FlipExtendedRegexp),
            "flip-fixed-strings" => Ok(Self::FlipFixedStrings),
            "flip-perl-regexp" => Ok(Self::FlipPerlRegexp),
            "flip-context-before" => Ok(Self::FlipContextBefore),
            "flip-context" => Ok(Self::FlipContext),
            "clear-arg" => Ok(Self::ClearArg),
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
