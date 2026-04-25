use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateField {
    Due,
    Scheduled,
    Starting,
    Updated,
    CreationDate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextField {
    Project,
    Context,
    Description,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Due,
    Scheduled,
    Starting,
    Updated,
    CreationDate,
    Priority,
    Project,
    Context,
    Description,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareOp {
    Eq,
    Lt,
    Lte,
    Gt,
    Gte,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriorityOp {
    Eq,
    Above,
    Below,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TextOp {
    Includes,
    Excludes,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    Asc,
    Desc,
}

/// A date anchor in a smart-list date value.
///
/// `today` is resolved to the current date at evaluation time;
/// `Date(YYYY-MM-DD)` is a literal calendar anchor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateAnchor {
    Today,
    Date(String),
}

/// A resolved date value: anchor plus signed integer offset in days.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateValue {
    pub anchor: DateAnchor,
    pub offset: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Condition {
    DateComparison {
        field: DateField,
        op: CompareOp,
        value: DateValue,
    },
    PriorityComparison {
        op: PriorityOp,
        letter: char,
    },
    TextMatch {
        field: TextField,
        op: TextOp,
        text: String,
    },
    Existence {
        field: Field,
        present: bool,
    },
    DoneFilter {
        done: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterBlock {
    pub conditions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Directive {
    pub field: Field,
    pub direction: Direction,
}

/// Aggregated prefill declarations from a smart list.
///
/// Project / context fields accumulate in declaration order; scalar fields
/// keep the first valid declaration (`first wins`). All values are already
/// validated against the spec grammar — invalid lines are dropped during
/// parsing and never reach this struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Prefill {
    pub projects: Vec<String>,
    pub contexts: Vec<String>,
    pub priority: Option<char>,
    pub due: Option<DateValue>,
    pub scheduled: Option<DateValue>,
    pub starting: Option<DateValue>,
}

impl Prefill {
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty()
            && self.contexts.is_empty()
            && self.priority.is_none()
            && self.due.is_none()
            && self.scheduled.is_none()
            && self.starting.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmartList {
    pub name: String,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub group_path: Vec<String>,
    pub source_path: PathBuf,
    pub parse_error: Option<String>,
    pub blocks: Vec<FilterBlock>,
    pub sort_directives: Vec<Directive>,
    pub group_directives: Vec<Directive>,
    pub prefill: Prefill,
}
