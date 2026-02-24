#[derive(Debug, Clone)]
pub struct Command {
    pub name: &'static str,
    pub shortcut: &'static str,
    pub action: CommandAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandAction {
    OpenFile,
    MergeFile,
    Search,
    DateFilter,
    ToggleError,
    ToggleWarn,
    ToggleInfo,
    ToggleDebug,
    ToggleTrace,
    ToggleProfile,
    ToggleTail,
    ToggleWrap,
    ToggleSyntax,
    GoToTop,
    GoToBottom,
    NextError,
    PrevError,
    NextWarning,
    PrevWarning,
    ToggleBookmark,
    NextBookmark,
    PrevBookmark,
    ClearBookmarks,
    ExcludeManager,
    ClearExcludes,
    ExportFiltered,
    Clusters,
    Quit,
}

impl Command {
    pub const ALL: &'static [Command] = &[
        Command {
            name: "Open file...",
            shortcut: "Ctrl+O",
            action: CommandAction::OpenFile,
        },
        Command {
            name: "Merge file...",
            shortcut: "M",
            action: CommandAction::MergeFile,
        },
        Command {
            name: "Search logs...",
            shortcut: "/",
            action: CommandAction::Search,
        },
        Command {
            name: "Filter by date range...",
            shortcut: "Ctrl+D",
            action: CommandAction::DateFilter,
        },
        Command {
            name: "Toggle Error",
            shortcut: "1",
            action: CommandAction::ToggleError,
        },
        Command {
            name: "Toggle Warning",
            shortcut: "2",
            action: CommandAction::ToggleWarn,
        },
        Command {
            name: "Toggle Info",
            shortcut: "3",
            action: CommandAction::ToggleInfo,
        },
        Command {
            name: "Toggle Debug",
            shortcut: "4",
            action: CommandAction::ToggleDebug,
        },
        Command {
            name: "Toggle Trace",
            shortcut: "5",
            action: CommandAction::ToggleTrace,
        },
        Command {
            name: "Toggle Profile",
            shortcut: "6",
            action: CommandAction::ToggleProfile,
        },
        Command {
            name: "Toggle tail mode",
            shortcut: "t",
            action: CommandAction::ToggleTail,
        },
        Command {
            name: "Toggle word wrap",
            shortcut: "Ctrl+W",
            action: CommandAction::ToggleWrap,
        },
        Command {
            name: "Toggle syntax highlighting",
            shortcut: "s",
            action: CommandAction::ToggleSyntax,
        },
        Command {
            name: "Go to top",
            shortcut: "g",
            action: CommandAction::GoToTop,
        },
        Command {
            name: "Go to bottom",
            shortcut: "G",
            action: CommandAction::GoToBottom,
        },
        Command {
            name: "Next error",
            shortcut: "e",
            action: CommandAction::NextError,
        },
        Command {
            name: "Previous error",
            shortcut: "E",
            action: CommandAction::PrevError,
        },
        Command {
            name: "Next warning",
            shortcut: "w",
            action: CommandAction::NextWarning,
        },
        Command {
            name: "Previous warning",
            shortcut: "W",
            action: CommandAction::PrevWarning,
        },
        Command {
            name: "Toggle bookmark",
            shortcut: "m",
            action: CommandAction::ToggleBookmark,
        },
        Command {
            name: "Next bookmark",
            shortcut: "b",
            action: CommandAction::NextBookmark,
        },
        Command {
            name: "Previous bookmark",
            shortcut: "B",
            action: CommandAction::PrevBookmark,
        },
        Command {
            name: "Clear all bookmarks",
            shortcut: "",
            action: CommandAction::ClearBookmarks,
        },
        Command {
            name: "Exclude filters...",
            shortcut: "x",
            action: CommandAction::ExcludeManager,
        },
        Command {
            name: "Clear exclude filters",
            shortcut: "X",
            action: CommandAction::ClearExcludes,
        },
        Command {
            name: "Export filtered results...",
            shortcut: "Ctrl+S",
            action: CommandAction::ExportFiltered,
        },
        Command {
            name: "Detect clusters",
            shortcut: "",
            action: CommandAction::Clusters,
        },
        Command {
            name: "Quit",
            shortcut: "q/Esc",
            action: CommandAction::Quit,
        },
    ];
}
