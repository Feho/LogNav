#[derive(Debug, Clone)]
pub struct Command {
    pub name: &'static str,
    pub shortcut: &'static str,
    pub action: CommandAction,
    pub group: &'static str,
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
    VisualSelect,
    ExcludeManager,
    ClearExcludes,
    IncludeManager,
    ClearIncludes,
    ExportFiltered,
    Clusters,
    ThemePicker,
    Stats,
    Quit,
}

impl Command {
    pub const ALL: &'static [Command] = &[
        // ── Analysis ──
        Command {
            name: "Find repeating patterns",
            shortcut: "",
            action: CommandAction::Clusters,
            group: "Analysis",
        },
        Command {
            name: "Statistics dashboard",
            shortcut: "F2",
            action: CommandAction::Stats,
            group: "Analysis",
        },
        // ── Search & Filter ──
        Command {
            name: "Search logs...",
            shortcut: "/",
            action: CommandAction::Search,
            group: "Search & Filter",
        },
        Command {
            name: "Filter by date range...",
            shortcut: "Ctrl+D",
            action: CommandAction::DateFilter,
            group: "Search & Filter",
        },
        Command {
            name: "Exclude filters...",
            shortcut: "x",
            action: CommandAction::ExcludeManager,
            group: "Search & Filter",
        },
        Command {
            name: "Include filters...",
            shortcut: "i",
            action: CommandAction::IncludeManager,
            group: "Search & Filter",
        },
        Command {
            name: "Clear exclude filters",
            shortcut: "X",
            action: CommandAction::ClearExcludes,
            group: "Search & Filter",
        },
        Command {
            name: "Clear include filters",
            shortcut: "I",
            action: CommandAction::ClearIncludes,
            group: "Search & Filter",
        },
        Command {
            name: "Toggle Error",
            shortcut: "1",
            action: CommandAction::ToggleError,
            group: "Search & Filter",
        },
        Command {
            name: "Toggle Warning",
            shortcut: "2",
            action: CommandAction::ToggleWarn,
            group: "Search & Filter",
        },
        Command {
            name: "Toggle Info",
            shortcut: "3",
            action: CommandAction::ToggleInfo,
            group: "Search & Filter",
        },
        Command {
            name: "Toggle Debug",
            shortcut: "4",
            action: CommandAction::ToggleDebug,
            group: "Search & Filter",
        },
        Command {
            name: "Toggle Trace",
            shortcut: "5",
            action: CommandAction::ToggleTrace,
            group: "Search & Filter",
        },
        Command {
            name: "Toggle Profile",
            shortcut: "6",
            action: CommandAction::ToggleProfile,
            group: "Search & Filter",
        },
        // ── Navigation ──
        Command {
            name: "Go to top",
            shortcut: "g",
            action: CommandAction::GoToTop,
            group: "Navigation",
        },
        Command {
            name: "Go to bottom",
            shortcut: "G",
            action: CommandAction::GoToBottom,
            group: "Navigation",
        },
        Command {
            name: "Next error",
            shortcut: "e",
            action: CommandAction::NextError,
            group: "Navigation",
        },
        Command {
            name: "Previous error",
            shortcut: "E",
            action: CommandAction::PrevError,
            group: "Navigation",
        },
        Command {
            name: "Next warning",
            shortcut: "w",
            action: CommandAction::NextWarning,
            group: "Navigation",
        },
        Command {
            name: "Previous warning",
            shortcut: "W",
            action: CommandAction::PrevWarning,
            group: "Navigation",
        },
        Command {
            name: "Visual select mode",
            shortcut: "v",
            action: CommandAction::VisualSelect,
            group: "Navigation",
        },
        // ── Bookmarks ──
        Command {
            name: "Toggle bookmark",
            shortcut: "m",
            action: CommandAction::ToggleBookmark,
            group: "Bookmarks",
        },
        Command {
            name: "Next bookmark",
            shortcut: "b",
            action: CommandAction::NextBookmark,
            group: "Bookmarks",
        },
        Command {
            name: "Previous bookmark",
            shortcut: "B",
            action: CommandAction::PrevBookmark,
            group: "Bookmarks",
        },
        Command {
            name: "Clear all bookmarks",
            shortcut: "",
            action: CommandAction::ClearBookmarks,
            group: "Bookmarks",
        },
        // ── Display ──
        Command {
            name: "Toggle word wrap",
            shortcut: "Alt+W",
            action: CommandAction::ToggleWrap,
            group: "Display",
        },
        Command {
            name: "Toggle syntax highlighting",
            shortcut: "s",
            action: CommandAction::ToggleSyntax,
            group: "Display",
        },
        Command {
            name: "Toggle tail mode",
            shortcut: "t",
            action: CommandAction::ToggleTail,
            group: "Display",
        },
        // ── Files ──
        Command {
            name: "Open file...",
            shortcut: "Ctrl+O",
            action: CommandAction::OpenFile,
            group: "Files",
        },
        Command {
            name: "Merge file...",
            shortcut: "M",
            action: CommandAction::MergeFile,
            group: "Files",
        },
        Command {
            name: "Export filtered results...",
            shortcut: "Ctrl+S",
            action: CommandAction::ExportFiltered,
            group: "Files",
        },
        // ── Appearance ──
        Command {
            name: "Change theme...",
            shortcut: "",
            action: CommandAction::ThemePicker,
            group: "Appearance",
        },
        // ── App ──
        Command {
            name: "Quit",
            shortcut: "q/Esc",
            action: CommandAction::Quit,
            group: "App",
        },
    ];
}
